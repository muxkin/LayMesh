use std::{path::PathBuf, process::Command};
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "laydoc-中文-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("a.lay"),
            "p=canvas(size=(25.4,12.7),background=\"#fff\")\np.add(rect(size=(8,8),fill=\"#f00\"))",
        )
        .unwrap();
        std::fs::write(root.join("b.lay"),"p=canvas(size=(2,3),unit=\"cm\",background=\"#fff\")\np.add(rect(size=(1,1),fill=\"#00f\"))").unwrap();
        std::fs::write(root.join("report.laydoc"),"doc=document(name=\"报告\")\nfor p in [page(src=\"a.lay\",id=\"a\"),page(src=\"b.lay\",id=\"b\")] { doc.add(p) }").unwrap();
        Self(root)
    }
    fn cli(&self, args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_laymesh"))
            .current_dir(&self.0)
            .env("LAYMESH_NO_SYSTEM_FONTS", "1")
            .args(args)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str]) -> std::process::Output {
        let output = self.cli(args);
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
#[test]
fn whole_pdf_selection_and_numbered_outputs_preserve_pages() {
    let t = Temp::new();
    t.ok(&["validate", "report.laydoc"]);
    let info: serde_json::Value =
        serde_json::from_slice(&t.ok(&["inspect", "report.laydoc", "--json"]).stdout).unwrap();
    assert_eq!(info["document"]["pages"][1]["id"], "b");
    t.ok(&[
        "render",
        "report.laydoc",
        "-o",
        "report.pdf",
        "--preview-output",
        "preview.jpg",
        "--preview-page",
        "2",
    ]);
    let pdf = std::fs::read(t.0.join("report.pdf")).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 2"));
    t.ok(&[
        "render",
        "report.laydoc",
        "-o",
        "selected.png",
        "--page",
        "2",
        "--dpi",
        "96",
    ]);
    t.ok(&["render", "b.lay", "-o", "standalone.png", "--dpi", "96"]);
    assert_eq!(
        std::fs::read(t.0.join("selected.png")).unwrap(),
        std::fs::read(t.0.join("standalone.png")).unwrap()
    );
    t.ok(&[
        "render",
        "report.laydoc",
        "-o",
        "batch.png",
        "--all-pages",
        "--dpi",
        "96",
    ]);
    assert!(t.0.join("batch-001.png").is_file());
    assert!(t.0.join("batch-002.png").is_file());
    assert!(!t.0.join("batch.png").exists());
    assert_eq!(
        std::fs::read(t.0.join("batch-002.png")).unwrap(),
        std::fs::read(t.0.join("selected.png")).unwrap()
    );
}
#[test]
fn errors_never_export_an_implicit_first_page_or_replace_existing_outputs() {
    let t = Temp::new();
    std::fs::write(t.0.join("keep.png"), b"existing").unwrap();
    for args in [
        vec!["render", "report.laydoc", "-o", "keep.png"],
        vec!["render", "report.laydoc", "-o", "keep.png", "--page", "0"],
        vec!["render", "report.laydoc", "-o", "keep.png", "--page", "3"],
        vec![
            "render",
            "report.laydoc",
            "-o",
            "keep.png",
            "--page",
            "1",
            "--all-pages",
        ],
        vec![
            "render",
            "report.laydoc",
            "-o",
            "keep.png",
            "--preview-output",
            "keep.png",
            "--page",
            "1",
        ],
    ] {
        assert!(!t.cli(&args).status.success());
        assert_eq!(std::fs::read(t.0.join("keep.png")).unwrap(), b"existing");
    }
    std::fs::write(t.0.join("b.lay"), "invalid=missing()").unwrap();
    assert!(
        !t.cli(&[
            "render",
            "report.laydoc",
            "-o",
            "batch.png",
            "--all-pages",
            "--dpi",
            "96"
        ])
        .status
        .success()
    );
    assert!(!t.0.join("batch-001.png").exists());
    t.ok(&["render", "report.laydoc", "-o", "a.svg", "--page", "1"]);
    assert!(!t.cli(&["validate", "report.laydoc"]).status.success());
}
