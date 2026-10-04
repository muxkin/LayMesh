use super::*;
#[derive(Clone, Debug)]
pub struct Segment {
    pub domain: [f64; 2],
    pub range: [f64; 2],
}
#[derive(Clone, Debug)]
pub struct Axis {
    pub args: Args,
    pub name: String,
    pub side: String,
    pub offset: f64,
    pub scale: String,
    pub constant: f64,
    pub reverse: bool,
    pub domain: [f64; 2],
    pub segments: Vec<Segment>,
}
pub fn transform(v: f64, scale: &str, c: f64) -> f64 {
    match scale {
        "log" => v.ln(),
        "symlog" => v.signum() * (v.abs() / c).ln_1p(),
        _ => v,
    }
}
pub fn inverse(v: f64, scale: &str, c: f64) -> f64 {
    match scale {
        "log" => v.exp(),
        "symlog" => v.signum() * v.abs().exp_m1() * c,
        _ => v,
    }
}
pub fn interpolate(v: f64, d: [f64; 2], r: [f64; 2], scale: &str, c: f64) -> f64 {
    let lo = transform(d[0], scale, c);
    let hi = transform(d[1], scale, c);
    r[0] + (transform(v, scale, c) - lo) / (hi - lo) * (r[1] - r[0])
}
impl Axis {
    pub fn minor_ticks(&self, major: &[f64]) -> Vec<f64> {
        if string(&self.args, "minor_ticks", "") != "auto" {
            return nums(&self.args, "minor_ticks")
                .into_iter()
                .filter(|v| !major.contains(v))
                .collect();
        }
        let mut result = vec![];
        if self.scale == "log" {
            for exp in
                (self.domain[0].log10().floor() as i32)..=(self.domain[1].log10().ceil() as i32)
            {
                for i in 2..10 {
                    let v = i as f64 * 10f64.powi(exp);
                    if v > self.domain[0] && v < self.domain[1] {
                        result.push(v);
                    }
                }
            }
        } else {
            let mut stops = vec![self.domain[0]];
            stops.extend(
                major
                    .iter()
                    .copied()
                    .filter(|v| *v > self.domain[0] && *v < self.domain[1]),
            );
            stops.push(self.domain[1]);
            stops.sort_by(f64::total_cmp);
            for q in stops.windows(2) {
                for i in 1..5 {
                    result.push(q[0] + (q[1] - q[0]) * i as f64 / 5.);
                }
            }
        }
        result.retain(|v| !major.iter().any(|m| (*m - *v).abs() < 1e-12));
        result
    }
    pub fn new(
        e: &Engine,
        a: Args,
        name: &str,
        side: &str,
        offset: f64,
        values: &[f64],
        l: Loc,
    ) -> Result<Self> {
        let scale = string(&a, "scale", "linear");
        if !matches!(scale.as_str(), "linear" | "log" | "symlog") {
            return Err(e.error("E_PLOT", "未知轴尺度", l));
        }
        let constant = num(&a, "constant", 1.);
        if constant <= 0. {
            return Err(e.error("E_PLOT", "constant 必须大于零", l));
        }
        let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
        if scale == "log" && finite.iter().any(|v| *v <= 0.) {
            return Err(e.error("E_PLOT", "对数轴数据必须大于零", l));
        }
        let raw = nums(&a, "range");
        let domain = if raw.len() == 2 {
            [raw[0], raw[1]]
        } else {
            let (mut lo, mut hi) = bounds(&finite).unwrap_or((0., 1.));
            if scale == "log" {
                if lo <= 0. {
                    lo = 1.;
                    hi = 10.;
                }
                let pad = if lo == hi {
                    0.5
                } else {
                    (hi.log10() - lo.log10()) * 0.05
                };
                lo = 10f64.powf(lo.log10() - pad);
                hi = 10f64.powf(hi.log10() + pad);
            } else {
                let pad = if lo == hi {
                    (lo.abs() * 0.05).max(0.5)
                } else if yes(&a, "_no_pad", false) {
                    0.
                } else {
                    (hi - lo) * 0.05
                };
                lo -= pad;
                hi += pad;
            }
            [lo, hi]
        };
        if domain[0] >= domain[1]
            || !domain.iter().all(|v| v.is_finite())
            || scale == "log" && domain[0] <= 0.
        {
            return Err(e.error("E_PLOT", "range 须为递增有限范围，对数范围须为正", l));
        }
        let breaks = array(&a, "breaks");
        if !breaks.is_empty() && raw.len() != 2 {
            return Err(e.error("E_PLOT", "断轴需要显式 range", l));
        }
        let mut last = domain[0];
        for b in breaks {
            let v = b.list();
            if v.len() != 2 {
                return Err(e.error("E_PLOT", "breaks 需要两个边界", l));
            }
            let (lo, hi) = (
                v[0].number().unwrap_or(f64::NAN),
                v[1].number().unwrap_or(f64::NAN),
            );
            if !(lo > last && hi > lo && hi < domain[1]) {
                return Err(e.error("E_PLOT", "breaks 须递增、不重叠并严格位于 range 内", l));
            }
            last = hi;
        }
        if a.contains_key("tick_text") && array(&a, "tick_text").len() != array(&a, "ticks").len() {
            return Err(e.error("E_PLOT", "tick_text 需要等长显式 ticks", l));
        }
        for key in ["ticks", "minor_ticks"] {
            for v in nums(&a, key) {
                if !v.is_finite()
                    || ((!values.is_empty() || raw.len() == 2) && (v < domain[0] || v > domain[1]))
                {
                    return Err(diagnostic_origin(
                        e.error("E_PLOT", "显式刻度必须位于轴范围内", l),
                        a.get(if key == "minor_ticks" {
                            "__placement_origin"
                        } else {
                            "__call_origin"
                        }),
                    ));
                }
            }
        }
        Ok(Self {
            reverse: yes(&a, "reverse", false),
            args: a,
            name: name.into(),
            side: side.into(),
            offset,
            scale,
            constant,
            domain,
            segments: vec![],
        })
    }
    pub fn map_segments(&mut self, e: &Engine, length: f64, l: Loc) -> Result<()> {
        let mut domains = vec![];
        let mut start = self.domain[0];
        for b in array(&self.args, "breaks") {
            let b = b.list();
            let (lo, hi) = (b[0].number().unwrap(), b[1].number().unwrap());
            domains.push([start, lo]);
            start = hi;
        }
        domains.push([start, self.domain[1]]);
        let gaps = if matches!(self.args.get("break_gap"), Some(V::List(_))) {
            array(&self.args, "break_gap")
                .iter()
                .map(|v| value_length(v, &e.unit, e.dpi).unwrap_or(f64::NAN))
                .collect::<Vec<_>>()
        } else {
            vec![length_arg(e, &self.args, "break_gap", 2.); domains.len() - 1]
        };
        if gaps.len() != domains.len() - 1 || gaps.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return Err(e.error("E_PLOT", "break_gap 需要每个断口一个正长度", l));
        }
        let free = length - gaps.iter().sum::<f64>();
        if free <= 0. {
            return Err(e.error("E_LAYOUT", "断口间距超过绘图区边长", l));
        }
        let spans: Vec<_> = domains
            .iter()
            .map(|d| {
                transform(d[1], &self.scale, self.constant)
                    - transform(d[0], &self.scale, self.constant)
            })
            .collect();
        let total: f64 = spans.iter().sum();
        let lengths = if self.args.contains_key("segment_lengths") {
            array(&self.args, "segment_lengths")
                .iter()
                .map(|v| value_length(v, &e.unit, e.dpi).unwrap_or(f64::NAN))
                .collect()
        } else {
            spans.iter().map(|v| free * v / total).collect::<Vec<_>>()
        };
        if lengths.len() != domains.len()
            || lengths.iter().any(|v| !v.is_finite() || *v <= 0.)
            || (lengths.iter().sum::<f64>() - free).abs() > 1e-6
        {
            return Err(e.error(
                "E_LAYOUT",
                "segment_lengths 与断口间距之和须等于绘图区边长",
                l,
            ));
        }
        let horizontal = self.horizontal();
        let mut cursor = if horizontal ^ self.reverse {
            0.
        } else {
            length
        };
        let direction = if horizontal ^ self.reverse { 1. } else { -1. };
        self.segments.clear();
        for (i, d) in domains.into_iter().enumerate() {
            let end = cursor + direction * lengths[i];
            self.segments.push(Segment {
                domain: d,
                range: [cursor, end],
            });
            cursor = end + direction * gaps.get(i).copied().unwrap_or(0.);
        }
        Ok(())
    }
    pub fn horizontal(&self) -> bool {
        matches!(self.side.as_str(), "top" | "bottom")
    }
    pub fn map(&self, v: f64) -> Option<f64> {
        self.segments
            .iter()
            .find(|s| v >= s.domain[0] && v <= s.domain[1])
            .map(|s| self.map_in(v, s))
    }
    pub fn map_in(&self, v: f64, s: &Segment) -> f64 {
        interpolate(v, s.domain, s.range, &self.scale, self.constant)
    }
    pub fn json(&self) -> Json {
        json!({"scale":self.scale,"domain":self.domain,"reverse":self.reverse,"constant":self.constant,"side":self.side,"offset":self.offset,"segments":self.segments.iter().map(|s|json!({"domain":s.domain,"range":s.range})).collect::<Vec<_>>()})
    }
    pub fn ticks(&self) -> Vec<f64> {
        if self.args.contains_key("ticks") {
            return nums(&self.args, "ticks");
        }
        if self.segments.len() > 1 {
            let length = self
                .segments
                .iter()
                .flat_map(|s| s.range)
                .fold(f64::NEG_INFINITY, f64::max)
                - self
                    .segments
                    .iter()
                    .flat_map(|s| s.range)
                    .fold(f64::INFINITY, f64::min);
            let mut out = vec![];
            for segment in &self.segments {
                let count = (5. * (segment.range[1] - segment.range[0]).abs() / length)
                    .round()
                    .max(2.) as usize;
                for value in scale_ticks(segment.domain, count, &self.scale) {
                    if !out.contains(&value) {
                        out.push(value);
                    }
                }
            }
            return out;
        }
        scale_ticks(
            self.domain,
            num(&self.args, "_tick_count", 5.) as usize,
            &self.scale,
        )
    }
}
pub fn scale_ticks(d: [f64; 2], count: usize, scale: &str) -> Vec<f64> {
    if scale != "log" {
        return ticks(d, count);
    }
    let lo = d[0].log10();
    let hi = d[1].log10();
    if hi - lo < count as f64 {
        let mut out = vec![];
        for exponent in lo.floor() as i32..=hi.ceil() as i32 {
            for k in 1..=9 {
                let value = k as f64 * 10f64.powi(exponent);
                if value >= d[0] && value <= d[1] {
                    out.push(value);
                }
            }
        }
        if out.len() * 2 < count {
            ticks(d, count)
        } else {
            out
        }
    } else {
        ticks([lo, hi], count.min((hi - lo).ceil() as usize))
            .into_iter()
            .map(|v| 10f64.powf(v))
            .collect()
    }
}
pub fn ticks(d: [f64; 2], count: usize) -> Vec<f64> {
    let step = (d[1] - d[0]) / count.max(1) as f64;
    let p = 10f64.powf(step.log10().floor());
    let e = step / p;
    let k = if e >= 50f64.sqrt() {
        10.
    } else if e >= 10f64.sqrt() {
        5.
    } else if e >= 2f64.sqrt() {
        2.
    } else {
        1.
    };
    let s = k * p;
    if !s.is_finite() || s <= 0. {
        return vec![];
    }
    let lo = (d[0] / s - 1e-10).ceil() as i64;
    let hi = (d[1] / s + 1e-10).floor() as i64;
    (lo..=hi.min(lo + 10000))
        .map(|v| {
            if s < 1. {
                v as f64 / (1. / s)
            } else {
                v as f64 * s
            }
        })
        .collect()
}
pub fn map_json(a: &Json, v: f64) -> Option<f64> {
    let scale = jstr(a, "scale", "linear");
    let c = jnum(a, "constant", 1.);
    for s in a["segments"].as_array()? {
        let d = [s["domain"][0].as_f64()?, s["domain"][1].as_f64()?];
        if v >= d[0] && v <= d[1] {
            return Some(interpolate(
                v,
                d,
                [s["range"][0].as_f64()?, s["range"][1].as_f64()?],
                scale,
                c,
            ));
        }
    }
    None
}
