//! Content-addressed normalized pixels; compiler state never crosses threads.
use crate::{Diagnostic, Loc, Result};
use base64::Engine;
use image::{DynamicImage, ImageFormat};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    io::Cursor,
    sync::{Arc, LazyLock, Mutex},
};
thread_local! { static PREVIEW: Cell<bool> = const { Cell::new(false) }; static ACTIVE: RefCell<HashMap<String, Arc<DynamicImage>>> = RefCell::new(HashMap::new()); static ACTIVE_INPUTS: RefCell<HashMap<String,Value>> = RefCell::new(HashMap::new()); }
#[cfg(test)]
thread_local! { static INPUT_HASHES: Cell<usize> = const { Cell::new(0) }; }
pub fn preview_mode(value: bool) {
    ACTIVE.with(|v| v.borrow_mut().clear());
    ACTIVE_INPUTS.with(|v| v.borrow_mut().clear());
    PREVIEW.set(value);
    if !value {
        prune();
    }
}
pub fn is_preview() -> bool {
    PREVIEW.get()
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
struct Entry {
    pixels: Arc<DynamicImage>,
    png: Option<Arc<Vec<u8>>>,
    jpeg: Option<Arc<Vec<u8>>>,
    tick: u64,
}
struct SourceEntry {
    bytes: Vec<u8>,
    key: String,
    tick: u64,
}
struct Cache {
    entries: HashMap<String, Entry>,
    inputs: HashMap<String, Value>,
    sources: HashMap<String, SourceEntry>,
    tick: u64,
    owners: HashMap<usize, (std::sync::Weak<Value>, Vec<String>)>,
}
static CACHE: LazyLock<Mutex<Cache>> = LazyLock::new(|| {
    Mutex::new(Cache {
        entries: HashMap::new(),
        inputs: HashMap::new(),
        sources: HashMap::new(),
        tick: 0,
        owners: HashMap::new(),
    })
});
fn trim(c: &mut Cache, keep: &str) {
    c.owners.retain(|_, (owner, _)| owner.strong_count() > 0);
    let live: std::collections::HashSet<_> = c
        .owners
        .values()
        .flat_map(|(_, keys)| keys.iter().cloned())
        .collect();
    c.entries.retain(|key, value| {
        key == keep || live.contains(key) || Arc::strong_count(&value.pixels) > 1
    });
    c.inputs.retain(|_, value| {
        value["rasterKey"]
            .as_str()
            .is_some_and(|key| c.entries.contains_key(key))
    });
    c.sources
        .retain(|_, value| c.inputs.contains_key(&value.key));
}
pub fn prune() {
    trim(&mut CACHE.lock().unwrap(), "");
}
pub fn finish_compilation() {
    if !is_preview() {
        ACTIVE.with(|v| v.borrow_mut().clear());
    }
    prune();
}
pub fn retain_nodes(nodes: &[Arc<Value>]) {
    fn keys(node: &Value, out: &mut Vec<String>) {
        match node {
            Value::Object(m) => {
                if let Some(key) = m.get("rasterKey").and_then(Value::as_str) {
                    out.push(key.into());
                }
                for value in m.values() {
                    if value.is_object() || value.is_array() {
                        keys(value, out);
                    }
                }
            }
            Value::Array(a) => {
                for value in a {
                    keys(value, out);
                }
            }
            _ => {}
        }
    }
    let mut cache = CACHE.lock().unwrap();
    for node in nodes {
        let id = Arc::as_ptr(node) as usize;
        if cache
            .owners
            .get(&id)
            .is_some_and(|(old, _)| old.upgrade().is_some_and(|old| Arc::ptr_eq(&old, node)))
        {
            continue;
        }
        let mut refs = vec![];
        keys(node, &mut refs);
        if !refs.is_empty() {
            cache.owners.insert(id, (Arc::downgrade(node), refs));
        }
    }
}
/// Kept for hosts built against the previous API; no allocation quota is applied.
pub fn set_budget(_mib: usize) {}
pub fn insert(image: DynamicImage) -> String {
    let mut hash = Sha256::new();
    hash.update(image.width().to_le_bytes());
    hash.update(image.height().to_le_bytes());
    hash.update([image.color() as u8]);
    hash.update(image.as_bytes());
    let key = format!("{:x}", hash.finalize());
    let mut c = CACHE.lock().unwrap();
    c.tick += 1;
    let tick = c.tick;
    c.entries
        .entry(key.clone())
        .or_insert(Entry {
            pixels: Arc::new(image),
            png: None,
            jpeg: None,
            tick,
        })
        .tick = tick;
    // Pin new pixels until the current compilation has registered its scene.
    // Other compiler threads may collect their own obsolete scenes meanwhile.
    ACTIVE.with(|v| {
        v.borrow_mut()
            .insert(key.clone(), c.entries[&key].pixels.clone());
    });
    key
}
pub fn pixels(key: &str) -> Option<Arc<DynamicImage>> {
    let mut c = CACHE.lock().unwrap();
    c.tick += 1;
    let tick = c.tick;
    c.entries
        .get_mut(key)
        .map(|e| {
            e.tick = tick;
            e.pixels.clone()
        })
        .or_else(|| ACTIVE.with(|v| v.borrow().get(key).cloned()))
}
pub fn source_jpeg(key: &str) -> Option<Arc<Vec<u8>>> {
    CACHE.lock().unwrap().entries.get(key)?.jpeg.clone()
}
pub fn register_jpeg(key: &str, bytes: &[u8]) {
    let mut c = CACHE.lock().unwrap();
    if let Some(e) = c.entries.get_mut(key) {
        e.jpeg = Some(Arc::new(bytes.to_vec()));
    }
}
pub fn png(key: &str) -> Result<Arc<Vec<u8>>> {
    if let Some(data) = CACHE
        .lock()
        .unwrap()
        .entries
        .get(key)
        .and_then(|e| e.png.clone())
    {
        return Ok(data);
    }
    let image = pixels(key).ok_or_else(|| {
        Diagnostic::new("E_ASSET", "Image cache entry expired", "", Loc::default())
    })?;
    let mut out = Cursor::new(Vec::new());
    image
        .write_to(&mut out, ImageFormat::Png)
        .map_err(|e| Diagnostic::new("E_ASSET", e.to_string(), "", Loc::default()))?;
    let data = Arc::new(out.into_inner());
    let mut c = CACHE.lock().unwrap();
    if let Some(e) = c.entries.get_mut(key) {
        e.png = Some(data.clone());
    }
    Ok(data)
}
pub fn asset(key: &str) -> Result<Value> {
    let image = pixels(key).ok_or_else(|| {
        Diagnostic::new("E_ASSET", "Image cache entry expired", "", Loc::default())
    })?;
    let data = if is_preview() {
        ACTIVE.with(|v| {
            v.borrow_mut().insert(key.into(), image.clone());
        });
        String::new()
    } else {
        base64::engine::general_purpose::STANDARD.encode(png(key)?.as_ref())
    };
    Ok(
        json!({"rasterKey": key, "mime":"image/png", "data":data,"width":image.width(),"height":image.height()}),
    )
}
pub fn input_key(bytes: &[u8], path: &str) -> String {
    let preview = is_preview();
    if preview {
        let mut c = CACHE.lock().unwrap();
        c.tick += 1;
        let tick = c.tick;
        if let Some(source) = c.sources.get_mut(path) {
            // Exact byte comparison also catches equal-size replacements with
            // preserved timestamps. Never treat filesystem metadata as proof.
            if source.bytes == bytes {
                source.tick = tick;
                return source.key.clone();
            }
        }
        c.sources.remove(path);
    }
    #[cfg(test)]
    INPUT_HASHES.set(INPUT_HASHES.get() + 1);
    let mut h = Sha256::new();
    h.update(path.rsplit('.').next().unwrap_or("").to_ascii_lowercase());
    h.update(bytes);
    let key = format!("{:x}", h.finalize());
    if preview {
        let mut c = CACHE.lock().unwrap();
        {
            c.tick += 1;
            let tick = c.tick;
            c.sources.insert(
                path.into(),
                SourceEntry {
                    bytes: bytes.to_vec(),
                    key: key.clone(),
                    tick,
                },
            );
        }
    }
    key
}
pub fn get_input(key: &str) -> Option<Value> {
    let value = CACHE
        .lock()
        .unwrap()
        .inputs
        .get(key)
        .cloned()
        .or_else(|| ACTIVE_INPUTS.with(|v| v.borrow().get(key).cloned()))?;
    let mut result = asset(value["rasterKey"].as_str()?).ok()?;
    if value["sourceJpegHash"].is_string() {
        result["sourceJpegHash"] = value["sourceJpegHash"].clone();
    }
    Some(result)
}
pub fn put_input(key: String, asset: &Value) {
    if asset["rasterKey"].is_string() {
        let mut v = asset.clone();
        v["data"] = json!("");
        if is_preview() {
            ACTIVE_INPUTS.with(|entries| {
                entries.borrow_mut().insert(key.clone(), v.clone());
            });
        }
        CACHE.lock().unwrap().inputs.insert(key, v);
    }
}

pub fn matching_jpeg(key: &str, expected: &str) -> Option<Arc<Vec<u8>>> {
    let bytes = source_jpeg(key)?;
    (digest(bytes.as_ref()) == expected).then_some(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unchanged_input_reuses_hash_across_frames_but_checks_every_byte() {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                preview_mode(false);
            }
        }
        let _reset = Reset;
        let path = "/hash-regression/source.png";
        CACHE.lock().unwrap().sources.remove(path);
        INPUT_HASHES.set(0);
        preview_mode(true);
        let first = input_key(b"abcd", path);
        assert_eq!(first, digest(b"pngabcd"));
        assert_eq!(INPUT_HASHES.get(), 1);
        let pixels = insert(DynamicImage::new_rgb8(2, 2));
        let owner = Arc::new(json!({"rasterKey":pixels}));
        put_input(first.clone(), &owner);
        retain_nodes(&[owner.clone()]);
        preview_mode(false);
        preview_mode(true);
        assert_eq!(input_key(b"abcd", path), first);
        assert_eq!(INPUT_HASHES.get(), 1);
        let changed = input_key(b"abce", path);
        assert_ne!(changed, first);
        assert_eq!(changed, digest(b"pngabce"));
        assert_eq!(INPUT_HASHES.get(), 2);
        assert_eq!(input_key(b"abce", path), changed);
        assert_eq!(INPUT_HASHES.get(), 2);
        preview_mode(false);
        assert_eq!(input_key(b"abce", path), changed);
        assert_eq!(INPUT_HASHES.get(), 3);
    }

    #[test]
    fn live_resources_are_retained_without_count_or_byte_quotas() {
        let mut c = Cache {
            entries: HashMap::new(),
            inputs: HashMap::new(),
            sources: HashMap::new(),
            tick: 0,
            owners: HashMap::new(),
        };
        let mut owners = vec![];
        for i in 0..200 {
            let key = i.to_string();
            let node = Arc::new(json!({"rasterKey":key}));
            c.owners.insert(
                Arc::as_ptr(&node) as usize,
                (Arc::downgrade(&node), vec![key.clone()]),
            );
            owners.push(node);
            c.sources.insert(
                key.clone(),
                SourceEntry {
                    bytes: vec![0; 8],
                    key: key.clone(),
                    tick: i,
                },
            );
            c.inputs.insert(key.clone(), json!({"rasterKey":key}));
            c.entries.insert(
                key,
                Entry {
                    pixels: Arc::new(DynamicImage::new_rgb8(32, 32)),
                    png: None,
                    jpeg: None,
                    tick: i,
                },
            );
        }
        trim(&mut c, "");
        assert_eq!(c.sources.len(), 200);
        assert_eq!(c.entries.len(), 200);
        owners.clear();
        trim(&mut c, "");
        assert!(c.sources.is_empty());
        assert!(c.entries.is_empty());
    }
}
