//! Media declarations and deterministic export cues. Decoding stays in hosts.
use crate::{Diagnostic, Loc, Result, engine::Engine, model::*};
use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::BTreeMap};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    #[serde(default)]
    pub assets: BTreeMap<String, Asset>,
    #[serde(default)]
    pub cues: Vec<Cue>,
}
impl Media {
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty() && self.cues.is_empty()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub src: String,
    pub mime: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub has_audio: bool,
    #[serde(default, skip_serializing_if = "empty_bytes")]
    pub data: std::sync::Arc<Vec<u8>>,
    #[serde(default, skip_serializing_if = "empty_bytes")]
    pub poster: std::sync::Arc<Vec<u8>>,
    #[serde(default, skip_serializing_if = "empty_bytes")]
    pub preview_audio: std::sync::Arc<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_resource: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub asset: String,
    #[serde(default = "page_scope")]
    pub scope: String,
    pub target: Option<String>,
    pub step: Option<usize>,
    pub trigger: String,
    pub delay: f64,
    pub trim_start: f64,
    pub trim_end: f64,
    pub looped: bool,
    pub volume: f64,
    pub muted: bool,
    pub export_start: Option<f64>,
    pub file: String,
    pub loc: Loc,
}
fn empty_bytes(b: &std::sync::Arc<Vec<u8>>) -> bool {
    b.is_empty()
}
fn page_scope() -> String {
    "page".into()
}
impl Cue {
    pub fn start(
        &self,
        animation: Option<&crate::animation::Animation>,
        reduced: bool,
        print: bool,
    ) -> Option<f64> {
        if self.trigger == "click" {
            return self.export_start;
        }
        Some(
            self.step
                .and_then(|i| {
                    animation?
                        .timeline_with_media(reduced, print)
                        .steps
                        .get(i)
                        .map(|s| s.start)
                })
                .unwrap_or(0.)
                + self.delay,
        )
    }
    pub fn sample(&self, time: f64, start: f64) -> Option<f64> {
        if time < start {
            return None;
        }
        let span = self.trim_end - self.trim_start;
        Some(
            self.trim_start
                + if self.looped {
                    (time - start).rem_euclid(span)
                } else {
                    (time - start).min((span - 1e-8).max(0.))
                },
        )
    }
}
fn err(message: impl Into<String>, file: &str, loc: Loc) -> Diagnostic {
    Diagnostic::new("E_MEDIA", message, file, loc)
}
fn time(v: &V, file: &str, loc: Loc) -> Result<f64> {
    match v.value() {
        V::Number(n, u) if n.is_finite() && *n >= 0. && matches!(u.as_str(), "s" | "ms") => {
            Ok(n * if u == "ms" { 0.001 } else { 1. })
        }
        _ => Err(err(
            "Media time requires a nonnegative finite s/ms value",
            file,
            loc,
        )),
    }
}
thread_local! { static CACHE: RefCell<BTreeMap<String,Asset>>=RefCell::new(BTreeMap::new()); }

fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}
fn be64(b: &[u8], i: usize) -> Option<u64> {
    Some(u64::from_be_bytes(b.get(i..i + 8)?.try_into().ok()?))
}
fn boxes(b: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut out = vec![];
    let mut i = 0;
    while i + 8 <= b.len() {
        let n = be32(b, i).unwrap() as usize;
        let (n, h) = if n == 1 {
            let Some(n) = be64(b, i + 8).and_then(|n| usize::try_from(n).ok()) else {
                break;
            };
            (n, 16)
        } else if n == 0 {
            (b.len() - i, 8)
        } else {
            (n, 8)
        };
        if n < h || n > b.len() - i {
            break;
        }
        out.push((&b[i + 4..i + 8], &b[i + h..i + n]));
        i += n;
    }
    out
}
fn mp4(b: &[u8]) -> Option<(f64, u32, u32, bool)> {
    let moov = boxes(b).into_iter().find(|(k, _)| *k == b"moov")?.1;
    let mut duration = None;
    let (mut w, mut h, mut audio) = (0, 0, false);
    for (k, b) in boxes(moov) {
        if k == b"mvhd" {
            let version = *b.first()?;
            let (scale, units) = if version == 1 {
                (be32(b, 20)?, be64(b, 24)? as f64)
            } else {
                (be32(b, 12)?, be32(b, 16)? as f64)
            };
            if scale > 0 {
                duration = Some(units / scale as f64);
            }
        }
        if k == b"trak" {
            let parts = boxes(b);
            let mdia = parts.iter().find(|(k, _)| *k == b"mdia")?.1;
            let handler = boxes(mdia)
                .into_iter()
                .find(|(k, _)| *k == b"hdlr")?
                .1
                .get(8..12)?;
            if handler == b"soun" {
                audio = true;
            }
            if handler == b"vide" {
                let tkhd = parts.iter().find(|(k, _)| *k == b"tkhd")?.1;
                if tkhd.len() >= 8 {
                    w = be32(tkhd, tkhd.len() - 8)? >> 16;
                    h = be32(tkhd, tkhd.len() - 4)? >> 16;
                    if tkhd.len() >= 44 {
                        let m = tkhd.len() - 44;
                        if be32(tkhd, m) == Some(0)
                            && be32(tkhd, m + 16) == Some(0)
                            && be32(tkhd, m + 4).is_some_and(|v| v != 0)
                            && be32(tkhd, m + 12).is_some_and(|v| v != 0)
                        {
                            std::mem::swap(&mut w, &mut h);
                        }
                    }
                }
            }
        }
    }
    Some((duration?, w, h, audio))
}
fn wav(b: &[u8]) -> Option<f64> {
    if b.get(..4)? != b"RIFF" || b.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut i = 12;
    let (mut rate, mut size) = (0u32, 0usize);
    while i + 8 <= b.len() {
        let len = u32::from_le_bytes(b[i + 4..i + 8].try_into().ok()?) as usize;
        let d = b.get(i + 8..i + 8 + len)?;
        if &b[i..i + 4] == b"fmt " {
            rate = u32::from_le_bytes(d.get(8..12)?.try_into().ok()?);
        }
        if &b[i..i + 4] == b"data" {
            size += len;
        }
        i += 8 + len + (len % 2);
    }
    if rate == 0 {
        None
    } else {
        Some(size as f64 / rate as f64)
    }
}
fn mp3(b: &[u8]) -> Option<f64> {
    let mut i = if b.starts_with(b"ID3") {
        let t = b.get(6..10)?;
        10 + t.iter().fold(0usize, |n, v| (n << 7) + (v & 127) as usize)
            + if b.get(5)? & 16 != 0 { 10 } else { 0 }
    } else {
        0
    };
    let mut seconds = 0.;
    while i + 4 <= b.len() {
        let v = be32(b, i)?;
        if v >> 21 != 0x7ff {
            i += 1;
            continue;
        }
        let version = (v >> 19) & 3;
        let layer = (v >> 17) & 3;
        let idx = ((v >> 12) & 15) as usize;
        let sr = ((v >> 10) & 3) as usize;
        if version == 1 || layer != 1 || idx == 0 || idx == 15 || sr == 3 {
            i += 1;
            continue;
        }
        let bitrate = if version == 3 {
            [
                0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 0,
            ][idx]
        } else {
            [
                0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160, 0,
            ][idx]
        };
        let rate = [44100, 48000, 32000][sr]
            / if version == 3 {
                1
            } else if version == 2 {
                2
            } else {
                4
            };
        let samples = if version == 3 { 1152 } else { 576 };
        let len =
            if version == 3 { 144 } else { 72 } * bitrate * 1000 / rate + ((v >> 9) & 1) as usize;
        if len < 4 || i + len > b.len() {
            break;
        }
        seconds += samples as f64 / rate as f64;
        i += len;
    }
    if seconds > 0. { Some(seconds) } else { None }
}
#[cfg(not(target_arch = "wasm32"))]
fn decode(executable: &str, path: &str, args: &[&str], file: &str, loc: Loc) -> Result<Vec<u8>> {
    let out = std::process::Command::new(executable)
        .args(["-hide_banner", "-loglevel", "error", "-i", path])
        .args(args)
        .output()
        .map_err(|e| {
            err(
                format!("Media preparation requires FFmpeg or an explicit poster: {e}"),
                file,
                loc,
            )
        })?;
    if !out.status.success() {
        return Err(err(
            format!(
                "Media decoding failed: {}",
                String::from_utf8_lossy(&out.stderr)
            ),
            file,
            loc,
        ));
    }
    Ok(out.stdout)
}
impl Engine {
    pub(crate) fn prepare_media_posters(
        &self,
        nodes: &mut [std::sync::Arc<serde_json::Value>],
    ) -> Result<()> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.host.native {
            fn replace(
                nodes: &mut [serde_json::Value],
                target: &str,
                bytes: &[u8],
                file: &str,
                loc: Loc,
            ) -> Result<()> {
                for n in nodes {
                    if n["id"] == target && n["mediaPosterExplicit"] != true {
                        let mut a = crate::assets::load(bytes, "poster.png", file, loc)?;
                        if let (Some(offset), Some(size)) = (
                            n["mediaCrop"]["offset"].as_array().cloned(),
                            n["mediaCrop"]["size"].as_array().cloned(),
                        ) {
                            let w = a["width"].as_f64().unwrap_or(1.);
                            let h = a["height"].as_f64().unwrap_or(1.);
                            n["sourceImage"] = a.clone();
                            n["sourceImage"]["crop"] = serde_json::json!({"x":(offset[0].as_f64().unwrap()*w).round(),"y":(offset[1].as_f64().unwrap()*h).round(),"width":(size[0].as_f64().unwrap()*w).round(),"height":(size[1].as_f64().unwrap()*h).round()});
                            a = crate::assets::crop_asset(
                                &a,
                                [
                                    offset[0].as_f64().unwrap() * w,
                                    offset[1].as_f64().unwrap() * h,
                                    size[0].as_f64().unwrap() * w,
                                    size[1].as_f64().unwrap() * h,
                                ],
                                file,
                                loc,
                            )?;
                        }
                        for key in ["data", "mime", "rasterKey", "sourceJpegHash"] {
                            n[key] = a[key].clone();
                        }
                    }
                    if let Some(children) = n["children"].as_array_mut() {
                        replace(children, target, bytes, file, loc)?;
                    }
                }
                Ok(())
            }
            for cue in &self.media.cues {
                if cue.trim_start <= 0. {
                    continue;
                }
                let Some(target) = &cue.target else { continue };
                let a = &self.media.assets[&cue.asset];
                let exe = if let Some(exe) = &self.host.media_ffmpeg {
                    exe.clone()
                } else if let Some(resolve) = &self.host.media_ffmpeg_resolver {
                    resolve()?
                } else {
                    "ffmpeg".into()
                };
                let output = std::process::Command::new(exe)
                    .args([
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-ss",
                        &cue.trim_start.to_string(),
                        "-i",
                        &a.src,
                        "-map",
                        "0:v:0",
                        "-frames:v",
                        "1",
                        "-f",
                        "image2pipe",
                        "-c:v",
                        "png",
                        "pipe:1",
                    ])
                    .output()
                    .map_err(|e| err(e.to_string(), &cue.file, cue.loc))?;
                if !output.status.success() {
                    return Err(err(
                        "Could not prepare the trimmed video poster",
                        &cue.file,
                        cue.loc,
                    ));
                }
                fn contains(n: &serde_json::Value, target: &str) -> bool {
                    n["id"] == target
                        || n["children"]
                            .as_array()
                            .is_some_and(|a| a.iter().any(|n| contains(n, target)))
                }
                for node in nodes.iter_mut().filter(|n| contains(n, target)) {
                    replace(
                        std::slice::from_mut(std::sync::Arc::make_mut(node)),
                        target,
                        &output.stdout,
                        &cue.file,
                        cue.loc,
                    )?;
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = nodes;
        Ok(())
    }
    pub(crate) fn load_media(
        &mut self,
        path: &str,
        video: bool,
        poster: Option<&str>,
        file: &str,
        loc: Loc,
    ) -> Result<String> {
        crate::session::queries::effect();
        let data = self.host.read(path, file, loc)?;
        let id = crate::asset_cache::digest(&data);
        if let Some(asset) = self.media.assets.get(&id) {
            // Explicit posters belong to instances. If the material was first
            // declared with one, prepare its default poster only when needed.
            if !(video && poster.is_none() && self.host.native && asset.poster.is_empty()) {
                return Ok(id);
            }
        }
        let metadata = mp4(&data)
            .or_else(|| wav(&data).map(|d| (d, 0, 0, true)))
            .or_else(|| mp3(&data).map(|d| (d, 0, 0, true)))
            .ok_or_else(|| {
                err(
                    "Unsupported or corrupt media. Supported formats are MP4/M4A, MP3 and WAV.",
                    file,
                    loc,
                )
            })?;
        let (duration, width, height, has_audio) = metadata;
        if duration <= 0. || !duration.is_finite() || video && (width == 0 || height == 0) {
            return Err(err(
                "Media requires a finite positive duration and video dimensions",
                file,
                loc,
            ));
        }
        let mime = if data.get(4..8) == Some(b"ftyp") {
            if video { "video/mp4" } else { "audio/mp4" }
        } else if data.starts_with(b"RIFF") {
            "audio/wav"
        } else {
            "audio/mpeg"
        };
        let executable = if self.host.native
            && (video || mime == "audio/mp4")
            && (poster.is_none() || has_audio)
        {
            if let Some(exe) = &self.host.media_ffmpeg {
                exe.clone()
            } else if let Some(resolve) = &self.host.media_ffmpeg_resolver {
                resolve().map_err(|e| Diagnostic::new(&e.code, e.message, file, loc))?
            } else {
                "ffmpeg".into()
            }
        } else {
            "ffmpeg".into()
        };
        let cachekey = format!("{id}/{poster:?}/{executable}");
        let cached = if self.host.native {
            CACHE.with(|c| c.borrow().get(&cachekey).cloned())
        } else {
            None
        };
        let asset = if let Some(mut a) = cached {
            a.src = path.into();
            a
        } else {
            #[allow(unused_mut)]
            let mut a = Asset {
                src: path.into(),
                mime: mime.into(),
                duration,
                width,
                height,
                has_audio,
                data: data.into(),
                poster: Default::default(),
                preview_audio: Default::default(),
                audio_resource: None,
            };
            #[cfg(not(target_arch = "wasm32"))]
            if (video || mime == "audio/mp4") && self.host.native {
                let exe = executable.as_str();
                if video && poster.is_none() {
                    a.poster = decode(
                        exe,
                        path,
                        &[
                            "-map",
                            "0:v:0",
                            "-frames:v",
                            "1",
                            "-f",
                            "image2pipe",
                            "-c:v",
                            "png",
                            "pipe:1",
                        ],
                        file,
                        loc,
                    )?
                    .into();
                }
                if has_audio {
                    a.preview_audio = decode(
                        exe,
                        path,
                        &[
                            "-map",
                            "0:a:0",
                            "-vn",
                            "-c:a",
                            "libmp3lame",
                            "-q:a",
                            "2",
                            "-f",
                            "mp3",
                            "pipe:1",
                        ],
                        file,
                        loc,
                    )?
                    .into();
                }
            }
            if self.host.native {
                CACHE.with(|c| {
                    let mut c = c.borrow_mut();
                    let size = a.data.len() + a.poster.len() + a.preview_audio.len();
                    if size <= 128 * 1024 * 1024 {
                        if size
                            + c.values()
                                .map(|a| a.data.len() + a.poster.len() + a.preview_audio.len())
                                .sum::<usize>()
                            > 128 * 1024 * 1024
                        {
                            c.clear();
                        }
                        c.insert(cachekey, a.clone());
                    }
                });
            }
            a
        };
        if let Some(p) = poster {
            self.host.read(p, file, loc)?;
        }
        self.media.assets.insert(id.clone(), asset);
        Ok(id)
    }
    pub(crate) fn play_media(
        &mut self,
        owner: std::rc::Rc<RefCell<Object>>,
        mut pos: Vec<V>,
        mut args: Args,
        loc: Loc,
    ) -> Result<V> {
        if pos.is_empty() {
            if let Some(target) = args.remove("target") {
                pos.push(target);
            }
        }
        let kind = owner.borrow().kind.clone();
        if !["canvas", "step", "document"].contains(&kind.as_str()) || pos.len() != 1 {
            return Err(err(
                "play requires a canvas, step or document and one video instance/audio material",
                &self.file,
                loc,
            ));
        }
        let target = pos[0]
            .object()
            .ok_or_else(|| err("play requires a media object", &self.file, loc))?;
        let o = target.borrow();
        let (asset, name) = if o.kind == "audio" {
            (string(&o.args, "__media_id", ""), None)
        } else if let Some(n) = &o.node {
            (
                jstr(n, "mediaId", "").into(),
                Some(jstr(n, "id", "").into()),
            )
        } else {
            return Err(err(
                "Video must be placed with add before play",
                &self.file,
                loc,
            ));
        };
        let a = self.media.assets.get(&asset).ok_or_else(|| {
            err(
                "play target is not a placed video or audio material",
                &self.file,
                loc,
            )
        })?;
        if kind == "document" && name.is_some() {
            return Err(err(
                "Document background tracks require audio",
                &self.file,
                loc,
            ));
        }
        let allowed = [
            "trigger",
            "delay",
            "trim_start",
            "trim_end",
            "loop",
            "volume",
            "muted",
            "export_start",
        ];
        for k in args.keys().filter(|k| !k.starts_with("__")) {
            if !allowed.contains(&k.as_str()) {
                return Err(err(format!("Unknown play parameter: {k}"), &self.file, loc));
            }
        }
        let timer = |k: &str, d: f64| -> Result<f64> {
            args.get(k)
                .map(|v| time(v, &self.file, loc))
                .transpose()
                .map(|v| v.unwrap_or(d))
        };
        let trigger = string(&args, "trigger", "auto");
        if !["auto", "click"].contains(&trigger.as_str()) || trigger == "click" && name.is_none() {
            return Err(err(
                "trigger accepts auto or click; click requires a video instance",
                &self.file,
                loc,
            ));
        }
        for k in ["loop", "muted"] {
            if args
                .get(k)
                .is_some_and(|v| !matches!(v.value(), V::Bool(_)))
            {
                return Err(err(format!("{k} requires a boolean"), &self.file, loc));
            }
        }
        let volume = args
            .get("volume")
            .map(|v| self.scalar(v, loc))
            .transpose()?
            .unwrap_or(1.);
        if !(0.0..=1.).contains(&volume) {
            return Err(err("volume requires a value from 0 to 1", &self.file, loc));
        }
        let cue = Cue {
            asset,
            scope: if kind == "document" {
                "document"
            } else {
                "page"
            }
            .into(),
            target: name,
            step: if kind == "step" {
                Some(num(&owner.borrow().args, "__index", 0.) as usize)
            } else {
                None
            },
            trigger,
            delay: timer("delay", 0.)?,
            trim_start: timer("trim_start", 0.)?,
            trim_end: timer("trim_end", a.duration)?,
            looped: matches!(args.get("loop"), Some(V::Bool(true))),
            volume,
            muted: matches!(args.get("muted"), Some(V::Bool(true))),
            export_start: args
                .get("export_start")
                .map(|v| time(v, &self.file, loc))
                .transpose()?,
            file: self.file.clone(),
            loc,
        };
        if cue.trim_start >= cue.trim_end || cue.trim_end > a.duration + 0.001 {
            return Err(err(
                "Trim interval must be positive and within the source duration",
                &self.file,
                loc,
            ));
        }
        if cue.target.is_some() && self.media.cues.iter().any(|c| c.target == cue.target) {
            return Err(err(
                "Each video instance can have only one playback binding",
                &self.file,
                loc,
            ));
        }
        crate::session::queries::effect();
        self.media.cues.push(cue);
        Ok(V::Null)
    }
}
pub fn duration(
    media: &Media,
    animation: Option<&crate::animation::Animation>,
    reduced: bool,
    print: bool,
) -> Result<f64> {
    let mut end = animation
        .map(|a| a.timeline_with_media(reduced, print).duration)
        .unwrap_or(0.);
    for c in &media.cues {
        if c.scope == "document" || c.looped {
            continue;
        }
        if let Some(start) = c.start(animation, reduced, print) {
            end = end.max(start + c.trim_end - c.trim_start);
        }
    }
    for c in &media.cues {
        if c.scope == "page" && c.looped {
            if c.start(animation, reduced, print)
                .is_some_and(|start| start >= end)
            {
                return Err(err(
                    "Looping page media requires a finite step duration/hold",
                    &c.file,
                    c.loc,
                ));
            }
        }
    }
    Ok(end)
}
