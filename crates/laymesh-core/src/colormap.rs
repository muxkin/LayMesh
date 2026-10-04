//! Version-pinned Matplotlib LUTs. No Python or network dependency at runtime.
use crate::color::Color;
use serde::Deserialize;
use std::{collections::BTreeMap, sync::LazyLock};

pub const VERSION: &str = "3.11.2";
#[derive(Deserialize)]
struct RawPreset {
    name: String,
    category: String,
    colors: String,
    reverse_colors: String,
}
#[derive(Deserialize)]
struct RawRegistry {
    aliases: BTreeMap<String, String>,
    presets: Vec<RawPreset>,
}
struct Preset {
    name: String,
    category: String,
    colors: Vec<[u8; 3]>,
    reverse_colors: Vec<[u8; 3]>,
}
struct Registry {
    aliases: BTreeMap<String, String>,
    presets: Vec<Preset>,
}
static REGISTRY: LazyLock<Registry> = LazyLock::new(|| {
    let raw: RawRegistry = serde_json::from_str(include_str!("../cmaps.json")).unwrap();
    fn decode(s: &str) -> Vec<[u8; 3]> {
        (0..s.len())
            .step_by(6)
            .map(|i| {
                std::array::from_fn(|j| {
                    u8::from_str_radix(&s[i + j * 2..i + j * 2 + 2], 16).unwrap()
                })
            })
            .collect()
    }
    Registry {
        aliases: raw.aliases,
        presets: raw
            .presets
            .into_iter()
            .map(|p| Preset {
                name: p.name,
                category: p.category,
                colors: decode(&p.colors),
                reverse_colors: decode(&p.reverse_colors),
            })
            .collect(),
    }
});
/// Immutable palette handle. Copying or reversing it never mutates another value.
#[derive(Clone, Debug)]
pub struct Colormap {
    index: usize,
    reversed: bool,
}
impl Colormap {
    pub fn get(name: &str) -> Option<Self> {
        let (name, reversed) = name
            .strip_suffix("_r")
            .map(|n| (n, true))
            .unwrap_or((name, false));
        let name = REGISTRY
            .aliases
            .get(name)
            .map(String::as_str)
            .unwrap_or(name);
        REGISTRY
            .presets
            .iter()
            .position(|p| p.name == name)
            .map(|index| Self { index, reversed })
    }
    pub fn name(&self) -> String {
        format!(
            "{}{}",
            REGISTRY.presets[self.index].name,
            if self.reversed { "_r" } else { "" }
        )
    }
    pub fn category(&self) -> &str {
        &REGISTRY.presets[self.index].category
    }
    pub fn reversed(&self) -> Self {
        Self {
            index: self.index,
            reversed: !self.reversed,
        }
    }
    fn table(&self) -> &[[u8; 3]] {
        let p = &REGISTRY.presets[self.index];
        if self.reversed {
            &p.reverse_colors
        } else {
            &p.colors
        }
    }
    fn at(&self, i: usize) -> Color {
        Color::new("rgb", self.table()[i].map(f64::from), 1.).unwrap()
    }
    pub fn sample(&self, t: f64) -> std::result::Result<Color, &'static str> {
        if !t.is_finite() {
            return Err("配色位置必须有限 / colormap position must be finite");
        }
        let i = ((t.clamp(0., 1.) * self.table().len() as f64).floor() as usize)
            .min(self.table().len() - 1);
        Ok(self.at(i))
    }
    pub fn colors(&self, n: Option<usize>) -> std::result::Result<Vec<Color>, &'static str> {
        let n = n.unwrap_or(self.table().len());
        if n > 10_000 {
            return Err("配色数量超过 10,000 / palette exceeds 10,000 colors");
        }
        (0..n)
            .map(|i| {
                if self.category() == "qualitative" {
                    Ok(self.at(i % self.table().len()))
                } else {
                    self.sample(if self.category() == "cyclic" {
                        i as f64 / n as f64
                    } else if n == 1 {
                        0.5
                    } else {
                        i as f64 / (n - 1) as f64
                    })
                }
            })
            .collect()
    }
}
pub fn names(
    category: Option<&str>,
    reversed: bool,
) -> std::result::Result<Vec<String>, &'static str> {
    if category
        .is_some_and(|c| !["sequential", "diverging", "cyclic", "qualitative", "misc"].contains(&c))
    {
        return Err("未知配色类别 / unknown colormap category");
    }
    Ok(REGISTRY
        .presets
        .iter()
        .filter(|p| category.is_none_or(|c| p.category == c))
        .flat_map(|p| {
            let mut names = vec![p.name.clone()];
            if reversed {
                names.push(format!("{}_r", p.name));
            }
            names
        })
        .collect())
}
