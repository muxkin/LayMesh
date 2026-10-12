//! One FFmpeg resolver and installation directory for CLI, Python and editors.
//! Finding an existing program never accesses the network. Installation requires
//! explicit consent from the calling frontend, never a persisted config flag.
use flate2::read::GzDecoder;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const MANIFEST: &str = include_str!("../assets/ffmpeg.json");
#[derive(Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub executable: Option<PathBuf>,
    pub directory: Option<PathBuf>,
    pub search_paths: Vec<PathBuf>,
    pub search_system: bool,
    pub download_source: Option<String>,
    pub mirror_base_url: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            executable: None,
            directory: None,
            search_paths: vec![],
            search_system: true,
            download_source: None,
            mirror_base_url: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Found {
    pub path: PathBuf,
    pub version: String,
    pub origin: String,
}
#[derive(Debug)]
pub struct Missing;
impl std::fmt::Display for Missing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "E_FFMPEG_MISSING: No usable FFmpeg found. Run `laymesh ffmpeg install` to confirm installation, or configure export.ffmpeg.executable. / 未找到可用 FFmpeg，请确认安装或配置路径。"
        )
    }
}
impl std::error::Error for Missing {}
fn manifest() -> Value {
    serde_json::from_str(MANIFEST).expect("pinned FFmpeg manifest")
}
fn platform() -> String {
    let os = match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{os}-{arch}")
}
fn executable_name() -> &'static str {
    if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    }
}
fn home() -> Result<PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        .map(PathBuf::from)
        .ok_or_else(|| "Cannot locate the user directory".into())
}
fn expand(path: &Path, base: &Path) -> Result<PathBuf> {
    let s = path.to_string_lossy();
    if s == "~" {
        return home();
    }
    if let Some(rest) = s.strip_prefix("~/").or_else(|| s.strip_prefix("~\\")) {
        return Ok(home()?.join(rest));
    }
    Ok(if path.is_absolute() {
        path.to_owned()
    } else {
        base.join(path)
    })
}
fn private_directory(settings: &Settings, file: &Path) -> Result<PathBuf> {
    if let Some(path) = &settings.directory {
        return expand(path, file.parent().unwrap_or(Path::new(".")));
    }
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or(home()?.join("AppData/Local"))
    } else if cfg!(target_os = "macos") {
        home()?.join("Library/Application Support")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or(home()?.join(".local/share"))
    };
    Ok(base.join("laymesh/ffmpeg"))
}
fn settings(config: &Value) -> Result<Settings> {
    let value = &config["export"]["ffmpeg"];
    let s: Settings = if value.is_null() {
        Settings::default()
    } else {
        serde_json::from_value(value.clone())?
    };
    if !["auto", "upstream", "mirror"].contains(&s.download_source.as_deref().unwrap_or("auto")) {
        return Err("export.ffmpeg.download_source requires auto, upstream or mirror".into());
    }
    for path in [&s.executable, &s.directory].into_iter().flatten() {
        if path.as_os_str().is_empty() {
            return Err("FFmpeg configuration paths must not be empty".into());
        }
    }
    if let Some(url) = &s.mirror_base_url {
        let u = reqwest::Url::parse(url)?;
        if !["http", "https"].contains(&u.scheme())
            || u.host_str().is_none()
            || !u.username().is_empty()
            || u.password().is_some()
        {
            return Err("FFmpeg mirror must be an HTTP(S) URL without embedded credentials".into());
        }
    }
    Ok(s)
}
// Bound probes, including accidentally configured programs that never exit.
fn output(path: &Path, args: &[&str]) -> Option<String> {
    let mut cmd = Command::new(path);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let mut child = cmd.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut bytes = vec![];
        let _ = stdout.take(256 * 1024).read_to_end(&mut bytes);
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Err(_) => break false,
            _ => {}
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            break false;
        }
        thread::sleep(Duration::from_millis(10));
    };
    let bytes = reader.join().ok()?;
    success.then(|| String::from_utf8_lossy(&bytes).into_owned())
}
fn probe(path: &Path, format: &str, origin: &str) -> Option<Found> {
    if !path.is_file() {
        return None;
    }
    let version = output(path, &["-version"])?;
    let line = version.lines().find(|s| s.starts_with("ffmpeg version "))?;
    let encoders = output(path, &["-hide_banner", "-encoders"])?;
    if format == "mp4" && !encoders.split_whitespace().any(|s| s == "libx264") {
        return None;
    }
    if format == "gif" {
        if !encoders.split_whitespace().any(|s| s == "gif") {
            return None;
        }
        let filters = output(path, &["-hide_banner", "-filters"])?;
        if !["palettegen", "paletteuse"]
            .iter()
            .all(|s| filters.split_whitespace().any(|v| v == *s))
        {
            return None;
        }
    }
    Some(Found {
        path: fs::canonicalize(path).ok()?,
        version: line.to_owned(),
        origin: origin.into(),
    })
}
fn record(directory: &Path, found: &Found) {
    let save = || -> Result<()> {
        fs::create_dir_all(directory)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("discovery.lock"))?;
        lock.lock()?;
        let data = json!({"schema":1,"platform":platform(),"found":found,"checkedAt":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()});
        super::export::write_atomic(
            &directory.join("discovery.json"),
            &serde_json::to_vec_pretty(&data)?,
        )?;
        Ok(())
    };
    // A read-only data directory must not prevent use of an existing encoder.
    if let Err(e) = save() {
        eprintln!("FFmpeg discovery record could not be saved: {e}");
    }
}
pub fn locate(config: &Value, file: &Path, format: &str) -> Result<Option<Found>> {
    if !["mp4", "gif"].contains(&format) {
        return Err("FFmpeg format requires mp4 or gif".into());
    }
    let s = settings(config)?;
    let dir = private_directory(&s, file)?;
    let base = file.parent().unwrap_or(Path::new("."));
    if let Some(p) = &s.executable {
        let path = expand(p, base)?;
        let f = probe(&path, format, "configured").ok_or_else(|| {
            format!(
                "Configured FFmpeg is missing or cannot encode {format}: {}",
                path.display()
            )
        })?;
        record(&dir, &f);
        return Ok(Some(f));
    }
    let m = manifest();
    let mut candidates = vec![
        (dir.join(executable_name()), "private"),
        (
            dir.join(platform())
                .join(m["version"].as_str().unwrap())
                .join(executable_name()),
            "private",
        ),
    ];
    if let Ok(bytes) = fs::read(dir.join("discovery.json")) {
        if let Ok(data) = serde_json::from_slice::<Value>(&bytes) {
            if data["platform"] == platform() {
                if let Some(p) = data["found"]["path"].as_str() {
                    candidates.push((PathBuf::from(p), "recorded"));
                }
            }
        }
    }
    for p in &s.search_paths {
        candidates.push((expand(p, base)?.join(executable_name()), "search-path"));
    }
    if s.search_system {
        if let Some(paths) = std::env::var_os("PATH") {
            candidates.extend(
                std::env::split_paths(&paths)
                    .filter(|p| !p.as_os_str().is_empty())
                    .map(|p| (p.join(executable_name()), "path")),
            );
        }
        if cfg!(windows) {
            for key in ["ProgramFiles", "ProgramData", "USERPROFILE"] {
                if let Some(p) = std::env::var_os(key) {
                    for suffix in [
                        "ffmpeg/bin",
                        "chocolatey/bin",
                        "scoop/apps/ffmpeg/current/bin",
                    ] {
                        candidates.push((
                            PathBuf::from(&p).join(suffix).join(executable_name()),
                            "standard",
                        ));
                    }
                }
            }
            candidates.push((PathBuf::from("C:/ffmpeg/bin/ffmpeg.exe"), "standard"));
        } else {
            for p in [
                "/usr/local/bin",
                "/usr/bin",
                "/bin",
                "/opt/homebrew/bin",
                "/opt/local/bin",
            ] {
                candidates.push((Path::new(p).join(executable_name()), "standard"));
            }
            candidates.push((
                home()?.join(".local/bin").join(executable_name()),
                "standard",
            ));
        }
    }
    let mut seen = std::collections::HashSet::new();
    for (p, origin) in candidates {
        if !seen.insert(p.clone()) {
            continue;
        }
        if let Some(found) = probe(&p, format, origin) {
            record(&dir, &found);
            return Ok(Some(found));
        }
    }
    Ok(None)
}
pub fn require(config: &Value, file: &Path, format: &str) -> Result<PathBuf> {
    locate(config, file, format)?
        .map(|f| f.path)
        .ok_or_else(|| Box::new(Missing) as Box<dyn std::error::Error>)
}
pub fn status(config: &Value, file: &Path, format: &str) -> Result<Value> {
    let s = settings(config)?;
    let m = manifest();
    let p = platform();
    let found = locate(config, file, format)?;
    Ok(
        json!({"available":found.is_some(),"found":found,"directory":private_directory(&s,file)?,
        "download":{"supported":m["assets"].get(&p).is_some(),"version":m["version"],"platform":p,"bytes":m["assets"][&p]["bytes"],"source":s.download_source.as_deref().unwrap_or("auto")}}),
    )
}
fn cancelled(cancel: Option<&Path>) -> Result<()> {
    if super::animation::cancelled() || cancel.is_some_and(Path::exists) {
        return Err("FFmpeg installation cancelled / 已取消下载".into());
    }
    Ok(())
}
fn client(timeout: u64) -> Result<Client> {
    Ok(Client::builder()
        .user_agent(concat!("LayMesh/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(Duration::from_secs(timeout.min(5)))
        .timeout(Duration::from_secs(timeout))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()?)
}
fn china_hint() -> bool {
    let locale = std::env::var("LAYMESH_FFMPEG_LOCALE")
        .ok()
        .or_else(sys_locale::get_locale)
        .unwrap_or_default();
    let zone = std::env::var("TZ").unwrap_or_default();
    let system_zone = fs::read_link("/etc/localtime")
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    locale.to_lowercase().replace('_', "-").ends_with("-cn")
        || [zone, system_zone]
            .iter()
            .any(|z| z.contains("Asia/Shanghai") || z.contains("Asia/Chongqing"))
}
fn sources(s: &Settings, m: &Value, asset: &str) -> Result<Vec<String>> {
    let upstream = format!(
        "{}/{}/{asset}",
        m["upstream"].as_str().unwrap(),
        m["release"].as_str().unwrap()
    );
    let mirror = format!(
        "{}/{}/{asset}",
        s.mirror_base_url
            .as_deref()
            .unwrap_or(m["mirror"].as_str().unwrap())
            .trim_end_matches('/'),
        m["release"].as_str().unwrap()
    );
    let mode = s.download_source.as_deref().unwrap_or("auto");
    if mode == "upstream" {
        return Ok(vec![upstream]);
    }
    if mode == "mirror" {
        return Ok(vec![mirror]);
    }
    let ordered = if china_hint() {
        vec![mirror, upstream]
    } else {
        vec![upstream, mirror]
    };
    // Hints seed the order; bounded measurements select the reachable source.
    let c = client(2)?;
    let mut probes = thread::scope(|scope| {
        let handles: Vec<_> = ordered
            .into_iter()
            .enumerate()
            .map(|(i, url)| {
                let c = c.clone();
                scope.spawn(move || {
                    let start = Instant::now();
                    let ok = c.head(&url).send().is_ok_and(|r| r.status().is_success());
                    (if ok { start.elapsed() } else { Duration::MAX }, i, url)
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("source probe"))
            .collect::<Vec<_>>()
    });
    probes.sort_by_key(|p| (p.0, p.1));
    Ok(probes.into_iter().map(|p| p.2).collect())
}
struct Stage(PathBuf);
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn request(
    builder: reqwest::blocking::RequestBuilder,
    cancel: Option<&Path>,
) -> Result<reqwest::blocking::Response> {
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    thread::spawn(move || {
        let _ = sender.send(builder.send());
    });
    loop {
        cancelled(cancel)?;
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(result) => return Ok(result?.error_for_status()?),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(_) => return Err("FFmpeg request interrupted".into()),
        }
    }
}
struct ProgressReader<R> {
    inner: R,
    bytes: u64,
    total: Option<u64>,
    last: Instant,
    json: bool,
    cancel: Option<PathBuf>,
}
// Poll cancellation even when the remote server stops sending body bytes.
struct CancelableBody {
    receiver: std::sync::mpsc::Receiver<io::Result<Vec<u8>>>,
    pending: io::Cursor<Vec<u8>>,
    cancel: Option<PathBuf>,
}
impl CancelableBody {
    fn new(mut response: reqwest::blocking::Response, cancel: Option<&Path>) -> Self {
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        thread::spawn(move || {
            loop {
                let mut bytes = vec![0; 65536];
                match response.read(&mut bytes) {
                    Ok(0) => {
                        let _ = sender.send(Ok(vec![]));
                        break;
                    }
                    Ok(n) => {
                        bytes.truncate(n);
                        if sender.send(Ok(bytes)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                        break;
                    }
                }
            }
        });
        Self {
            receiver,
            pending: io::Cursor::new(vec![]),
            cancel: cancel.map(Path::to_owned),
        }
    }
}
impl Read for CancelableBody {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            cancelled(self.cancel.as_deref()).map_err(|e| io::Error::other(e.to_string()))?;
            let n = self.pending.read(buf)?;
            if n > 0 {
                return Ok(n);
            }
            match self.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(Ok(bytes)) => {
                    if bytes.is_empty() {
                        return Ok(0);
                    }
                    self.pending = io::Cursor::new(bytes);
                }
                Ok(Err(e)) => return Err(e),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "FFmpeg download interrupted",
                    ));
                }
            }
        }
    }
}
impl<R: Read> Read for ProgressReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        cancelled(self.cancel.as_deref()).map_err(|e| io::Error::other(e.to_string()))?;
        let n = self.inner.read(buf)?;
        self.bytes += n as u64;
        if self.last.elapsed() >= Duration::from_millis(200) || n == 0 {
            self.last = Instant::now();
            if self.json {
                eprintln!(
                    "{}",
                    json!({"type":"progress","bytes":self.bytes,"total":self.total})
                );
            } else if io::stderr().is_terminal() {
                eprint!("\rDownloading FFmpeg: {} KiB", self.bytes / 1024);
            }
        }
        Ok(n)
    }
}
fn checked_write(
    mut reader: impl Read,
    path: &Path,
    sha: &str,
    limit: u64,
    cancel: Option<&Path>,
) -> Result<()> {
    let mut file = File::create(path)?;
    let mut hash = Sha256::new();
    let mut count = 0u64;
    let mut buf = [0u8; 65536];
    loop {
        cancelled(cancel)?;
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > limit {
            return Err("FFmpeg asset exceeds its pinned size".into());
        }
        hash.update(&buf[..n]);
        file.write_all(&buf[..n])?;
    }
    if format!("{:x}", hash.finalize()) != sha {
        return Err("FFmpeg SHA-256 verification failed".into());
    }
    file.sync_all()?;
    Ok(())
}
pub fn install(
    config: &Value,
    file: &Path,
    format: &str,
    authorized: bool,
    cancel: Option<&Path>,
    progress_json: bool,
) -> Result<Found> {
    install_with_manifest(
        config,
        file,
        format,
        authorized,
        cancel,
        progress_json,
        &manifest(),
    )
}
fn install_with_manifest(
    config: &Value,
    file: &Path,
    format: &str,
    authorized: bool,
    cancel: Option<&Path>,
    progress_json: bool,
    m: &Value,
) -> Result<Found> {
    if let Some(found) = locate(config, file, format)? {
        return Ok(found);
    }
    if !authorized {
        return Err(Box::new(Missing));
    }
    cancelled(cancel)?;
    let s = settings(config)?;
    let p = platform();
    let asset = m["assets"].get(&p).ok_or_else(|| format!("Automatic FFmpeg installation is unavailable for {p}; configure export.ffmpeg.executable"))?;
    let dir = private_directory(&s, file)?;
    fs::create_dir_all(&dir)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("install.lock"))?;
    let start = Instant::now();
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) => {
                cancelled(cancel)?;
                if start.elapsed() > Duration::from_secs(300) {
                    return Err("Timed out waiting for FFmpeg installation".into());
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(e.into()),
        }
    }
    if let Some(found) = locate(config, file, format)? {
        return Ok(found);
    }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let stage = Stage(dir.join(format!(".install-{}-{nonce}", std::process::id())));
    fs::create_dir(&stage.0)?;
    let c = client(300)?;
    let urls = sources(&s, &m, asset["file"].as_str().unwrap())?;
    let mut failures = vec![];
    for url in urls {
        cancelled(cancel)?;
        let attempt = || -> Result<Found> {
            let response = request(c.get(&url), cancel)?;
            let total = response.content_length();
            let reader = ProgressReader {
                inner: CancelableBody::new(response, cancel),
                bytes: 0,
                total,
                last: Instant::now(),
                json: progress_json,
                cancel: cancel.map(Path::to_owned),
            };
            let binary = stage.0.join(executable_name());
            checked_write(
                GzDecoder::new(reader),
                &binary,
                asset["sha256"].as_str().unwrap(),
                asset["bytes"].as_u64().unwrap(),
                cancel,
            )?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&binary, fs::Permissions::from_mode(0o755))?;
            }
            if probe(&binary, format, "private").is_none() {
                return Err("Downloaded FFmpeg cannot run or lacks the required encoder".into());
            }
            let base = url.rsplit_once('/').unwrap().0;
            for key in ["license", "readme"] {
                let metadata = &asset[key];
                let response = request(
                    c.get(format!("{base}/{}", metadata["file"].as_str().unwrap())),
                    cancel,
                )?;
                checked_write(
                    CancelableBody::new(response, cancel),
                    &stage.0.join(key.to_uppercase()),
                    metadata["sha256"].as_str().unwrap(),
                    1024 * 1024,
                    cancel,
                )?;
            }
            let info = json!({"schema":1,"version":m["version"],"platform":p,"sha256":asset["sha256"],"source":url,"installedAt":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()});
            fs::write(
                stage.0.join("installation.json"),
                serde_json::to_vec_pretty(&info)?,
            )?;
            cancelled(cancel)?;
            let target = dir.join(&p).join(m["version"].as_str().unwrap());
            fs::create_dir_all(target.parent().unwrap())?;
            if target.exists() {
                fs::rename(&target, dir.join(format!(".invalid-{p}-{nonce}")))?;
            }
            fs::rename(&stage.0, &target)?;
            let found = probe(&target.join(executable_name()), format, "private")
                .ok_or("Installed FFmpeg failed validation")?;
            record(&dir, &found);
            Ok(found)
        };
        match attempt() {
            Ok(found) => return Ok(found),
            Err(error) => {
                cancelled(cancel)?;
                failures.push(error.to_string());
            }
        }
    }
    Err(format!("FFmpeg installation failed: {}", failures.join("; ")).into())
}
pub fn confirm(info: &Value) -> Result<bool> {
    if !io::stdin().is_terminal() {
        return Ok(false);
    }
    eprintln!(
        "FFmpeg {} is missing. Download to {}? / 未找到 FFmpeg，是否下载到私有目录？ [y/N]",
        info["download"]["version"].as_str().unwrap_or(""),
        info["directory"].as_str().unwrap_or("")
    );
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(["y", "yes", "是"].contains(&answer.trim().to_lowercase().as_str()))
}
pub fn run(args: &[String]) -> Result<()> {
    super::animation::install_cancel();
    let command = args.get(1).map(String::as_str).unwrap_or("status");
    if !["status", "install"].contains(&command) {
        return Err("Usage: laymesh ffmpeg status|install [--file SOURCE] [--config FILE] [--format mp4|gif] [--json] [--yes] [--settings-stdin] [--cancel-file FILE]".into());
    }
    let mut file = std::env::current_dir()?.join("ffmpeg.lay");
    let mut config_path = None;
    let mut format = "mp4".to_string();
    let mut yes = false;
    let mut stdin_settings = false;
    let mut cancel = None;
    let mut progress_json = false;
    let mut i = 2;
    while i < args.len() {
        let flag = &args[i];
        i += 1;
        match flag.as_str() {
            "--yes" => yes = true,
            "--json" => {}
            "--settings-stdin" => stdin_settings = true,
            "--progress-json" => progress_json = true,
            "--file" | "--config" | "--format" | "--cancel-file" => {
                let value = args.get(i).ok_or("Missing FFmpeg option value")?;
                i += 1;
                match flag.as_str() {
                    "--file" => file = PathBuf::from(value),
                    "--config" => config_path = Some(PathBuf::from(value)),
                    "--format" => format = value.clone(),
                    _ => cancel = Some(PathBuf::from(value)),
                }
            }
            _ => return Err(format!("Unknown FFmpeg option: {flag}").into()),
        }
    }
    if !file.is_absolute() {
        file = std::env::current_dir()?.join(file);
    }
    let input: Value = if stdin_settings {
        serde_json::from_reader(io::stdin())?
    } else {
        Value::Null
    };
    let (config, _) = laymesh_render::config::resolve(
        &file,
        config_path.as_deref(),
        &input["editor_user"],
        &input["editor_workspace"],
    )?;
    let info = status(&config, &file, &format)?;
    if command == "status" {
        println!("{info}");
        return Ok(());
    }
    let authorized = yes || info["available"] == true || (!stdin_settings && confirm(&info)?);
    let found = install(
        &config,
        &file,
        &format,
        authorized,
        cancel.as_deref(),
        progress_json,
    )?;
    println!(
        "{}",
        json!({"available":true,"found":found,"directory":info["directory"]})
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pinned_platforms_have_integrity_and_license_metadata() {
        for (_, a) in manifest()["assets"].as_object().unwrap() {
            assert_eq!(a["sha256"].as_str().unwrap().len(), 64);
            assert!(a["bytes"].as_u64().unwrap() > 1_000_000);
            assert_eq!(a["license"]["sha256"].as_str().unwrap().len(), 64);
        }
    }
    #[test]
    fn invalid_source_and_persistent_consent_are_rejected() {
        assert!(settings(&json!({"export":{"ffmpeg":{"download_source":"anything"}}})).is_err());
        assert!(settings(&json!({"export":{"ffmpeg":{"allow_download":true}}})).is_err());
        assert!(settings(&json!({"export":{"ffmpeg":{"executable":""}}})).is_err());
    }
    #[test]
    fn checksum_and_size_are_checked_before_installing() {
        let dir =
            std::env::temp_dir().join(format!("laymesh-ffmpeg-integrity-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let stage = Stage(dir);
        let p = stage.0.join("asset");
        let hash = format!("{:x}", Sha256::digest(b"verified"));
        assert!(checked_write(&b"verified"[..], &p, &hash, 8, None).is_ok());
        assert!(checked_write(&b"tampered"[..], &p, &hash, 8, None).is_err());
        assert!(checked_write(&b"verified-extra"[..], &p, &hash, 8, None).is_err());
        let cancelled_file = stage.0.join("cancel");
        fs::write(&cancelled_file, b"").unwrap();
        assert!(checked_write(&b"verified"[..], &p, &hash, 8, Some(&cancelled_file)).is_err());
    }
    #[cfg(unix)]
    fn fixture(name: &str) -> (Stage, Value, PathBuf) {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "laymesh-ffmpeg-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&dir).unwrap();
        let config = json!({"export":{"ffmpeg":{"directory":dir,"search_system":false}}});
        let file = dir.join("test.lay");
        (Stage(dir), config, file)
    }
    #[cfg(unix)]
    fn script() -> Vec<u8> {
        b"#!/bin/sh\ncase $* in *-version*) echo 'ffmpeg version fixture';; *-encoders*) echo 'V libx264 gif';; *-filters*) echo 'palettegen paletteuse';; esac\n".to_vec()
    }
    #[cfg(unix)]
    fn put_binary(path: &Path) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, script()).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn private_precedes_recorded_paths_and_invalid_records_are_rediscovered() {
        let (stage, mut config, file) = fixture("discovery");
        let outside = stage.0.join("external");
        fs::create_dir(&outside).unwrap();
        put_binary(&outside.join("ffmpeg"));
        config["export"]["ffmpeg"]["search_paths"] = json!([outside]);
        assert_eq!(
            locate(&config, &file, "mp4").unwrap().unwrap().origin,
            "search-path"
        );
        let data: Value =
            serde_json::from_slice(&fs::read(stage.0.join("discovery.json")).unwrap()).unwrap();
        assert!(data["found"]["path"].as_str().unwrap().contains("external"));
        put_binary(&stage.0.join("ffmpeg"));
        assert_eq!(
            locate(&config, &file, "mp4").unwrap().unwrap().origin,
            "private"
        );
        fs::remove_file(stage.0.join("ffmpeg")).unwrap();
        assert_eq!(
            locate(&config, &file, "mp4").unwrap().unwrap().origin,
            "search-path"
        );
        config["export"]["ffmpeg"]["executable"] = json!(outside.join("ffmpeg"));
        put_binary(&stage.0.join("ffmpeg"));
        assert_eq!(
            locate(&config, &file, "mp4").unwrap().unwrap().origin,
            "configured"
        );
    }
    #[cfg(unix)]
    #[test]
    fn missing_program_needs_consent_and_does_not_create_install_files() {
        let (stage, config, file) = fixture("consent");
        let info = status(&config, &file, "mp4").unwrap();
        assert_eq!(info["available"], false);
        assert!(
            install(&config, &file, "mp4", false, None, false)
                .unwrap_err()
                .downcast_ref::<Missing>()
                .is_some()
        );
        assert_eq!(fs::read_dir(&stage.0).unwrap().count(), 0);
        let cancel = stage.0.join("cancel");
        fs::write(&cancel, b"").unwrap();
        assert!(
            install(&config, &file, "mp4", true, Some(&cancel), false)
                .unwrap_err()
                .to_string()
                .contains("cancelled")
        );
        assert!(!stage.0.join("install.lock").exists());
    }
    #[cfg(unix)]
    fn server(
        bad_binary: bool,
        slow_body: bool,
    ) -> (String, std::sync::Arc<std::sync::atomic::AtomicUsize>) {
        use std::net::TcpListener;
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let count = hits.clone();
        thread::spawn(move || {
            for incoming in listener.incoming() {
                let Ok(mut stream) = incoming else { break };
                count.fetch_add(1, Ordering::Relaxed);
                let mut header = vec![];
                let mut one = [0u8; 1];
                while stream.read(&mut one).unwrap_or(0) > 0 {
                    header.push(one[0]);
                    if header.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8_lossy(&header);
                let path = request.split_whitespace().nth(1).unwrap_or("");
                let body = if path.ends_with("LICENSE") {
                    b"license".to_vec()
                } else if path.ends_with("README") {
                    b"readme".to_vec()
                } else {
                    let content = if bad_binary {
                        b"tampered".to_vec()
                    } else {
                        script()
                    };
                    let mut gzip =
                        flate2::write::GzEncoder::new(vec![], flate2::Compression::default());
                    gzip.write_all(&content).unwrap();
                    gzip.finish().unwrap()
                };
                // Make source ordering deterministic: the valid upstream fails
                // its reachability probe but serves GET, so the tampered mirror
                // is tried first regardless of machine load or scheduling.
                let status = if request.starts_with("HEAD") && !bad_binary && !slow_body {
                    "503 Service Unavailable"
                } else {
                    "200 OK"
                };
                let _ = write!(
                    stream,
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                if !request.starts_with("HEAD") {
                    if slow_body {
                        thread::sleep(Duration::from_secs(2));
                    }
                    let _ = stream.write_all(&body);
                }
            }
        });
        (url, hits)
    }
    #[cfg(unix)]
    fn test_manifest(upstream: &str) -> Value {
        let hash = |bytes: &[u8]| format!("{:x}", Sha256::digest(bytes));
        let bytes = script();
        let mut m = manifest();
        m["upstream"] = json!(upstream);
        m["assets"][platform()] = json!({"file":"ffmpeg.gz","bytes":bytes.len(),"sha256":hash(&bytes),"license":{"file":"LICENSE","sha256":hash(b"license")},"readme":{"file":"README","sha256":hash(b"readme")}});
        m
    }
    #[cfg(unix)]
    #[test]
    fn tampered_mirror_falls_back_and_concurrent_installers_reuse_one_copy() {
        use std::sync::atomic::Ordering;
        let (stage, mut config, file) = fixture("fallback");
        let (origin, origin_hits) = server(false, false);
        let (mirror, mirror_hits) = server(true, false);
        config["export"]["ffmpeg"]["mirror_base_url"] = json!(mirror);
        let m = test_manifest(&origin);
        let results = thread::scope(|scope| {
            let a = scope.spawn(|| {
                install_with_manifest(&config, &file, "mp4", true, None, false, &m).unwrap()
            });
            let b = scope.spawn(|| {
                install_with_manifest(&config, &file, "mp4", true, None, false, &m).unwrap()
            });
            (a.join().unwrap(), b.join().unwrap())
        });
        assert_eq!(results.0.path, results.1.path);
        assert!(results.0.path.starts_with(stage.0.canonicalize().unwrap()));
        assert_eq!(origin_hits.load(Ordering::Relaxed), 4); // HEAD, binary, README, license.
        assert_eq!(mirror_hits.load(Ordering::Relaxed), 2); // HEAD then rejected binary.
        assert!(results.0.path.parent().unwrap().join("LICENSE").exists());
        assert!(!fs::read_dir(&stage.0).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-")
        }));
    }
    #[cfg(unix)]
    #[test]
    fn cancellation_while_server_stalls_cleans_staging_promptly() {
        let (stage, mut config, file) = fixture("cancel-body");
        let (mirror, _) = server(false, true);
        config["export"]["ffmpeg"]["download_source"] = json!("mirror");
        config["export"]["ffmpeg"]["mirror_base_url"] = json!(mirror);
        let m = test_manifest("http://127.0.0.1:1");
        let cancel = stage.0.join("cancel");
        let path = cancel.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(300));
            fs::write(path, b"").unwrap();
        });
        let start = Instant::now();
        assert!(
            install_with_manifest(&config, &file, "mp4", true, Some(&cancel), false, &m)
                .unwrap_err()
                .to_string()
                .contains("cancelled")
        );
        assert!(start.elapsed() < Duration::from_millis(1500));
        assert!(!fs::read_dir(&stage.0).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-")
        }));
    }
}
