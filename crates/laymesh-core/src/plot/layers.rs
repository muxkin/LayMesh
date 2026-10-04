use super::*;
#[derive(Clone)]
pub struct Primitive {
    pub points: Vec<[f64; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
    pub closed: bool,
    pub fill: Json,
    pub stroke: String,
    pub width: f64,
    pub marker: Option<String>,
    pub marker_size: f64,
    pub opacity: f64,
    pub index: usize,
}
#[derive(Clone)]
pub struct Layer {
    pub kind: String,
    pub args: Args,
    pub primitives: Vec<Primitive>,
    pub xaxis: String,
    pub yaxis: String,
    pub color: String,
    pub scale: Option<Args>,
    pub statistics: Json,
    pub zero: Option<usize>,
    pub actual: Option<Vec<f64>>,
    pub extent: Option<[f64; 4]>,
    pub masks: Option<Vec<[f64; 4]>>,
}
fn primitive(
    points: Vec<[f64; 2]>,
    closed: bool,
    fill: Json,
    stroke: &str,
    width: f64,
    opacity: f64,
) -> Primitive {
    Primitive {
        points,
        holes: vec![],
        closed,
        fill,
        stroke: stroke.into(),
        width,
        marker: None,
        marker_size: 1.,
        opacity,
        index: 0,
    }
}
fn vec_checked(e: &Engine, a: &Args, key: &str, l: Loc) -> Result<Vec<f64>> {
    let values = nums(a, key);
    if values.is_empty() {
        return Err(e.error("E_DATA", format!("{key} 数据数组不能为空"), l));
    }
    Ok(values)
}
fn expand(a: &Args, key: &str, n: usize, default: f64) -> Vec<f64> {
    if matches!(a.get(key), Some(V::List(_))) {
        nums(a, key)
    } else {
        vec![num(a, key, default); n]
    }
}
fn sorted(v: &[f64]) -> Vec<f64> {
    let mut v: Vec<_> = v.iter().copied().filter(|v| v.is_finite()).collect();
    v.sort_by(f64::total_cmp);
    v
}
pub fn quantile(v: &[f64], q: f64) -> f64 {
    let pos = (v.len() - 1) as f64 * q;
    let i = pos.floor() as usize;
    v[i] + (v[pos.ceil() as usize] - v[i]) * (pos - i as f64)
}
pub fn build(
    e: &mut Engine,
    kind: &str,
    args: &Args,
    index: usize,
    style: &Args,
    projection: &str,
    plotargs: &Args,
    l: Loc,
) -> Result<Layer> {
    let mut a = args.clone();
    if projection == "polar" {
        for (from, to) in [
            ("theta", "x"),
            ("r", "y"),
            ("theta_edges", "x_edges"),
            ("r_edges", "y_edges"),
            ("thetaerr", "xerr"),
            ("rerr", "yerr"),
        ] {
            if let Some(v) = a.remove(from) {
                a.insert(to.into(), v);
            }
        }
    } else if projection == "radar" {
        let values = vec_checked(e, &a, "values", l)?;
        a.insert(
            "x".into(),
            V::List((0..values.len()).map(|i| V::num(i as f64)).collect()),
        );
        a.insert(
            "y".into(),
            V::List(values.iter().map(|v| V::num(*v)).collect()),
        );
    }
    let palette = array(style, "colors");
    let default_color = if palette.is_empty() {
        PALETTE[index % PALETTE.len()].to_string()
    } else {
        palette[index % palette.len()].as_str()
    };
    let color = string(&a, "color", &default_color);
    a.entry("color".into()).or_insert_with(|| V::text(&color));
    let stroke = string(&a, "line_color", &string(&a, "border_color", &color));
    let fill = a.get("fill").map(V::json).unwrap_or(json!(color));
    let sw = length_arg(
        e,
        &a,
        "line_width",
        length_arg(
            e,
            &a,
            "border_width",
            length_arg(e, style, "line_width", 0.6 * PT),
        ),
    );
    let opacity = num(
        &a,
        "opacity",
        if matches!(kind, "band" | "area") {
            0.2
        } else {
            1.
        },
    );
    if !(0. ..=1.).contains(&opacity) {
        return Err(e.error("E_PLOT", "opacity 须在 0 到 1 之间", l));
    }
    let mut layer = Layer {
        kind: kind.into(),
        xaxis: string(&a, "x_axis", "x"),
        yaxis: string(&a, "y_axis", "y"),
        color: color.clone(),
        scale: a
            .get("color_scale")
            .and_then(V::object)
            .map(|o| o.borrow().args.clone()),
        args: a.clone(),
        primitives: vec![],
        statistics: Json::Null,
        zero: None,
        actual: None,
        extent: None,
        masks: None,
    };
    let horizontal = string(&a, "orientation", "vertical") == "horizontal";
    let point = |p: f64, v: f64| if horizontal { [v, p] } else { [p, v] };
    let rect = |p: f64, w: f64, lo: f64, hi: f64| {
        primitive(
            vec![
                point(p - w / 2., lo),
                point(p + w / 2., lo),
                point(p + w / 2., hi),
                point(p - w / 2., hi),
            ],
            true,
            fill.clone(),
            &string(&a, "border_color", &stroke),
            sw,
            opacity,
        )
    };
    if matches!(kind, "bar" | "hist" | "boxplot" | "violin") {
        let mut values = vec_checked(e, &a, "values", l)?;
        if kind == "hist" && projection == "polar" {
            let period = if string(plotargs, "angle_unit", "deg") == "rad" {
                std::f64::consts::TAU
            } else {
                360.
            };
            let theta = object_args(plotargs, "theta");
            let start = nums(&theta, "range").first().copied().unwrap_or(0.);
            for v in &mut values {
                *v = start + (*v - start).rem_euclid(period);
            }
        }
        let finite = sorted(&values);
        if finite.is_empty() {
            return Err(e.error("E_DATA", "统计图没有有效数据", l));
        }
        if finite.len() < values.len() {
            e.warn("W_PLOT_MISSING", "图层包含缺失值，将排除", l);
        }
        if kind == "bar" {
            let pos = vec_checked(e, &a, "positions", l)?;
            let baseline = expand(&a, "baseline", values.len(), 0.);
            if pos.len() != values.len() || baseline.len() != values.len() {
                return Err(e.error("E_PLOT", "柱位置、数值和 baseline 必须等长", l));
            }
            let width = num(
                &a,
                if projection == "polar" {
                    "angle_width"
                } else {
                    "data_width"
                },
                if projection == "polar" {
                    if string(plotargs, "angle_unit", "deg") == "rad" {
                        std::f64::consts::PI / 9.
                    } else {
                        20.
                    }
                } else {
                    0.8
                },
            );
            if width <= 0. {
                return Err(e.error("E_PLOT", "柱宽必须大于零", l));
            }
            if projection == "polar"
                && width
                    > if string(plotargs, "angle_unit", "deg") == "rad" {
                        std::f64::consts::TAU
                    } else {
                        360.
                    }
            {
                return Err(e.error("E_PLOT", "角宽不能超过一周", l));
            }
            if baseline.contains(&0.) {
                layer.zero = Some(if horizontal { 0 } else { 1 });
            }
            layer.actual = Some(
                values
                    .iter()
                    .zip(&baseline)
                    .flat_map(|(v, b)| {
                        if *b == 0. {
                            if *v == 0. { vec![] } else { vec![*v] }
                        } else {
                            vec![*b, *b + *v]
                        }
                    })
                    .collect(),
            );
            for i in 0..values.len() {
                if [pos[i], values[i], baseline[i]]
                    .iter()
                    .all(|v| v.is_finite())
                {
                    layer.primitives.push(rect(
                        pos[i],
                        width,
                        baseline[i],
                        baseline[i] + values[i],
                    ));
                }
            }
        } else if kind == "hist" {
            let weights = expand(&a, "weights", values.len(), 1.);
            if weights.len() != values.len() {
                return Err(e.error("E_PLOT", "weights 须与 values 等长", l));
            }
            if weights.iter().any(|v| !v.is_finite()) {
                e.warn("W_PLOT_MISSING", "直方图排除缺失权重", l);
            }
            let observations = sorted(
                &values
                    .iter()
                    .zip(&weights)
                    .filter_map(|(v, w)| {
                        if v.is_finite() && w.is_finite() {
                            Some(*v)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            if observations.is_empty() {
                return Err(e.error("E_PLOT", "直方图没有有效观测值及权重", l));
            }

            let edges = if matches!(a.get("bins"), Some(V::List(_))) {
                nums(&a, "bins")
            } else {
                let n = num(&a, "bins", 10.);
                if n < 1. || n > 100000. || n.fract() != 0. {
                    return Err(e.error("E_PLOT", "bins 需要 1 到 100000 的整数", l));
                }
                let mut lo = observations[0];
                let mut hi = *observations.last().unwrap();
                if projection == "polar" {
                    let period = if string(plotargs, "angle_unit", "deg") == "rad" {
                        std::f64::consts::TAU
                    } else {
                        360.
                    };
                    let range = nums(&object_args(plotargs, "theta"), "range");
                    lo = range.first().copied().unwrap_or(0.);
                    hi = range.get(1).copied().unwrap_or(period);
                }
                if lo == hi {
                    let pad = (lo.abs() * 0.05).max(0.5);
                    lo -= pad;
                    hi += pad;
                }
                linspace(lo, hi, n as usize + 1)
            };
            if edges.len() < 2 || edges.windows(2).any(|v| v[1] <= v[0]) {
                return Err(e.error("E_PLOT", "分箱边界必须严格递增", l));
            }
            let weights = expand(&a, "weights", values.len(), 1.);
            if weights.len() != values.len() || weights.iter().any(|v| *v < 0.) {
                return Err(e.error("E_PLOT", "weights 须非负并与 values 等长", l));
            }
            let mut counts = vec![0.; edges.len() - 1];
            let (mut total, mut excluded) = (0., 0);
            for (v, w) in values.iter().zip(weights) {
                if !v.is_finite() || !w.is_finite() {
                    continue;
                }
                if *v < edges[0] || *v > *edges.last().unwrap() {
                    excluded += 1;
                    continue;
                }
                let i = edges
                    .partition_point(|edge| edge <= v)
                    .saturating_sub(1)
                    .min(counts.len() - 1);
                counts[i] += w;
                total += w;
            }
            if !total.is_finite() || total <= 0. {
                return Err(e.error("E_PLOT", "直方图纳入的样本总权重必须大于零", l));
            }
            if excluded > 0 {
                e.warn(
                    "W_PLOT_MISSING",
                    format!("{excluded} 个样本超出显式分箱范围"),
                    l,
                );
            }
            let stat = string(&a, "stat", "count");
            let heights: Vec<_> = counts
                .iter()
                .enumerate()
                .map(|(i, v)| match stat.as_str() {
                    "density" => v / total / (edges[i + 1] - edges[i]),
                    "probability" => v / total,
                    _ => *v,
                })
                .collect();
            for (i, v) in heights.iter().enumerate() {
                layer.primitives.push(rect(
                    (edges[i] + edges[i + 1]) / 2.,
                    edges[i + 1] - edges[i],
                    0.,
                    *v,
                ));
            }
            layer.zero = Some(if horizontal { 0 } else { 1 });
            layer.actual = Some(heights.iter().copied().filter(|v| *v > 0.).collect());
            layer.statistics = json!({"edges":edges,"counts":counts,"heights":heights,"total":total,"excluded":excluded});
        } else {
            let (q1, median, q3) = (
                quantile(&finite, 0.25),
                quantile(&finite, 0.5),
                quantile(&finite, 0.75),
            );
            let whisker = num(&a, "whisker", 1.5);
            let lower = finite
                .iter()
                .copied()
                .find(|v| *v >= q1 - whisker * (q3 - q1))
                .unwrap()
                .min(q1);
            let upper = finite
                .iter()
                .copied()
                .rev()
                .find(|v| *v <= q3 + whisker * (q3 - q1))
                .unwrap()
                .max(q3);
            let outliers: Vec<_> = finite
                .iter()
                .copied()
                .filter(|v| *v < lower || *v > upper)
                .collect();
            layer.statistics = json!({"q1":q1,"median":median,"q3":q3,"lower":lower,"upper":upper,"outliers":outliers});
            let p = num(&a, "position", 0.);
            let w = num(&a, "data_width", 0.8);
            if kind == "boxplot" {
                layer.primitives.push(rect(p, w, q1, q3));
                layer.primitives.push(primitive(
                    vec![point(p - w / 2., median), point(p + w / 2., median)],
                    false,
                    json!("none"),
                    &string(&a, "median_color", &stroke),
                    length_arg(e, &a, "median_line_width", sw),
                    opacity,
                ));
                for (a, b, c, d) in [
                    (p, lower, p, q1),
                    (p, q3, p, upper),
                    (p - w / 4., lower, p + w / 4., lower),
                    (p - w / 4., upper, p + w / 4., upper),
                ] {
                    layer.primitives.push(primitive(
                        vec![point(a, b), point(c, d)],
                        false,
                        json!("none"),
                        &string(&layer.args, "whisker_color", &stroke),
                        length_arg(e, &layer.args, "whisker_line_width", sw),
                        opacity,
                    ));
                }
                if yes(&a, "outliers", true) {
                    for v in outliers {
                        let mut p = primitive(
                            vec![point(p, v)],
                            false,
                            a.get("outlier_fill").map(V::json).unwrap_or(json!(stroke)),
                            &string(&a, "outlier_border_color", "none"),
                            length_arg(e, &a, "outlier_border_width", sw),
                            opacity,
                        );
                        p.marker = Some(string(&a, "outlier_marker", "circle"));
                        p.marker_size = length_arg(e, &a, "outlier_size", 1.);
                        layer.primitives.push(p);
                    }
                }
            } else {
                let n = num(&a, "points", 128.) as usize;
                if !(2..=10000).contains(&n) {
                    return Err(e.error("E_PLOT", "points 须为 2 到 10000 的整数", l));
                }
                let mean = finite.iter().sum::<f64>() / finite.len() as f64;
                let sd = (finite.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                    / (finite.len() as f64 - 1.))
                    .sqrt();
                let h = num(&a, "bandwidth", sd * (finite.len() as f64).powf(-0.2));
                let degenerate = !h.is_finite() || h <= 0. || finite[0] == *finite.last().unwrap();
                if degenerate {
                    e.warn(
                        "W_PLOT_MISSING",
                        "样本不足或为常量，小提琴图仅显示中位数",
                        l,
                    );
                } else {
                    let positions = linspace(finite[0], *finite.last().unwrap(), n);
                    let density: Vec<_> = positions
                        .iter()
                        .map(|x| {
                            finite
                                .iter()
                                .map(|v| (-0.5 * ((x - v) / h).powi(2)).exp())
                                .sum::<f64>()
                                / (finite.len() as f64 * h * std::f64::consts::TAU.sqrt())
                        })
                        .collect();
                    let max = density.iter().copied().fold(0., f64::max);
                    let mut pts: Vec<_> = positions
                        .iter()
                        .zip(&density)
                        .map(|(v, d)| point(p - d / max * w / 2., *v))
                        .collect();
                    pts.extend(
                        positions
                            .iter()
                            .zip(&density)
                            .rev()
                            .map(|(v, d)| point(p + d / max * w / 2., *v)),
                    );
                    layer
                        .primitives
                        .push(primitive(pts, true, fill.clone(), &stroke, sw, opacity));
                    layer.statistics["bandwidth"] = json!(h);
                    layer.statistics["positions"] = json!(positions);
                    layer.statistics["density"] = json!(density);
                }
                layer.primitives.push(primitive(
                    vec![
                        point(p - w / if degenerate { 2. } else { 4. }, median),
                        point(p + w / if degenerate { 2. } else { 4. }, median),
                    ],
                    false,
                    json!("none"),
                    &stroke,
                    sw,
                    opacity,
                ));
            }
        }
    } else if kind == "heatmap" || kind == "contour" || kind == "contourf" {
        grid_layer(e, &mut layer, projection, plotargs, l, sw, opacity)?;
    } else if kind == "hline" || kind == "vline" {
        let v = num(&a, if kind == "hline" { "y" } else { "x" }, f64::NAN);
        if !v.is_finite() {
            return Err(e.error("E_PLOT", "参考线需要有限值", l));
        }
        layer.primitives.push(primitive(
            if kind == "hline" {
                vec![[f64::NEG_INFINITY, v], [f64::INFINITY, v]]
            } else {
                vec![[v, f64::NEG_INFINITY], [v, f64::INFINITY]]
            },
            false,
            json!("none"),
            &stroke,
            sw,
            opacity,
        ));
    } else {
        let (xs, ys) = if kind == "ecdf" {
            let values = vec_checked(e, &a, "values", l)?;
            let v = sorted(&values);
            if v.is_empty() {
                return Err(e.error("E_PLOT", "ECDF 没有有效数据", l));
            }
            if v.len() != values.len() {
                e.warn("W_PLOT_MISSING", "ECDF 排除缺失值", l);
            }
            let (mut x, mut y) = (vec![], vec![]);
            for (i, vv) in v.iter().enumerate() {
                if v.get(i + 1) != Some(vv) {
                    x.push(*vv);
                    y.push((i + 1) as f64 / v.len() as f64);
                }
            }
            (x, y)
        } else {
            (
                vec_checked(e, &a, "x", l)?,
                vec_checked(e, &a, if kind == "band" { "lower" } else { "y" }, l)?,
            )
        };
        if xs.len() != ys.len() {
            return Err(e.error("E_PLOT", "x/y 数据长度不匹配", l));
        }
        let n = xs.len();
        if matches!(kind, "area" | "ecdf") {
            layer.actual = Some(ys.clone());
        }

        let upper = if kind == "band" {
            vec_checked(e, &a, "upper", l)?
        } else {
            vec![]
        };
        if kind == "band" && (upper.len() != n || ys.iter().zip(&upper).any(|(lo, hi)| lo > hi)) {
            return Err(e.error("E_PLOT", "填充带数组长度不匹配或 lower 大于 upper", l));
        }
        let baseline = if kind == "area" && projection != "radar" {
            let baseline = expand(&a, "baseline", n, 0.);
            if baseline.len() != n {
                return Err(e.error("E_PLOT", "baseline 必须与 y 等长", l));
            }
            if baseline.iter().any(|v| *v == 0.) {
                layer.zero = Some(1);
                layer.actual = Some(
                    ys.iter()
                        .zip(&baseline)
                        .flat_map(|(v, b)| {
                            if !v.is_finite() || !b.is_finite() || *v == 0. && *b == 0. {
                                vec![]
                            } else if *b == 0. {
                                vec![*v]
                            } else {
                                vec![*v, *b]
                            }
                        })
                        .collect(),
                );
            }
            baseline
        } else {
            vec![]
        };
        let mut errors: [Option<(Vec<f64>, Vec<f64>)>; 2] = [None, None];
        if kind == "errorbar" {
            if !a.contains_key("xerr") && !a.contains_key("yerr") {
                return Err(e.error("E_PLOT", "errorbar 至少需要 xerr 或 yerr", l));
            }
            for (dim, key) in ["xerr", "yerr"].iter().enumerate() {
                if !a.contains_key(*key) {
                    continue;
                }
                let err = array(&a, key);
                let (lo, hi) = if matches!(err.first(), Some(V::List(_))) {
                    if err.len() != 2 {
                        return Err(e.error("E_PLOT", "非对称误差需要两个数组", l));
                    }
                    (
                        err[0]
                            .list()
                            .iter()
                            .map(|v| v.number().unwrap_or(f64::NAN))
                            .collect::<Vec<_>>(),
                        err[1]
                            .list()
                            .iter()
                            .map(|v| v.number().unwrap_or(f64::NAN))
                            .collect::<Vec<_>>(),
                    )
                } else {
                    let values = expand(&a, key, n, 0.);
                    (values.clone(), values)
                };
                if lo.len() != n || hi.len() != n || lo.iter().chain(&hi).any(|v| *v < 0.) {
                    return Err(e.error("E_PLOT", "误差不能为负，且长度必须匹配", l));
                }
                let centers = if dim == 0 { &xs } else { &ys };
                let lo: Vec<_> = centers.iter().zip(lo).map(|(v, d)| v - d).collect();
                let hi: Vec<_> = centers.iter().zip(hi).map(|(v, d)| v + d).collect();
                if lo.iter().chain(&hi).any(|v| v.is_infinite()) {
                    return Err(e.error("E_PLOT", "误差范围溢出", l));
                }
                errors[dim] = Some((lo, hi));
            }
        }
        let marker = string(
            &a,
            "marker",
            if kind == "scatter" { "circle" } else { "none" },
        );
        let marker_sizes = if matches!(a.get("marker_size"), Some(V::List(_))) {
            array(&a, "marker_size")
                .iter()
                .map(|v| value_length(v, &e.unit, e.dpi).unwrap_or(f64::NAN))
                .collect()
        } else {
            vec![length_arg(e, &a, "marker_size", 3. * PT); n]
        };
        let point_opacities = expand(&a, "opacity", n, opacity);
        let colors = nums(&a, "c");
        if a.contains_key("c") && layer.scale.is_none() {
            return Err(e.error("E_PLOT", "c 需要 color_scale", l));
        }
        if kind == "scatter" && layer.scale.is_some() && !a.contains_key("c") {
            return Err(e.error("E_PLOT", "散点 color_scale 需要 c 数据", l));
        }
        if a.contains_key("c") && a.contains_key("marker_fill") {
            return Err(e.error("E_PLOT", "c 与 marker_fill 不同时指定", l));
        }
        if layer
            .scale
            .as_ref()
            .is_some_and(|s| string(s, "norm", "linear") == "log")
            && colors.iter().any(|v| *v <= 0.)
        {
            return Err(e.error("E_PLOT", "对数颜色值须为正", l));
        }
        if marker_sizes.len() != n
            || point_opacities.len() != n
            || (!colors.is_empty() && colors.len() != n)
        {
            return Err(e.error("E_PLOT", "逐点尺寸、透明度、颜色需要等长数组", l));
        }
        let valid: Vec<_> = (0..n)
            .map(|i| {
                xs[i].is_finite()
                    && ys[i].is_finite()
                    && (kind != "band" || upper[i].is_finite())
                    && (baseline.is_empty() || baseline[i].is_finite())
                    && (colors.is_empty() || colors[i].is_finite())
                    && errors
                        .iter()
                        .flatten()
                        .all(|(lo, hi)| lo[i].is_finite() && hi[i].is_finite())
            })
            .collect();
        let missing = valid.iter().filter(|v| !**v).count();
        if missing == n {
            return Err(e.error("E_PLOT", "图层没有有效数据", l));
        }
        if missing > 0 {
            e.warn(
                "W_PLOT_MISSING",
                "图层包含缺失数据点；折线断开，散点跳过",
                l,
            );
        }
        if projection == "radar" && kind == "area" && missing > 0 {
            e.warn("W_PLOT_MISSING", "雷达图缺失值：断线并跳过封闭面积", l);
        }
        if kind == "errorbar" {
            for (dim, error) in errors.iter().enumerate() {
                let Some((lo, hi)) = error else {
                    continue;
                };
                for i in 0..n {
                    if !valid[i] {
                        continue;
                    }
                    let mut low = [xs[i], ys[i]];
                    let mut high = low;
                    low[dim] = lo[i];
                    high[dim] = hi[i];
                    let mut p =
                        primitive(vec![low, high], false, json!("none"), &stroke, sw, opacity);
                    p.index = dim + 1;
                    layer.primitives.push(p);
                }
            }
        } else if kind != "scatter" {
            let mut start = 0;
            while start < n {
                while start < n && !valid[start] {
                    start += 1;
                }
                if start == n {
                    break;
                }
                let mut end = start + 1;
                while end < n && valid[end] {
                    end += 1;
                }
                let mut pts: Vec<_> = (start..end).map(|i| [xs[i], ys[i]]).collect();
                if kind == "step" || kind == "ecdf" {
                    let where_ = string(&a, "where", "post");
                    let mut stepped = vec![pts[0]];
                    if kind == "ecdf" {
                        stepped.insert(0, [pts[0][0], 0.]);
                        layer.zero = Some(1);
                    }
                    for q in pts.windows(2) {
                        match where_.as_str() {
                            "post" => stepped.push([q[1][0], q[0][1]]),
                            "mid" => {
                                let m = (q[0][0] + q[1][0]) / 2.;
                                stepped.push([m, q[0][1]]);
                                stepped.push([m, q[1][1]]);
                            }
                            _ => stepped.push([q[0][0], q[1][1]]),
                        }
                        stepped.push(q[1]);
                    }
                    pts = stepped;
                }
                let mut closed =
                    kind == "band" || kind == "area" || yes(&a, "closed", projection == "radar");
                let mut paint = json!("none");
                if kind == "band" {
                    pts.extend((start..end).rev().map(|i| [xs[i], upper[i]]));
                    paint = fill.clone();
                } else if kind == "area" && projection != "radar" {
                    // Preserve the baseline at every sample, including crossings.
                    pts = (start..end)
                        .map(|i| [xs[i], ys[i].min(baseline[i])])
                        .collect();
                    pts.extend((start..end).rev().map(|i| [xs[i], ys[i].max(baseline[i])]));
                    paint = fill.clone();
                } else if projection == "radar" && kind == "area" {
                    if missing == 0 {
                        paint = fill.clone();
                    } else {
                        closed = false;
                    }
                }
                let scol = if matches!(kind, "area" | "band") {
                    string(&a, "border_color", "none")
                } else {
                    stroke.clone()
                };
                layer
                    .primitives
                    .push(primitive(pts, closed, paint, &scol, sw, opacity));
                start = end;
            }
        }
        if marker != "none" {
            for i in 0..n {
                if valid[i] {
                    let mut p = primitive(
                        vec![[xs[i], ys[i]]],
                        false,
                        if let Some(scale) = &layer.scale {
                            json!(scale_color(scale, *colors.get(i).unwrap_or(&f64::NAN)))
                        } else {
                            a.get("marker_fill").map(V::json).unwrap_or(json!(color))
                        },
                        &string(&a, "marker_border_color", "none"),
                        length_arg(e, &a, "marker_border_width", sw),
                        point_opacities[i],
                    );
                    p.marker = Some(marker.clone());
                    p.marker_size = marker_sizes[i];
                    p.index = i;
                    layer.primitives.push(p);
                }
            }
        }
    }
    if projection == "polar"
        && layer
            .primitives
            .iter()
            .flat_map(|p| &p.points)
            .any(|p| p[1] < 0.)
    {
        e.warn(
            "W_POLAR_NEGATIVE_RADIUS",
            "极坐标包含负半径坐标；已翻转半周并取绝对值",
            l,
        );
    }
    Ok(layer)
}
fn grid_layer(
    e: &mut Engine,
    layer: &mut Layer,
    projection: &str,
    plotargs: &Args,
    l: Loc,
    sw: f64,
    opacity: f64,
) -> Result<()> {
    let a = &layer.args;
    let mut z: Vec<Vec<f64>> = array(a, "z")
        .iter()
        .map(|r| {
            r.list()
                .iter()
                .map(|v| v.number().unwrap_or(f64::NAN))
                .collect()
        })
        .collect();
    if z.is_empty() || z[0].is_empty() || z.iter().any(|r| r.len() != z[0].len()) {
        return Err(e.error("E_PLOT", "需要非空矩形矩阵", l));
    }
    let heat = layer.kind == "heatmap";
    if !heat && (z.len() < 2 || z[0].len() < 2) {
        return Err(e.error("E_PLOT", "等高线需要至少 2×2 的矩阵", l));
    }
    let extent = nums(a, "extent");
    let extent = if extent.len() == 4 {
        extent
    } else {
        vec![
            0.,
            (z[0].len() - usize::from(!heat)) as f64,
            0.,
            (z.len() - usize::from(!heat)) as f64,
        ]
    };
    let mut xs = nums(a, if heat { "x_edges" } else { "x" });
    let mut ys = nums(a, if heat { "y_edges" } else { "y" });
    if xs.is_empty() {
        xs = linspace(extent[0], extent[1], z[0].len() + usize::from(heat));
    }
    if ys.is_empty() {
        ys = linspace(extent[2], extent[3], z.len() + usize::from(heat));
    }
    if xs.len() != z[0].len() + usize::from(heat)
        || ys.len() != z.len() + usize::from(heat)
        || xs.windows(2).chain(ys.windows(2)).any(|v| v[1] <= v[0])
    {
        return Err(e.error("E_PLOT", "网格坐标须递增并与矩阵匹配", l));
    }
    if projection == "polar" && !heat && yes(a, "periodic", false) {
        let period = if string(plotargs, "angle_unit", "deg") == "rad" {
            std::f64::consts::TAU
        } else {
            360.
        };
        xs.push(xs[0] + period);
        for row in &mut z {
            row.push(row[0]);
        }
    }
    layer.extent = Some([xs[0], *xs.last().unwrap(), ys[0], *ys.last().unwrap()]);
    let finite: Vec<_> = z
        .iter()
        .flatten()
        .copied()
        .filter(|v| v.is_finite())
        .collect();
    if finite.is_empty() {
        return Err(e.error("E_DATA", "没有有效网格数据", l));
    }
    if finite.len() < z.len() * z[0].len() {
        e.warn(
            "W_PLOT_MISSING",
            "网格包含缺失采样点；缺失单元及其相邻插值区域留空",
            l,
        );
    }
    if layer.scale.is_none() {
        let (mut lo, mut hi) = bounds(&finite).unwrap();
        if lo == hi {
            lo -= 0.5;
            hi += 0.5;
        }
        let mut s = Args::new();
        s.insert("vmin".into(), V::num(num(a, "vmin", lo)));
        s.insert("vmax".into(), V::num(num(a, "vmax", hi)));
        s.insert(
            "cmap".into(),
            a.get("cmap").cloned().unwrap_or(V::text("viridis")),
        );
        layer.scale = Some(s);
    }
    let scale = layer.scale.as_ref().unwrap();
    if string(scale, "norm", "linear") == "log" && finite.iter().any(|v| *v <= 0.) {
        return Err(e.error("E_PLOT", "对数颜色数据必须为正", l));
    }
    if heat {
        for (r, row) in z.iter().enumerate() {
            let rr = if string(a, "origin", "lower") == "upper" {
                z.len() - 1 - r
            } else {
                r
            };
            for (c, v) in row.iter().enumerate() {
                if v.is_finite() {
                    layer.primitives.push(primitive(
                        vec![
                            [xs[c], ys[rr]],
                            [xs[c + 1], ys[rr]],
                            [xs[c + 1], ys[rr + 1]],
                            [xs[c], ys[rr + 1]],
                        ],
                        true,
                        json!(scale_color(scale, *v)),
                        "none",
                        0.,
                        opacity,
                    ));
                }
            }
        }
    } else {
        let levels = vec_checked(e, a, "levels", l)?;
        if levels.windows(2).any(|v| v[1] <= v[0]) {
            return Err(e.error("E_PLOT", "levels 须严格递增", l));
        }
        // The legacy engine used D3's smoothed marching-square superlevel rings.
        // contour 0.13.1 ports that algorithm with f64 coordinates. Adjacent
        // superlevel sets form disjoint bands by even-odd subtraction.
        let floor = finite
            .iter()
            .copied()
            .chain(levels.iter().copied())
            .fold(f64::INFINITY, f64::min);
        let ceiling = finite
            .iter()
            .copied()
            .chain(levels.iter().copied())
            .fold(f64::NEG_INFINITY, f64::max);
        let missing_value = floor - (ceiling - floor).abs().max(1.);
        let values: Vec<f64> = z
            .iter()
            .flatten()
            .map(|v| if v.is_finite() { *v } else { missing_value })
            .collect();
        let polygons = contour::ContourBuilder::new(z[0].len(), z.len(), true)
            .contours(&values, &levels)
            .map_err(|err| e.error("E_PLOT", format!("等值区域计算失败：{err}"), l))?;
        let raw_rings: Vec<Vec<Vec<[f64; 2]>>> = polygons
            .iter()
            .map(|contour| {
                contour
                    .geometry()
                    .0
                    .iter()
                    .flat_map(|polygon| {
                        std::iter::once(polygon.exterior()).chain(polygon.interiors().iter())
                    })
                    .map(|ring| ring.0.iter().map(|c| [c.x, c.y]).collect())
                    .collect()
            })
            .collect();
        let coordinate = |coordinates: &[f64], v: f64| {
            let t = (v - 0.5).clamp(0., (coordinates.len() - 1) as f64);
            let i = (t.floor() as usize).min(coordinates.len() - 2);
            coordinates[i] + (coordinates[i + 1] - coordinates[i]) * (t - i as f64)
        };
        let map = |p: [f64; 2]| [coordinate(&xs, p[0]), coordinate(&ys, p[1])];
        for (i, rings) in raw_rings.iter().enumerate() {
            if layer.kind == "contourf" {
                let mut band: Vec<Vec<[f64; 2]>> = rings
                    .iter()
                    .chain(raw_rings.get(i + 1).into_iter().flatten())
                    .map(|ring| ring.iter().copied().map(map).collect())
                    .collect();
                if !band.is_empty() {
                    let mut p = primitive(
                        band.remove(0),
                        true,
                        json!(scale_color(scale, levels[i])),
                        "none",
                        0.,
                        opacity,
                    );
                    p.holes = band;
                    layer.primitives.push(p);
                }
            } else {
                let color = if a.contains_key("color_scale") {
                    scale_color(scale, levels[i])
                } else {
                    layer.color.clone()
                };
                for ring in rings {
                    for pair in ring.windows(2) {
                        let (u, v) = (pair[0], pair[1]);
                        if u[0] <= 0.5 && v[0] <= 0.5
                            || u[0] >= z[0].len() as f64 - 0.5 && v[0] >= z[0].len() as f64 - 0.5
                            || u[1] <= 0.5 && v[1] <= 0.5
                            || u[1] >= z.len() as f64 - 0.5 && v[1] >= z.len() as f64 - 0.5
                        {
                            continue;
                        }
                        layer.primitives.push(primitive(
                            vec![map(u), map(v)],
                            false,
                            json!("none"),
                            &color,
                            sw,
                            opacity,
                        ));
                    }
                }
            }
        }
        if finite.len() < z.len() * z[0].len() {
            let masks = grid_mask(&z, &xs, &ys);
            if masks.is_empty() {
                layer.primitives.clear();
            }
            layer.masks = Some(masks);
        }
    }
    Ok(())
}
/// Merge adjacent valid interpolation cells; every valid cell is covered once.
fn grid_mask(z: &[Vec<f64>], xs: &[f64], ys: &[f64]) -> Vec<[f64; 4]> {
    let mut output: Vec<[f64; 4]> = vec![];
    let mut active: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    for r in 0..z.len() - 1 {
        let mut next = BTreeMap::new();
        let mut c = 0;
        while c < z[0].len() - 1 {
            let valid = |c: usize| {
                [z[r][c], z[r][c + 1], z[r + 1][c], z[r + 1][c + 1]]
                    .iter()
                    .all(|v| v.is_finite())
            };
            if !valid(c) {
                c += 1;
                continue;
            }
            let start = c;
            while c < z[0].len() - 1 && valid(c) {
                c += 1;
            }
            let key = (start, c);
            let index = if let Some(&index) = active.get(&key) {
                output[index][3] = ys[r + 1];
                index
            } else {
                output.push([xs[start], xs[c], ys[r], ys[r + 1]]);
                output.len() - 1
            };
            next.insert(key, index);
        }
        active = next;
    }
    output
}
pub fn layer_values(layer: &Layer, dim: usize) -> Vec<f64> {
    if let Some(ext) = layer.extent {
        return vec![ext[dim * 2], ext[dim * 2 + 1]];
    }
    if layer.zero == Some(dim) {
        if let Some(values) = &layer.actual {
            return values.clone();
        }
    }

    layer
        .primitives
        .iter()
        .flat_map(|p| p.points.iter().map(move |q| q[dim]))
        .filter(|v| v.is_finite() && !(layer.zero == Some(dim) && *v == 0.))
        .collect()
}
