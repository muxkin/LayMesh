//! Layer lists retain CSS order (front to back). Layout uses the object box.
use crate::{
    animation::{
        css::split,
        values::{self, Context},
    },
    gradients::{self, Coordinate, Units},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
pub const PARTS: [&str; 5] = [
    "background_image",
    "background_color",
    "background_position",
    "background_size",
    "background_repeat",
];
pub fn initial(key: &str) -> &'static str {
    match key {
        "background_color" => "transparent",
        "background_position" => "0% 0%",
        "background_size" => "auto",
        "background_repeat" => "repeat",
        _ => "none",
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub paint: Json,
    pub position: [Coordinate; 2],
    pub size: Vec<String>,
    pub repeat: [String; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Background {
    pub layers: Vec<Layer>,
    pub color: Json,
    pub units: Units,
}
pub fn image(s: &str, ctx: Context, current: &str) -> Result<(), String> {
    if s == "none" {
        return Ok(());
    }
    if s.to_ascii_lowercase().contains("gradient(") {
        gradients::parse(s, ctx, current)?;
        return Ok(());
    }
    if (s.starts_with("url(") || s.starts_with("image(") || s.starts_with("hatch("))
        && s.ends_with(')')
    {
        return Ok(());
    }
    Err("Expected background image or gradient".into())
}
pub fn size(s: &str, ctx: Context) -> Result<Vec<String>, String> {
    let words = values::words(&s.to_ascii_lowercase());
    if words.len() == 1 && matches!(words[0].as_str(), "cover" | "contain") {
        return Ok(words);
    }
    if words.is_empty() || words.len() > 2 {
        return Err("Background size needs one or two dimensions".into());
    }
    for w in &words {
        if w != "auto"
            && values::length_context(
                w,
                Context {
                    reference: 1.,
                    ..ctx
                },
            )? < 0.
            && !["calc(", "min(", "max(", "clamp("]
                .iter()
                .any(|f| w.starts_with(f))
        {
            return Err("Background size cannot be negative".into());
        }
    }
    Ok(if words.len() == 1 {
        vec![words[0].clone(), "auto".into()]
    } else {
        words
    })
}
pub fn repeat(s: &str) -> Result<[String; 2], String> {
    let mut words = values::words(&s.to_ascii_lowercase());
    if words == ["repeat-x"] {
        return Ok(["repeat".into(), "no-repeat".into()]);
    }
    if words == ["repeat-y"] {
        return Ok(["no-repeat".into(), "repeat".into()]);
    }
    if words.len() == 1 {
        words.push(words[0].clone());
    }
    if words.len() != 2
        || words
            .iter()
            .any(|w| !matches!(w.as_str(), "repeat" | "no-repeat" | "space" | "round"))
    {
        return Err("Invalid background repeat".into());
    }
    Ok([words[0].clone(), words[1].clone()])
}
pub fn validate(key: &str, s: &str, ctx: Context, current: &str) -> Result<(), String> {
    if key == "background_color" {
        crate::css_values::color(s, current)?;
        return Ok(());
    }
    for v in split(s, ',') {
        match key {
            "background_image" => image(&v, ctx, current)?,
            "background_position" => {
                gradients::position(&values::words(&v), ctx)?;
            }
            "background_size" => {
                size(&v, ctx)?;
            }
            "background_repeat" => {
                repeat(&v)?;
            }
            "background_color" => {
                crate::css_values::color(&v, current)?;
            }
            _ => return Err("Unknown background property".into()),
        }
    }
    Ok(())
}
pub fn shorthand(s: &str, ctx: Context, current: &str) -> Result<Vec<(String, String)>, String> {
    let layers = split(s, ',');
    let mut images = vec![];
    let mut positions = vec![];
    let mut sizes = vec![];
    let mut repeats = vec![];
    let mut color = "transparent".to_string();
    let mut has_color = false;
    for (index, layer) in layers.iter().enumerate() {
        let halves = split(layer, '/');
        if halves.len() > 2 {
            return Err("Duplicate background size separator".into());
        }
        let mut source = None;
        let mut pos = vec![];
        let mut sz = vec![];
        let mut rep = vec![];
        for (half, text) in halves.iter().enumerate() {
            for word in values::words(text) {
                if image(&word, ctx, current).is_ok() {
                    if source.replace(word).is_some() {
                        return Err("Duplicate background image".into());
                    }
                } else if matches!(
                    word.as_str(),
                    "repeat" | "no-repeat" | "space" | "round" | "repeat-x" | "repeat-y"
                ) {
                    rep.push(word);
                } else if crate::css_values::color(&word, current).is_ok() {
                    if has_color || index + 1 != layers.len() {
                        return Err(
                            "Background color is only allowed once, in the final layer".into()
                        );
                    }
                    color = word;
                    has_color = true;
                } else if half == 0 {
                    pos.push(word);
                } else {
                    sz.push(word);
                }
            }
        }
        if layer.trim().is_empty() {
            return Err("Empty background layer".into());
        }
        if halves.len() == 2 && (pos.is_empty() || sz.is_empty()) {
            return Err("Background size requires position / size".into());
        }
        let source = source.unwrap_or_else(|| "none".into());
        image(&source, ctx, current)?;
        let pos = if pos.is_empty() {
            "0% 0%".into()
        } else {
            pos.join(" ")
        };
        gradients::position(&values::words(&pos), ctx)?;
        let sz = if sz.is_empty() {
            "auto".into()
        } else {
            sz.join(" ")
        };
        size(&sz, ctx)?;
        let rep = if rep.is_empty() {
            "repeat".into()
        } else {
            rep.join(" ")
        };
        repeat(&rep)?;
        images.push(source);
        positions.push(pos);
        sizes.push(sz);
        repeats.push(rep);
    }
    Ok(vec![
        ("background_image".into(), images.join(", ")),
        ("background_color".into(), color),
        ("background_position".into(), positions.join(", ")),
        ("background_size".into(), sizes.join(", ")),
        ("background_repeat".into(), repeats.join(", ")),
    ])
}
pub fn compose(
    images: Vec<Json>,
    color: Json,
    pos: &str,
    sz: &str,
    rep: &str,
    ctx: Context,
) -> Json {
    let positions = split(pos, ',');
    let sizes = split(sz, ',');
    let repeats = split(rep, ',');
    let layers = images
        .into_iter()
        .enumerate()
        .map(|(i, paint)| Layer {
            paint,
            position: gradients::position(&values::words(&positions[i % positions.len()]), ctx)
                .expect("validated position"),
            size: size(&sizes[i % sizes.len()], ctx).expect("validated size"),
            repeat: repeat(&repeats[i % repeats.len()]).expect("validated repeat"),
        })
        .collect();
    json!({"kind":"backgroundLayers","background":Background{layers,color,units:ctx.into()}})
}
#[derive(Clone, Debug)]
pub struct Tile {
    pub origin: [f64; 2],
    pub size: [f64; 2],
    pub period: [f64; 2],
    pub clip: [f64; 4],
}
impl Layer {
    pub fn layout(&self, units: &Units, b: [f64; 4]) -> Option<Tile> {
        let [x, y, w, h] = b;
        let dims = [w, h];
        let image = self.paint["kind"]
            .as_str()
            .is_some_and(|s| matches!(s, "imagePaint" | "image_fill" | "image_paint"));
        let intrinsic = if image {
            [
                self.paint["width"].as_f64().unwrap_or(w) * 25.4 / units.dpi,
                self.paint["height"].as_f64().unwrap_or(h) * 25.4 / units.dpi,
            ]
        } else {
            dims
        };
        let mut size = [0.; 2];
        let mut auto = [false; 2];
        if self.size.len() == 1 {
            let scale = if self.size[0] == "cover" {
                (w / intrinsic[0]).max(h / intrinsic[1])
            } else {
                (w / intrinsic[0]).min(h / intrinsic[1])
            };
            size = intrinsic.map(|n| n * scale);
        } else {
            for i in 0..2 {
                auto[i] = self.size[i] == "auto";
                size[i] = if auto[i] {
                    intrinsic[i]
                } else {
                    values::length_context(&self.size[i], units.context(dims[i]))
                        .expect("validated size")
                };
            }
            if image && auto[0] != auto[1] {
                let i = usize::from(auto[1]);
                size[i] = size[1 - i] * intrinsic[i] / intrinsic[1 - i];
            }
        }
        if size.iter().any(|s| !s.is_finite() || *s <= 0.) || w <= 0. || h <= 0. {
            return None;
        }
        for i in 0..2 {
            if self.repeat[i] == "round" {
                let old = size[i];
                size[i] = dims[i] / (dims[i] / size[i]).round().max(1.);
                if image && auto[1 - i] && self.repeat[1 - i] != "round" {
                    size[1 - i] *= size[i] / old;
                }
            }
        }
        let mut origin = [0.; 2];
        let mut period = size;
        let mut clip = [x, y, x + w, y + h];
        for i in 0..2 {
            let offset = self.position[i].resolve(units, dims[i] - size[i]);
            origin[i] = b[i] + offset;
            let count = (dims[i] / size[i] + 1e-9).floor();
            if self.repeat[i] == "space" && count >= 2. {
                origin[i] = b[i];
                period[i] = (dims[i] - size[i]) / (count - 1.);
            } else if self.repeat[i] == "no-repeat" || self.repeat[i] == "space" {
                clip[i] = clip[i].max(origin[i]);
                clip[i + 2] = clip[i + 2].min(origin[i] + size[i]);
            }
        }
        Some(Tile {
            origin,
            size,
            period,
            clip,
        })
    }
}
