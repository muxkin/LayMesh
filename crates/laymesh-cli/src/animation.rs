//! Streaming presentation export; a page is compiled once and frames are never retained.
use laymesh_core::{
    engine::compile_source,
    model::{Host, Scene},
};
use laymesh_render::{animation::AnimationRenderer, config};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
static CANCELLED: AtomicBool = AtomicBool::new(false);
pub fn install_cancel() {
    let _ = ctrlc::set_handler(|| CANCELLED.store(true, Ordering::Relaxed));
}
pub fn cancelled() -> bool {
    CANCELLED.load(Ordering::Relaxed)
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    pub width: u32,
    pub fps: f64,
    pub reduced_motion: bool,
    pub media: String,
    pub crf: u8,
    pub preset: String,
    pub background: String,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            width: 1920,
            fps: 30.,
            reduced_motion: false,
            media: "screen".into(),
            crf: 18,
            preset: "medium".into(),
            background: "#ffffff".into(),
        }
    }
}
fn options(format: &str, defaults: &Value, explicit: &Value) -> Result<Options> {
    let mut v = serde_json::to_value(Options::default())?;
    if format == "gif" {
        v["width"] = json!(960);
        v["fps"] = json!(20.);
    }
    if !defaults["export"]["animation"].is_null() {
        config::merge(&mut v, &defaults["export"]["animation"]);
    }
    if !defaults["export"]["animation_formats"][format].is_null() {
        config::merge(&mut v, &defaults["export"]["animation_formats"][format]);
    }
    if explicit.is_object() {
        config::merge(&mut v, explicit);
    }
    let o: Options = serde_json::from_value(v)?;
    if !(2..=8192).contains(&o.width)
        || !o.fps.is_finite()
        || o.fps <= 0.
        || o.fps > 120.
        || !["screen", "print"].contains(&o.media.as_str())
        || o.crf > 51
        || ![
            "ultrafast",
            "superfast",
            "veryfast",
            "faster",
            "fast",
            "medium",
            "slow",
            "slower",
            "veryslow",
        ]
        .contains(&o.preset.as_str())
    {
        return Err("Invalid animation encoding options".into());
    }
    background(&o.background)?;
    Ok(o)
}
fn background(s: &str) -> Result<[u8; 3]> {
    let s = s
        .strip_prefix('#')
        .ok_or("Animation background requires #RRGGBB")?;
    if s.len() != 6 || !s.is_ascii() {
        return Err("Animation background requires #RRGGBB".into());
    }
    Ok([
        u8::from_str_radix(&s[..2], 16)?,
        u8::from_str_radix(&s[2..4], 16)?,
        u8::from_str_radix(&s[4..], 16)?,
    ])
}
struct Staging {
    path: PathBuf,
    child: Option<Child>,
}
impl Drop for Staging {
    fn drop(&mut self) {
        if let Some(c) = &mut self.child {
            let _ = c.kill();
            let _ = c.wait();
        }
        let _ = std::fs::remove_file(&self.path);
    }
}
fn frame(
    scene: &Scene,
    o: &Options,
    w: u32,
    h: u32,
    time: Option<f64>,
    renderer: &mut AnimationRenderer,
) -> Result<Vec<u8>> {
    let ratio = w as f64 / h as f64;
    let (vw, vh) = if scene.width / scene.height > ratio {
        (scene.width, scene.width / ratio)
    } else {
        (scene.height * ratio, scene.height)
    };
    let x = (scene.width - vw) / 2.;
    let y = (scene.height - vh) / 2.;
    let r = renderer.render_environment(
        kurbo::Rect::new(x, y, x + vw, y + vh),
        w,
        h,
        time,
        o.reduced_motion,
        o.media == "print",
    )?;
    Ok(r.rgba)
}
/// Overlay-aware entrypoint used by both CLI and the editor's dedicated export worker.
pub fn export(
    file: &str,
    source: &str,
    mut host: Host,
    output: &Path,
    defaults: &Value,
    explicit: &Value,
    page: Option<usize>,
    cancel: Option<&Path>,
) -> Result<Value> {
    let format = output
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["mp4", "gif"].contains(&format.as_str()) {
        return Err("Animation output requires .mp4 or .gif".into());
    }
    let o = options(&format, defaults, explicit)?;
    let bg = background(&o.background)?;
    let encoder = super::ffmpeg::require(defaults, Path::new(file), &format)?;
    host.media_ffmpeg = Some(encoder.to_string_lossy().into_owned());
    let plan = if file.ends_with(".laydoc") {
        Some(laymesh_core::document::plan_source(
            source,
            file,
            host.clone(),
        )?)
    } else {
        None
    };
    let pages: Vec<usize> = if let Some(p) = page {
        vec![p]
    } else {
        (1..=plan.as_ref().map_or(1, |p| p.pages.len())).collect()
    };
    let compile = |number: usize| -> Result<Scene> {
        if let Some(p) = &plan {
            Ok(laymesh_core::document::compile_page(
                p.page(number)?,
                host.clone(),
            )?)
        } else if number == 1 {
            Ok(compile_source(source, file, host.clone())?)
        } else {
            Err("Page number is out of range".into())
        }
    };
    let first = compile(pages[0])?;
    let w = if format == "mp4" {
        o.width.div_ceil(2) * 2
    } else {
        o.width
    };
    let raw_h = (w as f64 * first.height / first.width).round().max(2.) as u32;
    let h = if format == "mp4" {
        raw_h.div_ceil(2) * 2
    } else {
        raw_h
    };
    if h > 8192 || w as u64 * h as u64 > 32_000_000 {
        return Err("Animation frame exceeds pixel limit".into());
    }
    let mut media_session = super::media_export::Session::new()?;
    for (i, number) in pages.iter().enumerate() {
        if CANCELLED.load(Ordering::Relaxed) || cancel.is_some_and(Path::exists) {
            return Err("Animation export cancelled".into());
        }
        if i == 0 {
            media_session.add(&first, o.reduced_motion, o.media == "print")?;
        } else {
            media_session.add(&compile(*number)?, o.reduced_motion, o.media == "print")?;
        }
    }
    let encoded_duration = (media_session.duration * o.fps).ceil() / o.fps;
    let dir = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temp = dir.join(format!(
        ".laymesh-animation-{}-{nonce}.{format}",
        std::process::id()
    ));
    // Reserve uniquely; FFmpeg then writes only this sibling staging file.
    std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)?;
    let mut stage = Staging {
        path: temp,
        child: None,
    };
    let mut cmd = Command::new(&encoder);
    cmd.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "rawvideo",
        "-pixel_format",
        "rgba",
        "-video_size",
        &format!("{w}x{h}"),
        "-framerate",
        &o.fps.to_string(),
        "-i",
        "pipe:0",
    ]);
    if format == "mp4" {
        media_session.audio_args(&mut cmd, encoded_duration);
        cmd.args([
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-crf",
            &o.crf.to_string(),
            "-preset",
            &o.preset,
            "-movflags",
            "+faststart",
        ]);
    } else {
        cmd.arg("-an");
        cmd.args([
            "-filter_complex",
            "split[a][b];[a]palettegen=stats_mode=single[p];[b][p]paletteuse=new=1",
            "-loop",
            "0",
        ]);
    }
    stage.child = Some(
        cmd.arg(&stage.path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Cannot start FFmpeg (install ffmpeg on PATH): {e}"))?,
    );
    let mut stdin = stage.child.as_mut().unwrap().stdin.take().unwrap();
    // Drain bounded diagnostics concurrently so a failing encoder cannot deadlock stdin.
    let stderr = stage.child.as_mut().unwrap().stderr.take().unwrap();
    let log = std::thread::spawn(move || {
        use std::io::Read;
        let mut text = String::new();
        let _ = stderr.take(65536).read_to_string(&mut text);
        text
    });
    drop(first);
    let mut count = 0u64;
    let mut duration = 0.;
    for (scene_path, length) in &media_session.pages {
        let scene: Scene = serde_json::from_reader(std::fs::File::open(scene_path)?)?;
        let length = *length;
        let list = laymesh_render::to_display_list(&scene)?;
        let mut media_frames =
            media_session.renderer(&list, &encoder, o.fps, o.reduced_motion, o.media == "print")?;
        let mut renderer = AnimationRenderer::new(list)?;
        // Round boundaries on the global clock, avoiding accumulated page rounding errors.
        let end = ((duration + length) * o.fps).ceil() as u64;
        while count < end {
            if CANCELLED.load(Ordering::Relaxed) || cancel.is_some_and(Path::exists) {
                return Err("Animation export cancelled".into());
            }
            let time = (count as f64 / o.fps - duration).max(0.).min(length);
            media_frames.sample(time, &mut renderer)?;
            let mut rgba = frame(&scene, &o, w, h, Some(time), &mut renderer)?;
            for p in rgba.chunks_exact_mut(4) {
                let a = p[3] as u32;
                for c in 0..3 {
                    p[c] = ((p[c] as u32 * a + bg[c] as u32 * (255 - a) + 127) / 255) as u8;
                }
                p[3] = 255;
            }
            stdin.write_all(&rgba)?;
            count += 1;
        }
        duration += length;
    }
    drop(stdin);
    let status = stage.child.as_mut().unwrap().wait()?;
    stage.child = None;
    let diagnostic = log.join().unwrap_or_default();
    if !status.success() {
        return Err(format!("FFmpeg failed: {diagnostic}").into());
    }
    if CANCELLED.load(Ordering::Relaxed) || cancel.is_some_and(Path::exists) {
        return Err("Animation export cancelled".into());
    }
    std::fs::rename(&stage.path, output)?;
    Ok(
        json!({"exported":output.to_string_lossy(),"frames":count,"fps":o.fps,"duration":count as f64/o.fps,"width":w,"height":h,"pageCount":pages.len(),"layoutCompilations":pages.len()}),
    )
}
pub fn run(args: &[String]) -> Result<()> {
    let file = std::fs::canonicalize(&args[1])?;
    let source = std::fs::read_to_string(&file)?;
    let mut output = None;
    let mut explicit = json!({});
    let (mut page, mut time, mut step, mut config_path) = (None, None, None, None);
    let mut i = 2;
    let mut authorized_download = false;
    while i < args.len() {
        let flag = &args[i];
        i += 1;
        if flag == "--yes" {
            authorized_download = true;
            continue;
        }
        let value = args.get(i).ok_or("Missing option value")?;
        i += 1;
        match flag.as_str() {
            "-o" | "--output" => output = Some(PathBuf::from(value)),
            "--width" => explicit["width"] = json!(value.parse::<u32>()?),
            "--fps" => explicit["fps"] = json!(value.parse::<f64>()?),
            "--crf" => explicit["crf"] = json!(value.parse::<u8>()?),
            "--media" => explicit["media"] = json!(value),
            "--preset" => explicit["preset"] = json!(value),
            "--background" => explicit["background"] = json!(value),
            "--reduced-motion" => {
                explicit["reduced_motion"] = json!(match value.as_str() {
                    "reduce" | "true" => true,
                    "no-preference" | "false" => false,
                    _ => return Err("Expected reduce or no-preference".into()),
                })
            }
            "--page" => page = Some(value.parse::<usize>()?),
            "--time" => time = Some(value.parse::<f64>()?),
            "--step" => step = Some(value.parse::<usize>()?),
            "--config" => config_path = Some(PathBuf::from(value)),
            "--warnings" if ["show", "hide"].contains(&value.as_str()) => {}
            _ => return Err(format!("Unknown animation option: {flag}").into()),
        }
    }
    let output = output.ok_or("Animation export requires -o")?;
    let (defaults, _) = config::resolve(&file, config_path.as_deref(), &Value::Null, &Value::Null)?;
    let host = super::documents::native_host();
    let file = file.to_string_lossy();
    if args[0] == "animate" {
        if time.is_some() || step.is_some() {
            return Err("Use render for time/step snapshots".into());
        }
        let result = export(
            &file,
            &source,
            host.clone(),
            &output,
            &defaults,
            &explicit,
            page,
            None,
        );
        let result = match result {
            Err(error) if error.downcast_ref::<super::ffmpeg::Missing>().is_some() => {
                let format = output
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("mp4")
                    .to_lowercase();
                let info = super::ffmpeg::status(&defaults, Path::new(file.as_ref()), &format)?;
                let authorized = authorized_download || super::ffmpeg::confirm(&info)?;
                super::ffmpeg::install(
                    &defaults,
                    Path::new(file.as_ref()),
                    &format,
                    authorized,
                    None,
                    false,
                )?;
                export(
                    &file, &source, host, &output, &defaults, &explicit, page, None,
                )?
            }
            other => other?,
        };
        println!("{result}");
        return Ok(());
    }
    if time.is_some() && step.is_some() {
        return Err("Choose --time or --step".into());
    }
    if output.extension().and_then(|s| s.to_str()) != Some("png") {
        return Err("Animation snapshots require PNG".into());
    }
    let scene = if file.ends_with(".laydoc") {
        let p = laymesh_core::document::plan_source(&source, &file, host.clone())?;
        laymesh_core::document::compile_page(p.page(page.unwrap_or(1))?, host)?
    } else {
        if page.is_some_and(|p| p != 1) {
            return Err("Page number out of range".into());
        }
        compile_source(&source, &file, host)?
    };
    let o = options("png", &defaults, &explicit)?;
    let timeline = scene
        .animation
        .as_ref()
        .ok_or("No presentation steps")?
        .timeline_with_media(o.reduced_motion, o.media == "print");
    let t = if let Some(step) = step {
        timeline.step_end(step)?
    } else {
        time.ok_or("Specify --time or --step")?
    };
    let w = o.width;
    let h = (w as f64 * scene.height / scene.width).round().max(1.) as u32;
    let mut renderer = AnimationRenderer::new(laymesh_render::to_display_list(&scene)?)?;
    let mut media_session = super::media_export::Session::new()?;
    if scene.media.cues.iter().any(|c| c.target.is_some()) {
        let encoder = super::ffmpeg::require(&defaults, Path::new(file.as_ref()), "mp4")?;
        media_session.add(&scene, o.reduced_motion, o.media == "print")?;
        let mut frames = media_session.renderer(
            renderer.base(),
            &encoder,
            o.fps,
            o.reduced_motion,
            o.media == "print",
        )?;
        frames.sample(t, &mut renderer)?;
    }
    let rgba = frame(&scene, &o, w, h, Some(t), &mut renderer)?;
    let mut bytes = vec![];
    {
        let mut encoder = png::Encoder::new(&mut bytes, w, h);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&rgba)?;
    }
    super::export::write_atomic(&output, &bytes)?;
    println!("已导出：{}", output.display());
    Ok(())
}
