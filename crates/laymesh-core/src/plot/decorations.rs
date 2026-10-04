use super::*;
pub fn exponent(a: &Args, domain: [f64; 2]) -> i32 {
    num(
        a,
        "exponent",
        domain[0]
            .abs()
            .max(domain[1].abs())
            .max(1e-323)
            .log10()
            .floor(),
    )
    .clamp(-323., 308.) as i32
}
pub fn format_number(value: f64, a: &Args) -> String {
    let fmt = string(a, "format", "");
    let precision = fmt
        .split('.')
        .nth(1)
        .and_then(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse::<usize>()
                .ok()
        })
        .unwrap_or(2)
        .min(100);
    let mut out = match fmt.chars().last() {
        Some('f') => format!("{value:.precision$}"),
        Some('%') => format!("{:.precision$}%", value * 100.),
        Some('e') => format!("{value:.precision$e}"),
        Some('d') => format!("{}", value.round() as i64),
        _ => {
            let value = if value.abs() < 1e-13 { 0. } else { value };
            if value != 0. && (value.abs() < 1e-4 || value.abs() >= 1e7) {
                format!("{value:0.2e}")
            } else {
                let s = format!("{value:0.8}");
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            }
        }
    };
    if fmt.contains(',') {
        let split = out.find(['.', 'e', '%']).unwrap_or(out.len());
        let prefix = usize::from(out.starts_with('-'));
        let digits = &out[prefix..split];
        let grouped = digits
            .chars()
            .enumerate()
            .fold(String::new(), |mut acc, (i, c)| {
                if i > 0 && (digits.len() - i) % 3 == 0 {
                    acc.push(',');
                }
                acc.push(c);
                acc
            });
        out.replace_range(prefix..split, &grouped);
    }
    if fmt.contains('$') {
        let prefix = usize::from(out.starts_with('-'));
        out.insert(prefix, '$');
    }
    if fmt.contains('+') && value >= 0. {
        out.insert(0, '+');
    }
    if out.contains('e') {
        let (coefficient, exp) = out.split_once('e').unwrap();
        if let Ok(exponent) = exp.parse::<i32>() {
            out = format!("{coefficient}e{exponent:+}");
        }
    }
    if fmt.contains('(') && value < 0. {
        out = format!("({})", out.trim_start_matches('-'));
    }
    if out.starts_with('-') {
        out.replace_range(..1, "−");
    }
    let format_head = fmt.split('.').next().unwrap_or(&fmt);
    let alignment = format_head
        .char_indices()
        .find(|(_, c)| matches!(c, '<' | '>' | '^' | '='));
    if let Some((i, align)) = alignment {
        let fill = if i > 0 {
            format_head[..i].chars().last().unwrap_or(' ')
        } else {
            ' '
        };
        let width = format_head[i + align.len_utf8()..]
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<usize>()
            .unwrap_or(0)
            .min(10000);
        let padding = width.saturating_sub(out.chars().count());
        match align {
            '<' => out.push_str(&fill.to_string().repeat(padding)),
            '^' => {
                out = format!(
                    "{}{}{}",
                    fill.to_string().repeat(padding / 2),
                    out,
                    fill.to_string().repeat(padding - padding / 2)
                )
            }
            _ => out = format!("{}{}", fill.to_string().repeat(padding), out),
        }
    }
    out
}
fn format_si(value: f64) -> String {
    if value == 0. {
        return "0".into();
    }
    let exponent = ((value.abs().log10() / 3.).floor() as i32 * 3).clamp(-24, 24);
    let prefixes = [
        "y", "z", "a", "f", "p", "n", "µ", "m", "", "k", "M", "G", "T", "P", "E", "Z", "Y",
    ];
    let v = value / 10f64.powi(exponent);
    let precision = (5. - v.abs().log10().floor()).max(0.) as usize;
    let mut text = format!("{v:.precision$}");
    if text.contains('.') {
        text = text.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    if text.starts_with('-') {
        text.replace_range(..1, "−");
    }
    format!("{}{}", text, prefixes[((exponent + 24) / 3) as usize])
}
pub fn number_label(
    e: &mut Engine,
    value: f64,
    a: &Args,
    domain: [f64; 2],
    style: &Args,
    size: f64,
    o: &Object,
) -> Result<Json> {
    if !a.contains_key("format")
        && string(a, "scale", "linear") == "log"
        && string(a, "notation", "plain") != "scientific"
    {
        let factor = if string(a, "notation", "plain") == "offset" {
            10f64.powi(exponent(a, domain))
        } else {
            1.
        };
        let d = [domain[0] / factor, domain[1] / factor];
        let v = value / factor;
        let count = num(a, "_tick_count", 5.) as usize;
        let limit =
            (10. * count as f64 / scales::scale_ticks(d, count, "log").len().max(1) as f64).max(1.);
        let mut coefficient = v / 10f64.powf(v.log10().round());
        if coefficient * 10. < 9.5 {
            coefficient *= 10.;
        }
        let content = if coefficient <= limit {
            format_si(v)
        } else {
            String::new()
        };
        return label(e, &V::text(content), style, size, o);
    }
    let mut formatted = a.clone();
    if !a.contains_key("format") && string(a, "notation", "plain") != "scientific" {
        let factor = if string(a, "notation", "plain") == "offset" {
            10f64.powi(exponent(a, domain))
        } else {
            1.
        };
        let ticks = scales::ticks(
            [domain[0] / factor, domain[1] / factor],
            num(a, "_tick_count", 5.) as usize,
        );
        let step = if ticks.len() > 1 {
            ticks[1] - ticks[0]
        } else {
            (domain[1] - domain[0]) / factor
        };
        let precision = (-step.abs().log10().floor()).max(0.).min(16.) as usize;
        formatted.insert("format".into(), V::text(format!(",.{precision}f")));
    }
    let a = &formatted;
    match string(a, "notation", "plain").as_str() {
        "scientific" if value != 0. => {
            let mut fmt = a.clone();
            fmt.entry("format".into()).or_insert_with(|| V::text(".2e"));
            let formatted = format_number(value, &fmt);
            let (prefix, exp) = formatted
                .split_once('e')
                .ok_or_else(|| e.error("E_PLOT", "无法将 format 转为科学计数法", o.loc))?;
            let precision = string(&fmt, "format", ".2e")
                .split('.')
                .nth(1)
                .and_then(|s| {
                    s.chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>()
                        .parse::<usize>()
                        .ok()
                })
                .unwrap_or(6)
                .min(100);
            let plain = format!("{value:.precision$e}");
            let exponent = plain.split_once('e').unwrap().1.parse::<i32>().unwrap();
            let len = format!("{exponent:+}").len();
            if exp.len() < len {
                return Err(e.error("E_PLOT", "无法将 format 转为科学计数法", o.loc));
            }
            let suffix = &exp[len..];
            let mut node = label(
                e,
                &V::text(format!("{prefix}$\\times 10^{{{exponent}}}${}", suffix)),
                style,
                size,
                o,
            )?;
            node["content"] = json!(format!("{prefix}×10^{exponent}{suffix}"));
            Ok(node)
        }
        "scientific" => label(e, &V::text("0"), style, size, o),
        "offset" => label(
            e,
            &V::text(format_number(value / 10f64.powi(exponent(a, domain)), a)),
            style,
            size,
            o,
        ),
        _ => label(e, &V::text(format_number(value, a)), style, size, o),
    }
}
fn legend_layout(
    e: &mut Engine,
    o: &Object,
    a: &Args,
    style: &Args,
    layers: &[Layer],
) -> Result<Json> {
    let fs = length_arg(
        e,
        a,
        "font_size",
        length_arg(e, style, "font_size", 8. * PT),
    );
    let gap = length_arg(e, a, "gap", 1.2);
    let padding = length_arg(e, a, "padding", gap);
    let sg = length_arg(e, a, "sample_gap", 3.);
    let requested = num(a, "columns", 1.) as usize;
    if requested == 0 || requested > 10000 {
        return Err(e.error("E_PLOT", "图例 columns 需要正整数", o.loc));
    }
    let mut label_style = style.clone();
    label_style
        .entry("color".into())
        .or_insert_with(|| V::text("#222222"));
    let mut entries = vec![];
    for layer in layers {
        if layer.kind == "heatmap" {
            continue;
        }
        if let Some(value) = layer.args.get("label") {
            if !value.as_str().is_empty() || value.object().is_some() {
                entries.push((
                    layer,
                    label_at(e, value, &label_style, fs, o, &layer.args, "label")?,
                ));
            }
        }
    }
    let marker_size = entries
        .iter()
        .filter(|(l, _)| {
            matches!(l.kind.as_str(), "line" | "scatter" | "errorbar")
                && string(
                    &l.args,
                    "marker",
                    if l.kind == "scatter" {
                        "circle"
                    } else {
                        "none"
                    },
                ) != "none"
        })
        .map(|(l, _)| length_arg(e, &l.args, "marker_size", 3. * PT))
        .fold(0., f64::max);
    let error_size = entries
        .iter()
        .filter(|(l, _)| l.kind == "errorbar")
        .map(|(l, _)| length_arg(e, &l.args, "cap_size", 3. * PT).max(3.))
        .fold(0., f64::max);
    let sample = length_arg(e, a, "sample_width", 6.)
        .max(marker_size)
        .max(error_size);
    let columns = requested.min(entries.len().max(1));
    let rows = (entries.len() + columns - 1) / columns;
    let row_height = entries
        .iter()
        .map(|(_, n)| jnum(n, "height", 0.))
        .fold(marker_size.max(error_size), f64::max);
    let mut widths = vec![0f64; columns];
    for (i, (_, n)) in entries.iter().enumerate() {
        widths[i % columns] = widths[i % columns].max(sample + sg + jnum(n, "width", 0.));
    }
    let title = a
        .get("title")
        .map(|v| label_at(e, v, &label_style, fs, o, a, "title"))
        .transpose()?;
    let th = title
        .as_ref()
        .map(|n| jnum(n, "height", 0.) + gap)
        .unwrap_or(0.);
    let w = (widths.iter().sum::<f64>() + gap * columns.saturating_sub(1) as f64)
        .max(title.as_ref().map(|n| jnum(n, "width", 0.)).unwrap_or(0.))
        + padding * 2.;
    let h = rows as f64 * row_height + gap * rows.saturating_sub(1) as f64 + padding * 2. + th;
    let frame = yes(a, "frame", false) || a.contains_key("border_color");
    let mut children = vec![path(
        &[[0., 0.], [w, 0.], [w, h], [0., h]],
        true,
        w,
        h,
        a.get("background").map(V::json).unwrap_or(json!("#ffffff")),
        &if frame {
            string(a, "border_color", &string(style, "color", "#222222"))
        } else {
            "none".into()
        },
        if frame {
            length_arg(
                e,
                a,
                "border_width",
                length_arg(e, style, "line_width", 0.6 * PT),
            )
        } else {
            0.
        },
    )];
    if let Some(mut n) = title {
        place(&mut n, padding, padding, [0., 0.]);
        children.push(n);
    }
    for (i, (layer, mut n)) in entries.into_iter().enumerate() {
        let col = i % columns;
        let row = i / columns;
        let x = padding + widths[..col].iter().sum::<f64>() + gap * col as f64;
        let y = padding + th + row as f64 * (row_height + gap) + row_height / 2.;
        let opacity = num(
            &layer.args,
            "opacity",
            if matches!(layer.kind.as_str(), "area" | "band") {
                0.2
            } else {
                1.
            },
        );
        let sw = length_arg(
            e,
            &layer.args,
            "line_width",
            length_arg(
                e,
                &layer.args,
                "border_width",
                length_arg(e, style, "line_width", 0.6 * PT),
            ),
        );
        let stroke = string(
            &layer.args,
            "line_color",
            &string(&layer.args, "border_color", &layer.color),
        );
        let filled = matches!(
            layer.kind.as_str(),
            "area" | "band" | "bar" | "hist" | "boxplot" | "violin" | "contourf"
        );
        if filled {
            let border = string(
                &layer.args,
                "border_color",
                if layer.kind == "area" || layer.kind == "band" {
                    "none"
                } else {
                    &stroke
                },
            );
            let fill = layer
                .args
                .get("fill")
                .map(V::json)
                .unwrap_or(json!(layer.color));
            let pts = vec![
                [x, y - 1.],
                [x + sample, y - 1.],
                [x + sample, y + 1.],
                [x, y + 1.],
            ];
            let mut node = path(&pts, true, w, h, fill.clone(), &border, sw);
            node["opacity"] = json!(opacity);
            children.push(node);
            if layer.args.contains_key("hatch") {
                let primitive = Primitive {
                    points: vec![],
                    holes: vec![],
                    closed: true,
                    fill,
                    stroke: border,
                    width: sw,
                    marker: None,
                    marker_size: 0.,
                    opacity,
                    index: 0,
                };
                let local = vec![vec![[0., 0.], [sample, 0.], [sample, 2.], [0., 2.]]];
                let mut g = group(
                    sample,
                    2.,
                    hatch(e, &primitive, &local, &layer.args, sample, 2.)?,
                );
                g["x"] = json!(x);
                g["y"] = json!(y - 1.);
                g["clip"] = json!({"x":0,"y":0,"width":sample,"height":2});
                children.push(g);
            }
        } else if layer.kind == "errorbar" {
            let cx = x + sample / 2.;
            let cap = length_arg(e, &layer.args, "cap_size", 3. * PT) / 2.;
            let mut d = BezPath::new();
            let mut segment = |a: [f64; 2], b: [f64; 2]| {
                d.move_to((a[0], a[1]));
                d.line_to((b[0], b[1]));
            };
            if layer.args.contains_key("yerr") {
                segment([cx, y - 1.5], [cx, y + 1.5]);
                for yy in [y - 1.5, y + 1.5] {
                    segment([cx - cap, yy], [cx + cap, yy]);
                }
            }
            if layer.args.contains_key("xerr") {
                segment([x, y], [x + sample, y]);
                for xx in [x, x + sample] {
                    segment([xx, y - cap], [xx, y + cap]);
                }
            }
            let mut node = crate::geometry::path_node(d, json!("none"), &stroke, sw);
            node["width"] = json!(w);
            node["height"] = json!(h);
            node["opacity"] = json!(opacity);
            children.push(node);
        } else if layer.kind != "scatter" {
            let mut node = line([x, y], [x + sample, y], w, h, &stroke, sw);
            node["strokeStyle"] = e.stroke(&layer.args, true, sw);
            node["strokeStyle"]["color"] = json!(stroke);
            node["opacity"] = json!(opacity);
            children.push(node);
        }
        if matches!(layer.kind.as_str(), "line" | "scatter" | "errorbar") {
            if let Some(p) = layer.primitives.iter().find(|p| p.marker.is_some()) {
                let mut p = p.clone();
                p.marker_size = length_arg(e, &layer.args, "marker_size", 3. * PT);
                p.opacity = opacity;
                p.fill = layer
                    .args
                    .get("marker_fill")
                    .map(V::json)
                    .unwrap_or(json!(layer.color));
                children.push(marker_node(&p, [x + sample / 2., y], w, h));
            }
        }
        place(&mut n, x + sample + sg, y, [0., 0.5]);
        children.push(n);
    }
    Ok(group(w, h, children))
}
pub(super) fn colorbar_layout(
    e: &mut Engine,
    o: &Object,
    a: &Args,
    style: &Args,
    scale: &Args,
    default_length: f64,
) -> Result<Json> {
    colorbar_layout_inner(e, o, a, style, scale, default_length).map_err(|mut error| {
        if error.code == "E_PLOT" {
            if let Some(V::Map(origin)) = a.get("__call_origin") {
                if let Some(loc) = origin
                    .get("loc")
                    .and_then(|v| serde_json::from_value::<Loc>(v.json()).ok())
                {
                    error.loc = loc;
                }
                if let Some(file) = origin.get("file") {
                    error.file = file.as_str();
                }
            }
        }
        error
    })
}
fn colorbar_layout_inner(
    e: &mut Engine,
    o: &Object,
    a: &Args,
    style: &Args,
    scale: &Args,
    default_length: f64,
) -> Result<Json> {
    let side = string(a, "position", "right");
    let horizontal = string(
        a,
        "orientation",
        if matches!(side.as_str(), "top" | "bottom") {
            "horizontal"
        } else {
            "vertical"
        },
    ) == "horizontal";
    let negative = matches!(side.as_str(), "left" | "top");
    let sign = if negative { -1. } else { 1. };
    let len = length_arg(e, a, "length", default_length);
    let thick = length_arg(e, a, "thickness", 3.);
    let (w, h) = if horizontal {
        (len, thick)
    } else {
        (thick, len)
    };
    let fs = length_arg(
        e,
        style,
        "tick_font_size",
        length_arg(e, style, "font_size", 8. * PT),
    );
    let domain = color_domain(scale);
    let values = if a.contains_key("ticks") {
        nums(a, "ticks")
    } else if scale.contains_key("boundaries") {
        nums(scale, "boundaries")
    } else if string(scale, "norm", "linear") == "log" {
        let aa = Args::from([
            (
                "range".into(),
                V::List(domain.into_iter().map(V::num).collect()),
            ),
            ("scale".into(), V::text("log")),
        ]);
        Axis::new(e, aa, "cb", "left", 0., &[], o.loc)?.ticks()
    } else {
        ticks(domain, 4)
    };
    let mut children = vec![];
    let color = string(style, "color", "#222222");
    let sw = length_arg(e, style, "line_width", 0.6 * PT);
    if string(scale, "norm", "linear") == "boundary" {
        let edges = nums(scale, "boundaries");
        for pair in edges.windows(2) {
            let start = color_position(scale, pair[0]) * len;
            let end = color_position(scale, pair[1]) * len;
            let points = if horizontal {
                vec![[start, 0.], [end, 0.], [end, thick], [start, thick]]
            } else {
                vec![
                    [0., len - end],
                    [thick, len - end],
                    [thick, len - start],
                    [0., len - start],
                ]
            };
            children.push(path(
                &points,
                true,
                w,
                h,
                json!(scale_color(scale, (pair[0] + pair[1]) / 2.)),
                "none",
                0.,
            ));
        }
    } else {
        let stops:Vec<_>=(0..=128).map(|i|{let t=i as f64/128.;json!({"at":t,"opacity":1,"color":scale_color(scale,color_value(scale,if horizontal{t}else{1.-t}))})}).collect();
        children.push(path(&[[0.,0.],[w,0.],[w,h],[0.,h]],true,w,h,json!({"kind":"linearGradient","start":[0,0],"end":if horizontal{[1,0]}else{[0,1]},"stops":stops}),"none",0.));
    }
    let mut tick_spec = a.clone();
    tick_spec.insert("_tick_count".into(), V::num(4.));
    tick_spec.insert("scale".into(), V::text(string(scale, "norm", "linear")));
    let mut tickwidth: f64 = 0.;
    let mut tickheight: f64 = 0.;
    let mut texts = vec![];
    for v in values {
        if v < domain[0] || v > domain[1] {
            return Err(e.error("E_PLOT", "色标刻度须位于颜色范围内", o.loc));
        }
        let t = color_position(scale, v);
        let mut n = number_label(e, v, &tick_spec, domain, style, fs, o)?;
        tickwidth = tickwidth.max(jnum(&n, "width", 0.));
        tickheight = tickheight.max(jnum(&n, "height", 0.));
        let coordinate = if horizontal { t * len } else { (1. - t) * len };
        if horizontal {
            let y = if negative { 0. } else { h };
            children.push(line(
                [coordinate, y],
                [coordinate, y + sign * 1.2],
                w,
                h,
                &color,
                sw,
            ));
            place(
                &mut n,
                coordinate,
                y + sign * 2.4,
                [0.5, if negative { 1. } else { 0. }],
            );
        } else {
            let x = if negative { 0. } else { w };
            children.push(line(
                [x, coordinate],
                [x + sign * 1.2, coordinate],
                w,
                h,
                &color,
                sw,
            ));
            place(
                &mut n,
                x + sign * 2.4,
                coordinate,
                [if negative { 1. } else { 0. }, 0.5],
            );
        }
        texts.push(n);
    }
    let tick_overlap = texts.windows(2).any(|q| {
        let axis = if horizontal { "x" } else { "y" };
        let extent = if horizontal { "width" } else { "height" };
        (jnum(&q[1], axis, 0.) + jnum(&q[1], extent, 0.) / 2.
            - jnum(&q[0], axis, 0.)
            - jnum(&q[0], extent, 0.) / 2.)
            .abs()
            < (jnum(&q[0], extent, 0.) + jnum(&q[1], extent, 0.)) / 2. + 1.2
    });
    let mut long_label = false;
    let factor = if string(a, "notation", "plain") == "offset" && exponent(a, domain) != 0 {
        Some(formula(
            e,
            format!("\\times 10^{{{}}}", exponent(a, domain)),
            style,
            fs,
            o,
        )?)
    } else {
        None
    };
    if let Some(v) = a.get("label") {
        let mut n = label(e, v, style, length_arg(e, style, "label_font_size", fs), o)?;
        long_label = jnum(&n, "width", 0.) > len;
        let off = pair(a, "label_offset", [0., 0.], &e.unit, e.dpi);
        if horizontal {
            let extra = factor
                .as_ref()
                .map(|f| jnum(f, "height", 0.) + 1.2)
                .unwrap_or(0.);
            place(
                &mut n,
                w / 2. + off[0],
                (if negative { 0. } else { h }) + sign * (3.6 + tickheight + extra) + off[1],
                [0.5, if negative { 1. } else { 0. }],
            );
        } else {
            n["rotation"] = json!(-90.);
            let nh = jnum(&n, "height", 0.);
            place(
                &mut n,
                (if negative { 0. } else { w }) + sign * (3.6 + tickwidth + nh / 2.) + off[0],
                h / 2. + off[1],
                [0.5, 0.5],
            );
        }
        texts.push(n);
    }
    if let Some(mut n) = factor {
        let off = pair(a, "exponent_offset", [0., 0.], &e.unit, e.dpi);
        if horizontal {
            place(
                &mut n,
                w + off[0],
                (if negative { 0. } else { h }) + sign * (3.6 + tickheight) + off[1],
                [1., if negative { 1. } else { 0. }],
            );
        } else {
            place(&mut n, off[0], -1.2 + off[1], [0., 1.]);
        }
        texts.push(n);
    }
    let (mut xmin, mut ymin, mut xmax, mut ymax) = (0f64, 0f64, w, h);
    for n in &texts {
        let b = decoration_bounds(n, "");
        xmin = xmin.min(jnum(&b, "left", 0.));
        ymin = ymin.min(jnum(&b, "top", 0.));
        xmax = xmax.max(jnum(&b, "right", 0.));
        ymax = ymax.max(jnum(&b, "bottom", 0.));
    }
    children.extend(texts);
    let mut contents = group(w, h, children);
    contents["x"] = json!(-xmin);
    contents["y"] = json!(-ymin);
    let mut out = group(xmax - xmin, ymax - ymin, vec![contents]);
    out["_barOffset"] = json!([-xmin, -ymin]);
    out["_barSize"] = json!([w, h]);
    out["_tick_overlap"] = json!(tick_overlap);
    out["_long_label"] = json!(long_label);
    Ok(out)
}
pub(super) fn colorbar_source<'a>(o: &Object, layers: &'a [Layer], a: &Args) -> Option<&'a Args> {
    let index = a
        .get("_0")
        .and_then(V::object)
        .and_then(|obj| obj.borrow().args.get("_index").and_then(V::number))
        .map(|n| n as usize);
    let layer = index
        .and_then(|index| {
            layers.get(
                o.layers[..index.min(o.layers.len())]
                    .iter()
                    .filter(|(k, _)| !matches!(k.as_str(), "legend" | "colorbar" | "add_axis"))
                    .count(),
            )
        })
        .or_else(|| layers.iter().find(|l| l.scale.is_some()));
    layer.and_then(|l| l.scale.as_ref())
}
pub fn add_decorations(
    e: &mut Engine,
    o: &Object,
    style: &Args,
    layers: &[Layer],
    area: [f64; 4],
    size: [f64; 2],
    children: &mut Vec<Json>,
    decorations: &mut Vec<Json>,
) -> Result<()> {
    for (kind, raw) in &o.layers {
        if !matches!(kind.as_str(), "legend" | "colorbar") {
            continue;
        }
        let mut decoration_style = style.clone();
        if kind == "legend" {
            decoration_style.insert(
                "color".into(),
                object_args(&o.args, "style")
                    .get("color")
                    .cloned()
                    .unwrap_or(V::text("#222222")),
            );
        }
        let resolved = part_style(e, &o.args, &decoration_style, kind, None, raw)?;
        let a = &resolved;
        if kind == "legend" {
            let mut n = legend_layout(e, o, a, &resolved, layers)?;
            let pos = string(a, "position", "top_right");
            let manual = matches!(a.get("position"), Some(V::List(_)));
            let coords = if manual {
                let xy = pair(a, "position", [0., 0.], &e.unit, e.dpi);
                [area[0] + xy[0], area[1] + xy[1]]
            } else {
                [
                    area[0]
                        + if pos.ends_with("right") {
                            area[2] - jnum(&n, "width", 0.) - 1.2
                        } else {
                            1.2
                        },
                    area[1]
                        + if pos.starts_with("bottom") {
                            area[3] - jnum(&n, "height", 0.) - 1.2
                        } else {
                            1.2
                        },
                ]
            };
            place(&mut n, coords[0], coords[1], [0., 0.]);
            let mut bounds = decoration_bounds(&n, "legend");
            bounds["overflow_data_area"] = json!(
                !manual && (jnum(&n, "width", 0.) > area[2] || jnum(&n, "height", 0.) > area[3])
            );
            bounds["manual"] = json!(matches!(a.get("position"), Some(V::List(_))));
            decorations.push(bounds);
            children.push(n);
        } else if kind == "colorbar" {
            let scale = colorbar_source(o, layers, a)
                .ok_or_else(|| e.error("E_PLOT", "colorbar 需要本图表具有颜色映射的图层", o.loc))?;
            let side = string(a, "position", "right");
            let horizontal = string(
                a,
                "orientation",
                if matches!(side.as_str(), "top" | "bottom") {
                    "horizontal"
                } else {
                    "vertical"
                },
            ) == "horizontal";
            let mut n = colorbar_layout(
                e,
                o,
                a,
                style,
                scale,
                if horizontal { area[2] } else { area[3] },
            )?;
            let gap = length_arg(e, a, "gap", 2.4);
            let mut needed = [0f64; 4];
            for d in decorations.iter() {
                if jstr(d, "name", "") == "legend" {
                    continue;
                }
                needed[0] = needed[0].max(area[0] - jnum(d, "left", area[0]));
                needed[1] = needed[1].max(area[1] - jnum(d, "top", area[1]));
                needed[2] = needed[2].max(jnum(d, "right", area[0] + area[2]) - area[0] - area[2]);
                needed[3] = needed[3].max(jnum(d, "bottom", area[1] + area[3]) - area[1] - area[3]);
            }
            let bw = n["_barSize"][0].as_f64().unwrap();
            let bh = n["_barSize"][1].as_f64().unwrap();
            let xy = if matches!(a.get("position"), Some(V::List(_))) {
                let p = pair(a, "position", [0., 0.], &e.unit, e.dpi);
                [area[0] + p[0], area[1] + p[1]]
            } else {
                match side.as_str() {
                    "left" => [
                        area[0] - needed[0] - gap - bw,
                        area[1] + (area[3] - bh) / 2.,
                    ],
                    "top" => [
                        area[0] + (area[2] - bw) / 2.,
                        area[1] - needed[1] - gap - bh,
                    ],
                    "bottom" => [
                        area[0] + (area[2] - bw) / 2.,
                        area[1] + area[3] + needed[3] + gap,
                    ],
                    _ => [area[0] + area[2] + gap, area[1] + (area[3] - bh) / 2.],
                }
            };
            let bx = n["_barOffset"][0].as_f64().unwrap();
            let by = n["_barOffset"][1].as_f64().unwrap();
            place(&mut n, xy[0] - bx, xy[1] - by, [0., 0.]);
            let mut bounds = decoration_bounds(&n, "colorbar");
            bounds["tick_overlap"] = n["_tick_overlap"].clone();
            bounds["long_label"] = n["_long_label"].clone();
            bounds["manual"] = json!(matches!(a.get("position"), Some(V::List(_))));
            decorations.push(bounds);
            children.push(n);
        }
    }
    let _ = size;
    Ok(())
}
pub fn warn_layout(
    e: &mut Engine,
    o: &Object,
    decorations: &[Json],
    size: [f64; 2],
    fixed: bool,
    origin: Option<&V>,
) {
    let loc = origin
        .and_then(|v| {
            if let V::Map(a) = v {
                a.get("loc")
            } else {
                None
            }
        })
        .and_then(|v| serde_json::from_value(v.json()).ok())
        .unwrap_or(o.loc);
    for n in decorations {
        if n["overflow_data_area"] == true {
            e.warn("W_PLOT_LAYOUT", "图例超出绘图区；已保留完整内容", loc);
        }
        if n["tick_overlap"] == true {
            e.warn("W_PLOT_LAYOUT", "色标刻度标签重叠；已保留最终刻度", loc);
        }
        if n["long_label"] == true {
            e.warn("W_PLOT_LAYOUT", "轴标签长于绘图区；已保留文字", loc);
        }
    }
    if let Some(area) = o.args.get("plot_area").and_then(V::object) {
        let wh = pair(&area.borrow().args, "size", size, &e.unit, e.dpi);
        if wh[0] <= 2. || wh[1] <= 2. {
            e.warn(
                "W_PLOT_LAYOUT",
                "绘图区宽或高不超过 2 mm；已保留尺寸，内容可能难以辨认",
                loc,
            );
        }
        for n in decorations
            .iter()
            .filter(|n| jstr(n, "name", "").ends_with(".label"))
        {
            let horizontal = jstr(n, "name", "").starts_with("x.");
            let extent = if horizontal {
                jnum(n, "right", 0.) - jnum(n, "left", 0.)
            } else {
                jnum(n, "bottom", 0.) - jnum(n, "top", 0.)
            };
            if extent > wh[usize::from(!horizontal)] {
                e.warn("W_PLOT_LAYOUT", "轴标签长于绘图区；已保留文字", loc);
            }
        }
    }
    let aggregate = fixed && string(&o.args, "projection", "cartesian") == "cartesian";
    if aggregate {
        let mut overflow = [0f64; 4];
        for n in decorations
            .iter()
            .filter(|n| n["manual"] != true && n["name"] != "legend")
        {
            for (i, v) in [
                -jnum(n, "left", 0.),
                -jnum(n, "top", 0.),
                jnum(n, "right", 0.) - size[0],
                jnum(n, "bottom", 0.) - size[1],
            ]
            .into_iter()
            .enumerate()
            {
                overflow[i] = overflow[i].max(v + 1.);
            }
        }
        let sides = ["左", "上", "右", "下"];
        let shortages: Vec<_> = overflow
            .iter()
            .enumerate()
            .filter(|(_, v)| **v > 1e-6)
            .map(|(i, v)| format!("{}侧缺少约 {v:.2} mm", sides[i]))
            .collect();
        if !shortages.is_empty() {
            e.warn(
                "W_PLOT_LAYOUT",
                format!(
                    "{}不足以容纳刻度、标签或色标（{}）；已保留设置，文字可能重叠或超出图表外框",
                    if o.args.contains_key("margins") {
                        "固定留白"
                    } else {
                        "plot_area 周围空间"
                    },
                    shortages.join("，")
                ),
                loc,
            );
        }
    }
    for n in decorations {
        if aggregate && n["manual"] != true {
            continue;
        }
        if jnum(n, "left", 0.) < -0.1
            || jnum(n, "top", 0.) < -0.1
            || jnum(n, "right", 0.) > size[0] + 0.1
            || jnum(n, "bottom", 0.) > size[1] + 0.1
        {
            e.warn(
                "W_PLOT_LAYOUT",
                if !fixed && jstr(n, "name", "").ends_with(".label") {
                    format!("{} 标题偏移后超出图表外框；已保留设置", jstr(n, "name", ""))
                } else if n["manual"] == true {
                    format!(
                        "手动{}超出图表外框；已保留设置和完整内容",
                        if jstr(n, "name", "") == "legend" {
                            "图例"
                        } else {
                            "色标"
                        }
                    )
                } else {
                    format!(
                        "{} {}超出图表外框；已保留设置和完整内容",
                        jstr(n, "name", ""),
                        if fixed {
                            "固定绘图区周围空间不足，"
                        } else {
                            ""
                        }
                    )
                },
                loc,
            );
        }
    }
    for (i, a) in decorations.iter().enumerate() {
        for b in &decorations[i + 1..] {
            let an = jstr(a, "name", "");
            let bn = jstr(b, "name", "");
            if an == "legend" || bn == "legend" {
                continue;
            }
            if jnum(a, "left", 0.) < jnum(b, "right", 0.)
                && jnum(a, "right", 0.) > jnum(b, "left", 0.)
                && jnum(a, "top", 0.) < jnum(b, "bottom", 0.)
                && jnum(a, "bottom", 0.) > jnum(b, "top", 0.)
            {
                e.warn(
                    "W_PLOT_LAYOUT",
                    if an
                        .strip_suffix(".label")
                        .is_some_and(|name| bn == format!("{name}.exponent"))
                        || bn
                            .strip_suffix(".label")
                            .is_some_and(|name| an == format!("{name}.exponent"))
                    {
                        format!(
                            "{} 轴标题与倍率重叠；已保留位置，可调整偏移",
                            an.split('.').next().unwrap_or(an)
                        )
                    } else if an.contains(".tick:")
                        && bn.contains(".tick:")
                        && an.split('.').next() == bn.split('.').next()
                    {
                        format!(
                            "{} 轴刻度标签重叠；已保留最终刻度",
                            an.split('.').next().unwrap()
                        )
                    } else {
                        format!("{an} 与 {bn} 重叠；已保留位置，可调整偏移或间距")
                    },
                    loc,
                );
            }
        }
    }
}
pub fn decoration(e: &mut Engine, o: &Object, a: &Args) -> Result<Json> {
    let mut style = object_args(a, "style");
    if o.kind == "legend" {
        let mut layers = vec![];
        for v in array(a, "layers") {
            let obj = v
                .object()
                .ok_or_else(|| e.error("E_PLOT", "legend 需要图层列表", o.loc))?;
            let obj = obj.borrow();
            let owner = obj.args.get("_owner").and_then(V::object);
            if let Some(owner) = owner {
                let owner = owner.borrow();
                if style.is_empty() {
                    style = object_args(&owner.args, "style");
                }
                let projection = string(&owner.args, "projection", "cartesian");
                let warning_count = e.warnings.len();
                layers.push(build(
                    e,
                    &string(&obj.args, "_type", "line"),
                    &obj.args,
                    num(&obj.args, "_index", 0.) as usize,
                    &style,
                    &projection,
                    &owner.args,
                    o.loc,
                )?);
                e.warnings.truncate(warning_count);
            }
        }
        legend_layout(e, o, a, &style, &layers)
    } else {
        let scale = object_args(a, "scale");
        if scale.is_empty() {
            return Err(e.error("E_PLOT", "colorbar 需要 color_scale", o.loc));
        }
        style
            .entry("color".into())
            .or_insert_with(|| V::text("#222222"));
        colorbar_layout(e, o, a, &style, &scale, 40.)
    }
}
