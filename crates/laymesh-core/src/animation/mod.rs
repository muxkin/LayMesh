//! Deterministic CSS animation effects, scheduled by Lay presentation steps.
//! Times are seconds and geometry is millimetres; CSS pixels use canvas layout_dpi.
pub mod css;
pub mod matrix;
pub mod values;
use crate::{Diagnostic, Loc, Result, engine::Engine, model::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use values::{Easing, Matrix, Transform};

#[derive(Clone, Debug)]
pub struct StepSpec {
    pub name: String,
    pub trigger: String,
    pub hold: f64,
    pub duration: Option<f64>,
    pub clips: Vec<ClipSpec>,
    pub file: String,
    pub loc: Loc,
}
#[derive(Clone, Debug)]
pub struct ClipSpec {
    pub target: usize,
    pub args: Args,
    pub file: String,
    pub loc: Loc,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Animation {
    pub normal: Timeline,
    pub reduced: Timeline,
    pub print: Timeline,
    pub print_reduced: Timeline,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Timeline {
    pub steps: Vec<Step>,
    pub duration: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    #[serde(default = "click_trigger")]
    pub trigger: String,
    pub start: f64,
    pub duration: f64,
    pub hold: f64,
    pub clips: Vec<Clip>,
}
fn click_trigger() -> String {
    "click".into()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub target: String,
    pub name: String,
    pub duration: f64,
    pub delay: f64,
    /// None represents infinite; never serialize non-finite JSON numbers.
    pub iterations: Option<f64>,
    pub direction: String,
    pub fill: String,
    pub paused: bool,
    pub origin: [f64; 3],
    pub backface: String,
    pub base_opacity: f64,
    pub tracks: Vec<Track>,
    pub important: BTreeMap<String, Value>,
    pub underlying: BTreeMap<String, Value>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub property: String,
    pub frames: Vec<Keyframe>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    pub offset: f64,
    pub value: Value,
    pub easing: Easing,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Number(f64),
    Transform(Vec<Transform>),
    Origin([f64; 3]),
    Keyword(String),
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FrameStyle {
    pub opacity: Option<f64>,
    pub transform: Option<Matrix>,
    pub origin: Option<[f64; 3]>,
    pub visible: Option<bool>,
    pub backface: Option<bool>,
    pub draw: Option<f64>,
    pub reveal: Option<f64>,
}
impl Animation {
    pub fn timeline_with_media(&self, reduced: bool, print: bool) -> &Timeline {
        if print {
            if reduced {
                &self.print_reduced
            } else {
                &self.print
            }
        } else {
            self.timeline(reduced)
        }
    }

    pub fn timeline(&self, reduced: bool) -> &Timeline {
        if reduced { &self.reduced } else { &self.normal }
    }
}
fn animation_error(message: impl Into<String>, file: &str, loc: Loc) -> Diagnostic {
    Diagnostic::new("E_ANIMATION", message, file, loc)
}
fn timing(v: &V, file: &str, loc: Loc) -> Result<f64> {
    match v.value() {
        V::Number(n, u) if n.is_finite() && matches!(u.as_str(), "s" | "ms") => {
            Ok(n * if u == "ms" { 0.001 } else { 1. })
        }
        _ => Err(animation_error(
            "Time requires a finite s/ms value",
            file,
            loc,
        )),
    }
}
impl Engine {
    pub(crate) fn animation_method(
        &mut self,
        owner: std::rc::Rc<std::cell::RefCell<Object>>,
        method: &str,
        pos: Vec<V>,
        args: Args,
        loc: Loc,
    ) -> Result<V> {
        if method == "step" {
            if owner.borrow().kind != "canvas" || !pos.is_empty() {
                return Err(self.error(
                    "E_ANIMATION",
                    "step() requires a canvas and named arguments",
                    loc,
                ));
            }
            for k in args.keys() {
                if !k.starts_with("__")
                    && !["name", "hold", "duration", "start"].contains(&k.as_str())
                {
                    return Err(self.error(
                        "E_ANIMATION",
                        format!("Unknown step parameter: {k}"),
                        loc,
                    ));
                }
            }
            let hold = args
                .get("hold")
                .map(|v| timing(v, &self.file, loc))
                .transpose()?
                .unwrap_or(1.);
            let duration = args
                .get("duration")
                .map(|v| timing(v, &self.file, loc))
                .transpose()?;
            if hold < 0. || duration.is_some_and(|v| v < 0.) {
                return Err(self.error(
                    "E_ANIMATION",
                    "Step duration/hold must be nonnegative",
                    loc,
                ));
            }
            let trigger = string(&args, "start", "click");
            if !["click", "after-previous"].contains(&trigger.as_str()) {
                return Err(self.error(
                    "E_ANIMATION",
                    "step start must be click or after-previous",
                    loc,
                ));
            }
            let index = self.animation_steps.len();
            self.animation_steps.push(StepSpec {
                name: string(&args, "name", &format!("Step {}", index + 1)),
                trigger,
                hold,
                duration,
                clips: vec![],
                file: self.file.clone(),
                loc,
            });
            return Ok(self.object(
                "step",
                [("__index".into(), V::num(index as f64))]
                    .into_iter()
                    .collect(),
                loc,
            ));
        }
        if owner.borrow().kind != "step" || pos.len() != 1 {
            return Err(self.error(
                "E_ANIMATION",
                "animate() requires a step and one placed instance",
                loc,
            ));
        }
        for k in args.keys() {
            if !k.starts_with("__")
                && ![
                    "name",
                    "class",
                    "duration",
                    "delay",
                    "easing",
                    "iterations",
                    "direction",
                    "fill",
                    "vars",
                    "play_state",
                    "start",
                ]
                .contains(&k.as_str())
            {
                return Err(self.error(
                    "E_ANIMATION",
                    format!("Unknown animate parameter: {k}"),
                    loc,
                ));
            }
        }
        let target = pos[0]
            .object()
            .filter(|o| o.borrow().kind == "instance")
            .ok_or_else(|| {
                self.error(
                    "E_ANIMATION",
                    "Animation target must be the result of add()",
                    loc,
                )
            })?
            .borrow()
            .id;
        let index = num(&owner.borrow().args, "__index", 0.) as usize;
        self.animation_steps[index].clips.push(ClipSpec {
            target,
            args,
            file: self.file.clone(),
            loc,
        });
        Ok(V::Null)
    }
    pub(crate) fn compile_animation(
        &mut self,
        nodes: &[serde_json::Value],
    ) -> Result<Option<Animation>> {
        if self.animation_steps.is_empty() {
            return Ok(None);
        }
        let mut diagnostics = vec![];
        let animation = Animation {
            normal: self.compile_timeline(nodes, false, false, &mut diagnostics)?,
            reduced: self.compile_timeline(nodes, true, false, &mut diagnostics)?,
            print: self.compile_timeline(nodes, false, true, &mut diagnostics)?,
            print_reduced: self.compile_timeline(nodes, true, true, &mut diagnostics)?,
        };
        for diagnostic in diagnostics {
            if !self.warnings.contains(&diagnostic) {
                self.warnings.push(diagnostic);
            }
        }
        Ok(Some(animation))
    }
    fn compile_timeline(
        &self,
        nodes: &[serde_json::Value],
        reduced: bool,
        print: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Timeline> {
        let mut timeline = Timeline::default();
        for spec in &self.animation_steps {
            let mut clips = vec![];
            let mut previous_start = 0.;
            let mut previous_end = Some(0.);
            for c in &spec.clips {
                let start = match string(&c.args, "start", "step").as_str() {
                    "step" => 0.,
                    "with-previous" => previous_start,
                    "after-previous" => previous_end.ok_or_else(|| animation_error(
                        "after-previous cannot follow an infinite or paused action; use a separate step with a finite duration",
                        &c.file, c.loc,
                    ))?,
                    _ => return Err(animation_error("animate start must be step, with-previous or after-previous", &c.file, c.loc)),
                };
                let mut action = self.compile_clip(c, nodes, reduced, print, diagnostics)?;
                for clip in &mut action {
                    clip.delay += start;
                    if !clip.delay.is_finite() {
                        return Err(animation_error("Animation delay overflow", &c.file, c.loc));
                    }
                }
                previous_start = action
                    .iter()
                    .map(|c| c.delay.max(start))
                    .reduce(f64::min)
                    .unwrap_or(start);
                previous_end = if action
                    .iter()
                    .any(|c| c.paused || (c.iterations.is_none() && c.duration > 0.))
                {
                    None
                } else {
                    Some(
                        action
                            .iter()
                            .map(|c| (c.delay + c.duration * c.iterations.unwrap_or(0.)).max(start))
                            .fold(start, f64::max),
                    )
                };
                clips.extend(action);
            }
            let unbounded = clips
                .iter()
                .any(|c| c.paused || (c.iterations.is_none() && c.duration > 0.));
            if unbounded && spec.duration.is_none() {
                return Err(animation_error(
                    "Infinite or paused animation requires step(duration=...)",
                    &spec.file,
                    spec.loc,
                ));
            }
            let duration = spec.duration.unwrap_or_else(|| {
                clips
                    .iter()
                    .map(|c| (c.delay + c.duration * c.iterations.unwrap_or(0.)).max(0.))
                    .fold(0., f64::max)
            });
            if !duration.is_finite() {
                return Err(animation_error(
                    "Animation duration overflow",
                    &spec.file,
                    spec.loc,
                ));
            }
            let step = Step {
                name: spec.name.clone(),
                trigger: spec.trigger.clone(),
                start: timeline.duration,
                duration,
                hold: spec.hold,
                clips,
            };
            timeline.duration += duration + spec.hold;
            timeline.steps.push(step);
        }
        Ok(timeline)
    }
    fn compile_clip(
        &self,
        spec: &ClipSpec,
        nodes: &[serde_json::Value],
        reduced: bool,
        print: bool,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<Vec<Clip>> {
        let fail = |m: String| animation_error(m, &spec.file, spec.loc);
        let instance = self
            .objects
            .get(&spec.target)
            .ok_or_else(|| fail("Unknown target instance".into()))?
            .borrow();
        let id = instance
            .node
            .as_ref()
            .and_then(|n| n["id"].as_str())
            .ok_or_else(|| fail("Missing target instance".into()))?;
        fn find<'a>(
            nodes: &'a [serde_json::Value],
            id: &str,
            out: &mut Vec<&'a serde_json::Value>,
        ) {
            for n in nodes {
                if n["id"] == id {
                    out.push(n);
                }
                if let Some(children) = n["children"].as_array() {
                    find(children, id, out);
                }
            }
        }
        let mut found = vec![];
        find(nodes, id, &mut found);
        if found.len() != 1 {
            return Err(fail("Target must identify exactly one placed instance on this page; animate the placed group when reusing a component".into()));
        }
        let node = found[0];
        let size = [jnum(node, "width", 0.), jnum(node, "height", 0.)];
        let base_opacity = jnum(node, "opacity", 1.);
        let mut chain = vec![];
        let mut style_chain = vec![];
        let mut current = Some(spec.target);
        let mut target_file = spec.file.clone();
        while let Some(i) = current {
            let Some(o) = self.objects.get(&i) else {
                break;
            };
            let o = o.borrow();
            let material = o.args.get("__material").and_then(V::object);
            let (kind, args, file) = if let Some(m) = material {
                let m = m.borrow();
                (m.kind.clone(), m.args.clone(), m.file.clone())
            } else {
                (o.kind.clone(), o.args.clone(), o.file.clone())
            };
            let mut styled_args = args.clone();
            if let Some(class) = o.args.get("class") {
                styled_args.insert("class".into(), class.clone());
            }
            styled_args.insert("__style_file".into(), V::text(&file));
            style_chain.push((kind.clone(), styled_args));
            if i == spec.target {
                target_file = file;
            }
            chain.push(css::Element {
                kind,
                class: o
                    .args
                    .get("class")
                    .map(V::as_str)
                    .unwrap_or_else(|| string(&args, "class", "")),
            });
            current = (o.parent != 0).then_some(o.parent);
        }
        chain.reverse();
        let mut computed = Args::new();
        for (kind, args) in style_chain.into_iter().rev() {
            computed = self.styled(&kind, &args, &computed)?;
        }
        let ctx = self.css_context(&computed);
        if let Some(c) = spec.args.get("class") {
            if let Some(last) = chain.last_mut() {
                last.class = format!("{} {}", last.class, c.as_str());
            }
        }
        let mut overrides = BTreeMap::new();
        if let Some(v) = spec.args.get("vars") {
            let entries = match v.value() {
                V::Dict(d) => d
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<Vec<_>>(),
                V::Map(m) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                _ => return Err(fail("vars requires a dictionary".into())),
            };
            for (k, v) in entries {
                if !k.starts_with("--") {
                    return Err(fail("CSS variable names start with --".into()));
                }
                let s = match v.value() {
                    V::Number(_, u) if matches!(u.as_str(), "mm" | "cm" | "in" | "pt" | "px") => {
                        format!(
                            "{}mm",
                            value_length(&v, &self.unit, self.dpi)
                                .ok_or_else(|| fail("Invalid variable length".into()))?
                        )
                    }
                    V::Number(n, u) => format!("{n}{u}"),
                    _ => v.as_str(),
                };
                overrides.insert(k, s);
            }
        }
        let (mut props, vars) = cascade(
            &self.animation_sheets,
            &chain,
            &target_file,
            self.css_entry_file.as_deref(),
            reduced,
            print,
            &overrides,
            ctx,
            &|query| self.css_supports(query, ctx),
        );
        props.retain(|property, d| {
            match css::substitute(&d.value, &vars).and_then(|value| {
                validate_motion(property, &value, size, ctx)?;
                Ok(value)
            }) {
                Ok(value) => {
                    d.value = value;
                    true
                }
                Err(message) => {
                    self.css_warning(Diagnostic::new(
                        "W_CSS",
                        format!("Ignored {property}: {message}"),
                        &d.file,
                        d.loc,
                    ));
                    false
                }
            }
        });
        // Valid shorthands were expanded during cascade; invalid CSS was discarded.
        props = expand_shorthand(props).map_err(fail)?;
        // An explicit `none` remains a valid no-op, including media overrides.
        // An animate() call without any resolved animation is usually a missing
        // stylesheet or mistyped class. Diagnose the .lay call once, on screen.
        if !reduced
            && !print
            && !spec.args.contains_key("name")
            && !props.contains_key("animation-name")
        {
            let classes = chain.last().map(|e| e.class.as_str()).unwrap_or("");
            let hint = if !props.contains_key("animation-duration")
                && classes
                    .split_whitespace()
                    .any(|c| c.starts_with("animate__"))
            {
                " Load Animate.css with stylesheet=\"@laymesh/animate.css\" (or add it to the stylesheet list)."
            } else {
                " Check the animation class and stylesheet, or specify name=... with duration=...."
            };
            diagnostics.push(Diagnostic::new(
                "W_ANIMATION",
                format!("animate() has no matching animation-name for class=\"{classes}\"; this action will not play.{hint}"),
                &spec.file,
                spec.loc,
            ));
        }
        if !reduced
            && !print
            && !spec.args.contains_key("duration")
            && !props.contains_key("animation-duration")
            && (spec
                .args
                .get("name")
                .map(V::as_str)
                .is_some_and(|name| name != "none")
                || props
                    .get("animation-name")
                    .is_some_and(|d| d.value != "none"))
        {
            let classes = chain.last().map(|e| e.class.as_str()).unwrap_or("");
            let hint = if classes
                .split_whitespace()
                .any(|c| c.starts_with("animate__"))
                && !classes.split_whitespace().any(|c| c == "animate__animated")
            {
                " Add animate__animated alongside the effect class, or set duration=... explicitly."
            } else {
                " Set animation-duration in LCSS or duration=... in animate()."
            };
            diagnostics.push(Diagnostic::new("W_ANIMATION", format!("animate() has an animation name but no duration; CSS defaults to 0s, so this action will not play.{hint}"), &spec.file, spec.loc));
        }
        let get = |k: &str, default: &str| {
            props
                .get(k)
                .map(|d| d.value.clone())
                .unwrap_or(default.into())
        };
        let names = spec
            .args
            .get("name")
            .map(|v| vec![v.as_str()])
            .unwrap_or_else(|| css::split(&get("animation-name", "none"), ','));
        let nth = |k: &str, default: &str, i: usize| {
            let list = css::split(&get(k, default), ',');
            list[i % list.len()].clone()
        };
        let mut out = vec![];
        for (i, name) in names.iter().enumerate() {
            if name == "none" {
                continue;
            }
            let name = name.trim_matches(['\'', '"']);
            let builtin = builtin(name);
            let mut definitions: Vec<_> = self
                .animation_sheets
                .iter()
                .filter(|s| {
                    crate::css_selectors::scope_applies(
                        s.scope.as_deref(),
                        &target_file,
                        self.css_entry_file.as_deref(),
                    )
                })
                .flat_map(|s| {
                    s.keyframes.iter().map(move |k| {
                        (
                            crate::css_selectors::theme_priority(
                                s.scope.as_deref(),
                                self.css_entry_file.as_deref(),
                            ),
                            k,
                        )
                    })
                })
                .filter(|(_, k)| k.name == name)
                .collect();
            definitions.sort_by_key(|(priority, _)| *priority);
            let keyframes = definitions.into_iter().map(|(_, k)| k)
                .fold(None, |previous: Option<&css::Keyframes>, k| {
                    if k.prefixed && previous.is_some_and(|p| !p.prefixed) {
                        previous
                    } else {
                        Some(k)
                    }
                })
                .cloned()
                .or(builtin)
                .ok_or_else(|| fail(format!("Unknown animation: {name}. No matching @keyframes definition was loaded; check the name and explicitly import its stylesheet (Animate.css: @laymesh/animate.css).")))?;
            let parse_time = |key: &str, csskey: &str| -> Result<f64> {
                if let Some(v) = spec.args.get(key) {
                    timing(v, &spec.file, spec.loc)
                } else {
                    values::time(&nth(csskey, "0s", i)).map_err(fail)
                }
            };
            let duration = parse_time("duration", "animation-duration")?;
            let delay = parse_time("delay", "animation-delay")?;
            if duration < 0. {
                return Err(fail("Animation duration must be nonnegative".into()));
            }
            let iterations = spec
                .args
                .get("iterations")
                .map(|v| v.as_str())
                .unwrap_or_else(|| nth("animation-iteration-count", "1", i));
            let iterations = if iterations == "infinite" {
                None
            } else {
                let n = values::number(&iterations).map_err(fail)?;
                if n < 0. {
                    return Err(fail("Iteration count must be nonnegative".into()));
                }
                Some(n)
            };
            let option = |key: &str, csskey: &str, default: &str| {
                spec.args
                    .get(key)
                    .map(V::as_str)
                    .unwrap_or_else(|| nth(csskey, default, i))
            };
            let direction = option("direction", "animation-direction", "normal");
            let fill = option("fill", "animation-fill-mode", "none");
            let play = option("play_state", "animation-play-state", "running");
            if !["normal", "reverse", "alternate", "alternate-reverse"]
                .contains(&direction.as_str())
                || !["none", "both", "forwards", "backwards"].contains(&fill.as_str())
                || !["running", "paused"].contains(&play.as_str())
            {
                return Err(fail("Invalid direction, fill, or play state".into()));
            }
            let easing = Easing::parse(&option("easing", "animation-timing-function", "ease"))
                .map_err(fail)?;
            let origin = values::origin_context(&get("transform-origin", "50% 50%"), size, ctx)
                .map_err(fail)?;
            let backface = get("backface-visibility", "visible");
            let mut per_property: BTreeMap<String, Vec<Keyframe>> = BTreeMap::new();
            let mut frames = keyframes.frames.clone();
            frames.sort_by(|a, b| a.0.total_cmp(&b.0));
            // A declaration retains the timing function of its own keyframe
            // rule. Chromium exposes duplicate offsets with different easings
            // separately; an easing-only rule does not restyle other rules.
            let mut endpoint_easing = easing.clone();
            for (at, declarations) in frames {
                let ds: BTreeMap<_, _> = declarations
                    .into_iter()
                    .map(|d| (d.property.clone(), d))
                    .collect();
                let e = ds
                    .get("animation-timing-function")
                    .and_then(|d| {
                        match css::substitute(&d.value, &vars).and_then(|s| Easing::parse(&s)) {
                            Ok(value) => Some(value),
                            Err(message) => {
                                self.css_warning(Diagnostic::new(
                                    "W_CSS",
                                    format!("Ignored keyframe easing: {message}"),
                                    &d.file,
                                    d.loc,
                                ));
                                None
                            }
                        }
                    })
                    .unwrap_or_else(|| easing.clone());
                if at == 0. {
                    endpoint_easing = e.clone();
                }
                for (property, d) in ds {
                    if property.starts_with("animation-") || property.starts_with("transition-") {
                        continue;
                    }
                    let value = match css::substitute(&d.value, &vars)
                        .and_then(|text| parse_value(&property, &text, size, ctx))
                    {
                        Ok(value) => value,
                        Err(message) => {
                            self.css_warning(Diagnostic::new(
                                "W_CSS",
                                format!("Ignored animated {property}: {message}"),
                                &d.file,
                                d.loc,
                            ));
                            continue;
                        }
                    };
                    let track = per_property.entry(property).or_default();
                    if track.last().is_some_and(|f| f.offset == at) {
                        track.pop();
                    }
                    track.push(Keyframe {
                        offset: at,
                        value,
                        easing: e.clone(),
                    });
                }
            }
            let mut tracks = vec![];
            let mut important = BTreeMap::new();
            for (property, mut frames) in per_property {
                let base = match props.get(&property) {
                    Some(d) => parse_value(&property, &d.value, size, ctx).map_err(fail)?,
                    None => base_value(&property, base_opacity, origin),
                };
                if frames.first().is_none_or(|f| f.offset > 0.) {
                    frames.insert(
                        0,
                        Keyframe {
                            offset: 0.,
                            value: base.clone(),
                            easing: endpoint_easing.clone(),
                        },
                    );
                }
                if frames.last().is_none_or(|f| f.offset < 1.) {
                    frames.push(Keyframe {
                        offset: 1.,
                        value: base.clone(),
                        easing: easing.clone(),
                    });
                }
                if props.get(&property).is_some_and(|d| d.important) {
                    important.insert(property.clone(), base);
                }
                tracks.push(Track { property, frames });
            }
            let mut underlying = BTreeMap::new();
            for property in ["opacity", "transform", "visibility", "backface-visibility"] {
                if let Some(d) = props.get(property) {
                    underlying.insert(
                        property.into(),
                        parse_value(property, &d.value, size, ctx).map_err(fail)?,
                    );
                }
            }
            out.push(Clip {
                target: id.into(),
                name: name.into(),
                duration,
                delay,
                iterations,
                direction,
                fill,
                paused: play == "paused",
                origin,
                backface,
                base_opacity,
                tracks,
                important,
                underlying,
            });
        }
        Ok(out)
    }
}
fn validate_motion(
    property: &str,
    s: &str,
    size: [f64; 2],
    ctx: values::Context,
) -> std::result::Result<(), String> {
    for value in css::split(s, ',') {
        let valid = match property {
            "animation" => shorthand(&value).map(|_| true)?,
            "animation-duration" => values::time(&value)? >= 0.,
            "animation-delay" => {
                values::time(&value)?;
                true
            }
            "animation-iteration-count" => value == "infinite" || values::number(&value)? >= 0.,
            "animation-timing-function" => {
                Easing::parse(&value)?;
                true
            }
            "animation-direction" => matches!(
                value.as_str(),
                "normal" | "reverse" | "alternate" | "alternate-reverse"
            ),
            "animation-fill-mode" => {
                matches!(value.as_str(), "none" | "forwards" | "backwards" | "both")
            }
            "animation-play-state" => matches!(value.as_str(), "running" | "paused"),
            _ => true,
        };
        if !valid {
            return Err("Invalid CSS animation value".into());
        }
    }
    if matches!(
        property,
        "transform" | "transform-origin" | "opacity" | "visibility" | "backface-visibility"
    ) {
        parse_value(property, s, size, ctx)?;
    }
    Ok(())
}

fn cascade(
    sheets: &[css::Sheet],
    chain: &[css::Element],
    file: &str,
    entry: Option<&str>,
    reduced: bool,
    print: bool,
    overrides: &BTreeMap<String, String>,
    ctx: values::Context,
    supports: &dyn Fn(&str) -> bool,
) -> (BTreeMap<String, css::Declaration>, BTreeMap<String, String>) {
    let mut vars = BTreeMap::new();
    let mut target = BTreeMap::new();
    for end in 1..=chain.len() {
        let mut candidates = vec![];
        let mut order = 0;
        for sheet in sheets {
            if !crate::css_selectors::scope_applies(sheet.scope.as_deref(), file, entry) {
                continue;
            }
            for rule in &sheet.rules {
                if crate::css_values::media(&rule.media, ctx, reduced, print) == Some(true)
                    && supports(&rule.supports)
                    && css::matches(&rule.selector, &chain[..end])
                {
                    for d in &rule.declarations {
                        candidates.push((
                            (
                                d.important,
                                crate::css_selectors::theme_priority(sheet.scope.as_deref(), entry),
                                css::specificity(&rule.selector),
                                order,
                            ),
                            d.clone(),
                        ));
                        order += 1;
                    }
                }
            }
        }
        candidates.sort_by_key(|c| c.0);
        for (_, d) in &candidates {
            if d.property.starts_with("--") {
                vars.insert(d.property.clone(), d.value.clone());
            }
        }
        if end == chain.len() {
            vars.extend(overrides.clone());
        }
        target.clear();
        for (_, d) in candidates {
            if d.property == "animation" {
                let resolved = css::substitute(&d.value, &vars).unwrap_or_else(|_| d.value.clone());
                if let Ok(ps) = shorthand(&resolved) {
                    for (k, v) in ps {
                        target.insert(
                            k.clone(),
                            css::Declaration {
                                property: k,
                                value: v,
                                ..d.clone()
                            },
                        );
                    }
                } else {
                    target.insert(d.property.clone(), d);
                }
            } else {
                target.insert(d.property.clone(), d);
            }
        }
    }
    (target, vars)
}
fn expand_shorthand(
    mut props: BTreeMap<String, css::Declaration>,
) -> std::result::Result<BTreeMap<String, css::Declaration>, String> {
    if let Some(d) = props.remove("animation") {
        for (k, v) in shorthand(&d.value)? {
            props.entry(k.clone()).or_insert_with(|| css::Declaration {
                property: k,
                value: v,
                ..d.clone()
            });
        }
    }
    Ok(props)
}
fn shorthand(s: &str) -> std::result::Result<BTreeMap<String, String>, String> {
    let keys = [
        "animation-name",
        "animation-duration",
        "animation-timing-function",
        "animation-delay",
        "animation-iteration-count",
        "animation-direction",
        "animation-fill-mode",
        "animation-play-state",
    ];
    let mut columns: Vec<Vec<String>> = vec![vec![]; 8];
    for animation in css::split(s, ',') {
        let mut values =
            ["none", "0s", "ease", "0s", "1", "normal", "none", "running"].map(str::to_string);
        let mut seen = [false; 8];
        for token in values::words(&animation) {
            let index = if values::time(&token).is_ok() {
                if !seen[1] { 1 } else { 3 }
            } else if Easing::parse(&token).is_ok() {
                2
            } else if token == "infinite" || values::number(&token).is_ok() {
                4
            } else if ["normal", "reverse", "alternate", "alternate-reverse"]
                .contains(&token.as_str())
            {
                5
            } else if ["none", "forwards", "backwards", "both"].contains(&token.as_str())
                && !seen[6]
            {
                6
            } else if ["running", "paused"].contains(&token.as_str()) {
                7
            } else {
                0
            };
            if seen[index] {
                return Err("Duplicate animation shorthand component".into());
            }
            values[index] = token;
            seen[index] = true;
        }
        for (i, v) in values.into_iter().enumerate() {
            columns[i].push(v);
        }
    }
    Ok(keys
        .into_iter()
        .zip(columns)
        .map(|(k, v)| (k.into(), v.join(", ")))
        .collect())
}
fn parse_value(
    property: &str,
    s: &str,
    size: [f64; 2],
    ctx: values::Context,
) -> std::result::Result<Value, String> {
    Ok(match property {
        "opacity" => Value::Number(values::number_percentage(s, 1.)?.clamp(0., 1.)),
        "-laymesh-draw" | "-laymesh-reveal" => Value::Number(values::number(s)?),
        "transform" => Value::Transform(values::transform_context(s, size, ctx)?),
        "transform-origin" => Value::Origin(values::origin_context(s, size, ctx)?),
        "visibility" if ["visible", "hidden", "collapse"].contains(&s) => Value::Keyword(s.into()),
        "backface-visibility" if ["visible", "hidden"].contains(&s) => Value::Keyword(s.into()),
        _ => return Err(format!("Unsupported animated property: {property}")),
    })
}
fn base_value(property: &str, opacity: f64, origin: [f64; 3]) -> Value {
    match property {
        "opacity" => Value::Number(opacity),
        "transform" => Value::Transform(vec![]),
        "transform-origin" => Value::Origin(origin),
        "visibility" | "backface-visibility" => Value::Keyword("visible".into()),
        _ => Value::Number(1.),
    }
}
fn builtin(name: &str) -> Option<css::Keyframes> {
    let (property, from, to) = match name {
        "draw" | "laymesh-draw" => ("-laymesh-draw", "0", "1"),
        "reveal" | "laymesh-reveal" => ("-laymesh-reveal", "0", "1"),
        _ => return None,
    };
    Some(css::Keyframes {
        name: name.into(),
        frames: vec![
            (
                0.,
                vec![css::Declaration {
                    property: property.into(),
                    value: from.into(),
                    important: false,
                    file: "<builtin>".into(),
                    loc: Loc::default(),
                }],
            ),
            (
                1.,
                vec![css::Declaration {
                    property: property.into(),
                    value: to.into(),
                    important: false,
                    file: "<builtin>".into(),
                    loc: Loc::default(),
                }],
            ),
        ],
        prefixed: false,
        file: "<builtin>".into(),
        loc: Loc::default(),
    })
}
impl Clip {
    pub fn progress(&self, local: f64) -> Option<(f64, bool)> {
        let local = if self.paused { 0. } else { local };
        let t = local - self.delay;
        let count = self.iterations.unwrap_or(f64::INFINITY);
        let active = if self.duration == 0. {
            0.
        } else {
            self.duration * count
        };
        let before = t < 0.;
        let after = t >= active;
        if before && !matches!(self.fill.as_str(), "backwards" | "both") {
            return None;
        }
        if after && !matches!(self.fill.as_str(), "forwards" | "both") {
            return None;
        }
        let overall = if before {
            0.
        } else if after {
            if count.is_finite() { count } else { 0. }
        } else {
            t / self.duration
        };
        let mut iteration = overall.floor();
        let mut p = overall - iteration;
        if after && p == 0. && overall > 0. {
            iteration -= 1.;
            p = 1.;
        }
        let reverse = match self.direction.as_str() {
            "reverse" => true,
            "alternate" => (iteration as u64) % 2 == 1,
            "alternate-reverse" => (iteration as u64) % 2 == 0,
            _ => false,
        };
        if reverse {
            p = 1. - p;
        }
        Some((p, before))
    }
    fn apply(&self, p: f64, before: bool, style: &mut FrameStyle) {
        for (k, v) in &self.underlying {
            let missing = match k.as_str() {
                "opacity" => style.opacity.is_none(),
                "transform" => style.transform.is_none(),
                "visibility" => style.visible.is_none(),
                "backface-visibility" => style.backface.is_none(),
                _ => false,
            };
            if missing {
                apply_value(k, v, v, 0., style);
            }
        }
        style.origin = Some(self.origin);
        style.backface = Some(self.backface != "hidden");
        for track in &self.tracks {
            let i = track
                .frames
                .partition_point(|f| f.offset <= p)
                .saturating_sub(1)
                .min(track.frames.len() - 2);
            let (a, b) = (&track.frames[i], &track.frames[i + 1]);
            let t = if b.offset == a.offset {
                1.
            } else {
                (p - a.offset) / (b.offset - a.offset)
            };
            let t = if before { t } else { a.easing.at(t, false) };
            apply_value(&track.property, &a.value, &b.value, t, style);
        }
        for (k, v) in &self.important {
            apply_value(k, v, v, 0., style);
        }
    }
}
fn apply_value(property: &str, a: &Value, b: &Value, p: f64, style: &mut FrameStyle) {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            let n = a + (b - a) * p;
            match property {
                "opacity" => style.opacity = Some(n.clamp(0., 1.)),
                "-laymesh-draw" => style.draw = Some(n.clamp(0., 1.)),
                "-laymesh-reveal" => style.reveal = Some(n.clamp(0., 1.)),
                _ => {}
            }
        }
        (Value::Transform(a), Value::Transform(b)) => {
            style.transform = Some(values::interpolate(a, b, p))
        }
        (Value::Origin(a), Value::Origin(b)) => {
            style.origin = Some(std::array::from_fn(|i| a[i] + (b[i] - a[i]) * p))
        }
        (Value::Keyword(a), Value::Keyword(b)) => {
            let value = if property == "visibility"
                && (a == "visible" || b == "visible")
                && p > 0.
                && p < 1.
            {
                "visible"
            } else if p < 0.5 {
                a
            } else {
                b
            };
            if property == "visibility" {
                style.visible = Some(value == "visible");
            } else {
                style.backface = Some(value != "hidden");
            }
        }
        _ => {}
    }
}
impl Timeline {
    pub fn step_end(&self, number: usize) -> Result<f64> {
        self.steps
            .get(number.wrapping_sub(1))
            .map(|s| s.start + s.duration)
            .ok_or_else(|| animation_error("Step number is out of range", "", Loc::default()))
    }
    pub fn sample(&self, time: f64) -> BTreeMap<String, FrameStyle> {
        let mut out = BTreeMap::new();
        // Presentation pre-roll uses only the first clip for each property.
        let mut initialized: std::collections::BTreeSet<(String, String)> = Default::default();
        for step in &self.steps {
            for clip in &step.clips {
                let mut first = FrameStyle::default();
                clip.apply(
                    if matches!(clip.direction.as_str(), "reverse" | "alternate-reverse") {
                        1.
                    } else {
                        0.
                    },
                    true,
                    &mut first,
                );
                let style = out
                    .entry(clip.target.clone())
                    .or_insert_with(FrameStyle::default);
                for track in &clip.tracks {
                    if initialized.insert((clip.target.clone(), track.property.clone()))
                        && time < step.start
                        && matches!(clip.fill.as_str(), "both" | "backwards")
                    {
                        let v =
                            if matches!(clip.direction.as_str(), "reverse" | "alternate-reverse") {
                                &track.frames.last().unwrap().value
                            } else {
                                &track.frames[0].value
                            };
                        apply_value(&track.property, v, v, 0., style);
                        style.origin = first.origin;
                        style.backface = first.backface;
                    }
                }
            }
        }
        for step in &self.steps {
            if time < step.start {
                break;
            }
            let local = (time - step.start).min(step.duration);
            for clip in &step.clips {
                if let Some((p, before)) = clip.progress(local) {
                    clip.apply(p, before, out.entry(clip.target.clone()).or_default());
                }
            }
        }
        out
    }
}
