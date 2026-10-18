//! Bounded frame decoding and document-wide audio planning for native exports.
use laymesh_core::{media::Cue, model::Scene};
use laymesh_render::display_list::{DisplayList, DrawKind, DrawNode};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::{Path, PathBuf},
    process::{Child, ChildStdout, Command, Stdio},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
pub struct Session {
    directory: PathBuf,
    pub pages: Vec<(PathBuf, f64)>,
    files: BTreeMap<String, PathBuf>,
    audio: Vec<(Cue, f64, f64)>,
    global: BTreeSet<String>,
    pub duration: f64,
}
impl Session {
    pub fn new() -> Result<Self> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("laymesh-media-{}-{nonce}", std::process::id()));
        std::fs::create_dir(&directory)?;
        Ok(Self {
            directory,
            pages: vec![],
            files: BTreeMap::new(),
            audio: vec![],
            global: BTreeSet::new(),
            duration: 0.,
        })
    }
    pub fn add(&mut self, scene: &Scene, reduced: bool, print: bool) -> Result<()> {
        let length =
            laymesh_core::media::duration(&scene.media, scene.animation.as_ref(), reduced, print)?;
        if length <= 0. {
            return Err(
                "Animation duration must be positive; add page.step or finite media playback"
                    .into(),
            );
        }
        for (id, a) in &scene.media.assets {
            if !self.files.contains_key(id) {
                let path = self.directory.join(format!("{id}.media"));
                std::fs::write(&path, a.data.as_ref())?;
                self.files.insert(id.clone(), path);
            }
        }
        for c in &scene.media.cues {
            if c.trigger == "click" && c.export_start.is_none() {
                eprintln!(
                    "{}:{}:{}: W_MEDIA_EXPORT: Click video has no export_start; exporting its poster",
                    c.file, c.loc.line, c.loc.column
                );
                continue;
            }
            let a = &scene.media.assets[&c.asset];
            if c.muted || !a.has_audio || c.volume == 0. {
                continue;
            }
            if c.scope == "document" {
                let key = serde_json::to_string(c)?;
                if self.global.insert(key) {
                    self.audio.push((c.clone(), 0., f64::INFINITY));
                }
            } else if let Some(start) = c.start(scene.animation.as_ref(), reduced, print) {
                self.audio
                    .push((c.clone(), self.duration + start, self.duration + length));
            }
        }
        let path = self
            .directory
            .join(format!("page-{}.json", self.pages.len()));
        let mut compact = scene.clone();
        for a in compact.media.assets.values_mut() {
            a.data = Default::default();
            a.poster = Default::default();
            a.preview_audio = Default::default();
        }
        serde_json::to_writer(std::fs::File::create(&path)?, &compact)?;
        self.pages.push((path, length));
        self.duration += length;
        Ok(())
    }
    pub fn audio_args(&self, cmd: &mut Command, encoded_duration: f64) -> bool {
        if self.audio.is_empty() {
            cmd.arg("-an");
            return false;
        }
        let mut filters = vec![];
        let mut labels = String::new();
        for (i, (c, start, end)) in self.audio.iter().enumerate() {
            cmd.arg("-i").arg(&self.files[&c.asset]);
            let mut f = format!(
                "[{}:a:0]aresample=48000,atrim=start={}:end={},asetpts=PTS-STARTPTS",
                i + 1,
                c.trim_start,
                c.trim_end
            );
            if c.looped {
                f += &format!(
                    ",aloop=loop=-1:size={},asetpts=N/SR/TB",
                    ((c.trim_end - c.trim_start) * 48000.).round().max(1.) as u64
                );
            }
            let limit = end.min(encoded_duration);
            let start = if c.scope == "document" {
                c.delay
            } else {
                *start
            };
            f += &format!(
                ",volume={},adelay={}:all=1,asetpts=N/SR/TB,apad,atrim=end={limit}[a{i}]",
                c.volume,
                (start * 1000.).round() as u64
            );
            filters.push(f);
            labels += &format!("[a{i}]");
        }
        filters.push(format!("{labels}amix=inputs={}:normalize=0,alimiter=limit=0.95:level=0:latency=1,asetpts=N/SR/TB,apad,atrim=end={encoded_duration}[mixed]",self.audio.len()));
        cmd.args([
            "-filter_complex",
            &filters.join(";"),
            "-map",
            "0:v:0",
            "-map",
            "[mixed]",
            "-c:a",
            "aac",
            "-b:a",
            "192k",
            "-ar",
            "48000",
            "-ac",
            "2",
        ]);
        true
    }
    pub fn renderer(
        &self,
        list: &DisplayList,
        exe: &Path,
        fps: f64,
        reduced: bool,
        print: bool,
    ) -> Result<Frames> {
        fn find(
            nodes: &[DrawNode],
            target: &str,
            inside: bool,
        ) -> Option<(String, serde_json::Value)> {
            for n in nodes {
                let inside = inside || n.name == target;
                if inside && n.hint["mediaId"].is_string() {
                    if let DrawKind::Image { resource, .. } = &n.kind {
                        return Some((resource.clone(), n.hint["mediaCrop"].clone()));
                    }
                }
                if let DrawKind::Group { children, .. } = &n.kind {
                    if let Some(v) = find(children, target, inside) {
                        return Some(v);
                    }
                }
            }
            None
        }
        let mut tracks = vec![];
        for c in &list.media.cues {
            let Some(target) = &c.target else { continue };
            let Some(start) = c.start(list.animation.as_ref(), reduced, print) else {
                continue;
            };
            let Some((resource, crop)) = find(&list.nodes, target, false) else {
                return Err("Video drawing target is missing".into());
            };
            let asset = &list.media.assets[&c.asset];
            let mut w = asset.width;
            let mut h = asset.height;
            let mut filter = String::new();
            if let (Some(offset), Some(size)) = (crop["offset"].as_array(), crop["size"].as_array())
            {
                let x = (offset[0].as_f64().unwrap_or(0.) * w as f64).round() as u32;
                let y = (offset[1].as_f64().unwrap_or(0.) * h as f64).round() as u32;
                w = (size[0].as_f64().unwrap_or(1.) * w as f64).round().max(1.) as u32;
                h = (size[1].as_f64().unwrap_or(1.) * h as f64).round().max(1.) as u32;
                filter = format!("crop={w}:{h}:{x}:{y}:exact=1,");
            }
            filter += &format!("fps={fps},format=rgba");
            if w as u64 * h as u64 > 32_000_000 {
                return Err("Video source exceeds the frame pixel budget".into());
            }
            tracks.push(Track {
                cue: c.clone(),
                start,
                path: self.files[&c.asset].clone(),
                resource,
                exe: exe.into(),
                fps,
                filter,
                w,
                h,
                decoder: None,
                index: 0,
                last: vec![],
            });
        }
        Ok(Frames { tracks })
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
struct Decoder {
    child: Child,
    output: ChildStdout,
}
impl Drop for Decoder {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
struct Track {
    cue: Cue,
    start: f64,
    path: PathBuf,
    resource: String,
    exe: PathBuf,
    fps: f64,
    filter: String,
    w: u32,
    h: u32,
    decoder: Option<Decoder>,
    index: u64,
    last: Vec<u8>,
}
impl Track {
    fn open(&mut self) -> Result<()> {
        let mut child = Command::new(&self.exe)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-threads",
                "1",
                "-ss",
                &self.cue.trim_start.to_string(),
                "-i",
            ])
            .arg(&self.path)
            .args([
                "-an",
                "-vf",
                &self.filter,
                "-f",
                "rawvideo",
                "-pix_fmt",
                "rgba",
                "pipe:1",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let output = child.stdout.take().unwrap();
        self.decoder = Some(Decoder { child, output });
        self.index = 0;
        self.last.clear();
        Ok(())
    }
    fn sample(
        &mut self,
        time: f64,
        renderer: &mut laymesh_render::animation::AnimationRenderer,
    ) -> Result<()> {
        let Some(source) = self.cue.sample(time, self.start) else {
            return Ok(());
        };
        let index = ((source - self.cue.trim_start) * self.fps + 1e-7).floor() as u64;
        if self.decoder.is_none() || index + 1 < self.index {
            self.open()?;
        }
        while self.index <= index {
            let mut bytes = vec![0; self.w as usize * self.h as usize * 4];
            match self.decoder.as_mut().unwrap().output.read_exact(&mut bytes) {
                Ok(()) => {
                    self.last = bytes;
                    self.index += 1;
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    let status = self.decoder.as_mut().unwrap().child.wait()?;
                    if !status.success() || self.last.is_empty() {
                        return Err(
                            format!("Video decoder failed for {}", self.path.display()).into()
                        );
                    }
                    break;
                }
                Err(e) => return Err(e.into()),
            }
        }
        renderer.set_media_frame(&self.resource, self.w, self.h, &self.last)?;
        Ok(())
    }
}
pub struct Frames {
    tracks: Vec<Track>,
}
impl Frames {
    pub fn sample(
        &mut self,
        time: f64,
        renderer: &mut laymesh_render::animation::AnimationRenderer,
    ) -> Result<()> {
        for t in &mut self.tracks {
            t.sample(time, renderer)?;
        }
        Ok(())
    }
}
