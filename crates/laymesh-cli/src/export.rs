use std::{
    fs,
    io::{self, Write},
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
pub fn write_atomic(output: &Path, data: &[u8]) -> io::Result<()> {
    let mut name = output.as_os_str().to_os_string();
    name.push(format!(
        ".laymesh-{}-{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let temporary = Path::new(&name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    let result = (|| {
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, output)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
