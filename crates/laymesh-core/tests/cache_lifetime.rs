use laymesh_core::{model::Host, session::CompileSession};

// Cross the former capacity with distinct short paragraphs. A huge single
// paragraph instead benchmarks line breaking and makes debug CI impractical.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "large capacity regression runs optimized in scripts/test-contracts.py"
)]
fn large_text_document_reuses_layout_and_releases_removed_content() {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    let mut session = CompileSession::new(false);
    session.sync_host(&host, 1).unwrap();
    let label = "AV office incremental layout ".repeat(8);
    let labels_per_page = 32;
    let mut pages = vec![];
    let mut revision = 1;
    while session.metrics().text_layout_cache_bytes <= 16 * 1024 * 1024 {
        assert!(
            pages.len() < 256,
            "fixture did not cross the former capacity"
        );
        let file = format!("/page-{}.lay", pages.len());
        let source = format!(
            "style {{ .label {{color:#224466}} }}\np=canvas(size=(100,100))\n\
             for i in range({labels_per_page}) {{\n\
               p.add(text(\"{label}{} \" + str(i),class=\"label\",font_family=\"/font.ttf\",font_size=3mm))\n\
             }}",
            pages.len()
        );
        revision += 1;
        session
            .apply_changes(revision, [(file.clone(), Some(source.as_bytes().to_vec()))])
            .unwrap();
        let scene = session.evaluate(&source, &file, None).unwrap();
        let live = session.metrics().text_layout_cache_bytes;
        assert!(live > 0, "layout cache empty: {:?}", scene.warnings);
        if pages.len() % 32 == 0 {
            eprintln!("page {}: {live} text layout bytes", pages.len());
        }
        pages.push((file, source));
    }
    let bytes = session.metrics().text_layout_cache_bytes;
    eprintln!(
        "{} pages retain {bytes} estimated text-layout bytes",
        pages.len()
    );
    for (file, source) in &pages {
        let recolored = source.replace("#224466", "#55aa88");
        revision += 1;
        session
            .apply_changes(
                revision,
                [(file.clone(), Some(recolored.as_bytes().to_vec()))],
            )
            .unwrap();
        session.evaluate(&recolored, file, None).unwrap();
        let metrics = session.metrics();
        assert_eq!(metrics.rebuilt_text_layouts, 0, "{file}: {metrics:?}");
        assert_eq!(
            metrics.reused_text_layouts, labels_per_page,
            "{file}: {metrics:?}"
        );
        assert_eq!(metrics.text_layout_cache_bytes, bytes);
    }
    session.retain_files(&Default::default());
    assert_eq!(session.metrics().text_layout_cache_bytes, 0);
    session.dispose();
    assert_eq!(session.metrics().cache_bytes, 0);
}
