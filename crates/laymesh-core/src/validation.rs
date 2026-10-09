use crate::{
    Loc, Result,
    engine::{API, Engine},
    model::*,
};
impl Engine {
    pub(crate) fn validate_definition(&mut self, name: &str, a: &Args, l: Loc) -> Result<()> {
        self.validate_art(name, a, l)?;
        crate::arrows::validate(self, name, a, l)?;
        if name == "line" && crate::endpoints::has_line_geometry(a) {
            crate::endpoints::line_vector(self, a, l)?;
        }
        if name == "head" {
            if let Some(v) = a.get("size") {
                let values = v.list();
                if values.len() != 2
                    || values
                        .iter()
                        .any(|v| self.len(v, l).map_or(true, |x| x <= 0.))
                {
                    return Err(self.error("E_STROKE", "头部 size 需要两个正长度", l));
                }
            }
        }
        for key in ["start_head", "end_head"] {
            if let Some(v) = a.get(key) {
                if crate::endpoints::head_args(v).is_none() {
                    return Err(self.error("E_STROKE", format!("{key} 需要 head(...) 配置"), l));
                }
            }
        }
        let plot = name.starts_with("plot.")
            || matches!(
                name,
                "plot" | "axis" | "plot_style" | "legend" | "colorbar" | "color_scale"
            );
        let api = API["api"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap();
        for requirement in api["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| v.as_str())
        {
            let alternatives: Vec<_> = requirement.split('/').map(str::trim).collect();
            if !alternatives.iter().any(|k| a.contains_key(*k)) {
                return Err(self.error("E_ARG", format!("缺少参数 {requirement}"), l));
            }
        }
        if let Some(font) = a.get("font_family") {
            let valid = match font {
                V::Text(s, _) => !s.trim().is_empty(),
                V::List(v) => {
                    !v.is_empty()
                        && v.iter()
                            .all(|v| matches!(v,V::Text(s,_) if !s.trim().is_empty()))
                }
                _ => false,
            };
            if !valid {
                return Err(self.error("E_FONT", "font_family 需要非空名称、路径或字符串列表", l));
            }
        }
        if a.get("math_font")
            .is_some_and(|v| !matches!(v,V::Text(s,_) if !s.trim().is_empty()))
        {
            return Err(self.error("E_MATH_FONT", "math_font 需要非空字体名称或路径", l));
        }
        if a.get("math_text_fallback")
            .is_some_and(|v| !matches!(v, V::Bool(_)))
        {
            return Err(self.error("E_FONT", "math_text_fallback 需要布尔值", l));
        }
        if let Some(V::Number(weight, u)) = a.get("font_weight") {
            if !u.is_empty() || weight.fract() != 0. || !(100.0..=900.0).contains(weight) {
                return Err(self.error("E_FONT", "font_weight 必须为 100–900 整数", l));
            }
        }
        for param in api["parameters"].as_array().unwrap() {
            let key = param["name"].as_str().unwrap();
            let Some(value) = a.get(key) else { continue };
            let typ = param["type"].as_str().unwrap_or("");
            if key == "size" && !matches!(value,V::List(v) if v.len()==2) {
                return Err(self.error("E_API_MIGRATION", "二维尺寸使用 size=(宽,高)", l));
            }
            if !plot
                && typ == "enum"
                && !param["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v.as_str() == Some(value.as_str().as_str()))
            {
                return Err(self.error("E_ARG", format!("无效 {key}"), l));
            }
            if typ == "color[]" {
                if !matches!(value,V::List(v) if !v.is_empty()) {
                    return Err(self.error("E_COLOR", "需要非空颜色列表", l));
                }
                self.validate_color(value, l, "E_COLOR")?;
            }
            if typ == "color" || (typ == "paint" && matches!(value, V::Text(..) | V::Color(_))) {
                self.validate_color(value, l, if plot { "E_PLOT" } else { "E_COLOR" })?;
            }
            if (param["unit"] == "geometry" || param["unit"] == "pt") && key != "plot_area" {
                fn lengths(e: &Engine, v: &V, l: Loc) -> Result<()> {
                    match v {
                        V::List(v) => {
                            for v in v {
                                lengths(e, v, l)?;
                            }
                            Ok(())
                        }
                        V::Text(s, _) if s == "auto" => Ok(()),
                        _ => e.len(v, l).map(|_| ()),
                    }
                }
                lengths(self, value, l)?;
            }
            if key == "size" {
                if !matches!(value,V::List(v) if v.len()==2) {
                    return Err(self.error("E_API_MIGRATION", "二维尺寸使用 size=(宽,高)", l));
                }
            }
            if key.ends_with("font_size") || key == "line_height" {
                if self.len(value, l)? <= 0. {
                    return Err(self.error(
                        if plot { "E_PLOT" } else { "E_VALUE" },
                        format!("{key} 必须为正"),
                        l,
                    ));
                }
            }
        }
        for prefix in ["line", "border"] {
            let code = if plot { "E_PLOT" } else { "E_STROKE" };
            if let Some(v) = a.get(&format!("{prefix}_width")) {
                if self.len(v, l)? < 0. {
                    return Err(self.error(code, "线宽不能为负", l));
                }
            }
            if let Some(v) = a.get(&format!("{prefix}_opacity")) {
                if !(0.0..=1.0).contains(&self.scalar(v, l)?) {
                    return Err(self.error(code, "描边透明度必须在 0–1 之间", l));
                }
            }
            if let Some(v) = a.get(&format!("{prefix}_dash")) {
                let vs = v.list();
                if vs.is_empty()
                    || vs.len() % 2 != 0
                    || vs
                        .iter()
                        .any(|v| value_length(v, &self.unit, self.dpi).is_none_or(|x| x <= 0.))
                {
                    return Err(self.error(code, "dash 需要成对正长度", l));
                }
            }
        }
        if let Some(v) = a.get("opacity") {
            if !matches!(v, V::List(..)) && !(0.0..=1.0).contains(&self.scalar(v, l)?) {
                return Err(self.error("E_VALUE", "透明度必须在 0–1 之间", l));
            }
        }
        if name == "path" {
            let commands = array(a, "commands");
            if commands
                .first()
                .and_then(V::object)
                .is_none_or(|v| v.borrow().kind != "move_to")
            {
                return Err(self.error("E_PATH", "路径必须以 move_to 开始", l));
            }
        }
        if name == "plot" && a.contains_key("plot_area") && a.contains_key("margins") {
            return Err(self.error("E_ARG", "plot_area 与 margins 不能同时指定", l));
        }
        if name == "image" {
            let src = string(a, "src", "");
            let path = resolve(&self.file, &src);
            self.load_image(&path, &self.file.clone(), l)?;
        }
        if name == "formula" {
            let mut spec = self.text_spec(a, &self.file.clone(), l)?;
            spec["kind"] = serde_json::json!("formula");
            crate::text::layout_text(
                &spec,
                None,
                &mut self.fonts,
                &mut self.warnings,
                &self.file,
                l,
            )?;
        }
        if matches!(name, "linear_gradient" | "radial_gradient") {
            let stops = array(a, "stops");
            let mut last = -1.;
            if stops.len() < 2 {
                return Err(self.error("E_PAINT", "渐变至少需要两个色标", l));
            }
            for s in stops {
                let s = s.list();
                if !(2..=3).contains(&s.len()) {
                    return Err(self.error("E_PAINT", "色标需要位置、颜色和可选透明度", l));
                }
                let x = self.scalar(&s[0], l)?;
                if !(0.0..=1.0).contains(&x) || x < last {
                    return Err(self.error("E_PAINT", "色标需要递增的 0–1 位置", l));
                }
                last = x;
                self.validate_color(&s[1], l, "E_COLOR")?;
            }
        }
        Ok(())
    }
    pub(crate) fn validate_color(&self, v: &V, l: Loc, code: &str) -> Result<()> {
        if let V::List(values) = v {
            for value in values {
                self.validate_color(value, l, code)?;
            }
            return Ok(());
        }
        crate::color::Color::parse(&v.as_str())
            .map(|_| ())
            .map_err(|m| self.error(code, m, l))
    }
}
