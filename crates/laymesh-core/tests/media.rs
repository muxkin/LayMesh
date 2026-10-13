use laymesh_core::{document::plan_source, engine::compile_source, model::Host};
fn wave() -> Vec<u8> {
    let mut b = b"RIFF".to_vec();
    b.extend(76u32.to_le_bytes());
    b.extend(b"WAVEfmt ");
    b.extend(16u32.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(1u16.to_le_bytes());
    b.extend(10u32.to_le_bytes());
    b.extend(20u32.to_le_bytes());
    b.extend(2u16.to_le_bytes());
    b.extend(16u16.to_le_bytes());
    b.extend(b"data");
    b.extend(40u32.to_le_bytes());
    b.extend([0u8; 40]);
    b
}
fn boxed(kind: &[u8], b: Vec<u8>) -> Vec<u8> {
    let mut a = ((8 + b.len()) as u32).to_be_bytes().to_vec();
    a.extend(kind);
    a.extend(b);
    a
}
fn movie() -> Vec<u8> {
    let mut header = vec![0u8; 20];
    header[12..16].copy_from_slice(&1000u32.to_be_bytes());
    header[16..20].copy_from_slice(&3000u32.to_be_bytes());
    let mut track = vec![0u8; 84];
    track[76..80].copy_from_slice(&(320u32 << 16).to_be_bytes());
    track[80..84].copy_from_slice(&(180u32 << 16).to_be_bytes());
    let mut handler = vec![0u8; 12];
    handler[8..12].copy_from_slice(b"vide");
    let mut trak = boxed(b"tkhd", track);
    trak.extend(boxed(b"mdia", boxed(b"hdlr", handler)));
    let mut moov = boxed(b"mvhd", header);
    moov.extend(boxed(b"trak", trak));
    let mut b = boxed(b"ftyp", b"isom".to_vec());
    b.extend(boxed(b"moov", moov));
    b
}
fn host() -> Host {
    Host {
        files: [
            ("/assets/music.wav".into(), wave()),
            ("/assets/movie.mp4".into(), movie()),
        ]
        .into(),
        ..Host::default()
    }
}
#[test]
fn finite_media_can_define_a_page_timeline() {
    let s=compile_source("p=canvas(size=(100,60))\nv=p.add(video(src=\"assets/movie.mp4\"),size=(80,45))\np.play(target=v,trim_start=500ms,trim_end=2s)\np.play(audio(\"assets/music.wav\"),volume=0.25)","/main.lay",host()).unwrap();
    assert_eq!(s.media.assets.len(), 2);
    assert_eq!(s.media.cues[0].trim_start, 0.5);
    assert_eq!(s.animation.unwrap().normal.duration, 2.);
    assert!(s.nodes[0]["mediaId"].is_string());
}
#[test]
fn step_and_click_bindings_remain_distinct() {
    let s=compile_source("p=canvas(size=(100,60))\nv=p.add(video(src=\"assets/movie.mp4\"),size=(80,45))\ns=p.step(duration=1s,hold=0s)\ns.play(v,delay=200ms)","/main.lay",host()).unwrap();
    assert_eq!(s.media.cues[0].step, Some(0));
    assert_eq!(
        s.media.cues[0].start(s.animation.as_ref(), false, false),
        Some(0.2)
    );
    assert_eq!(s.animation.as_ref().unwrap().normal.duration, 3.2);
    let s=compile_source("p=canvas(size=(100,60))\nv=p.add(video(src=\"assets/movie.mp4\"),size=(80,45))\np.play(v,trigger=\"click\",export_start=2s)","/main.lay",host()).unwrap();
    assert_eq!(s.animation.as_ref().unwrap().normal.duration, 5.);
    assert_eq!(
        s.media.cues[0].start(s.animation.as_ref(), false, false),
        Some(2.)
    );
}
#[test]
fn missing_click_export_time_does_not_invent_a_schedule() {
    let s=compile_source("p=canvas(size=(100,60))\nv=p.add(video(src=\"assets/movie.mp4\"),size=(80,45))\np.play(v,trigger=\"click\")\ns=p.step(duration=1s,hold=0s)","/main.lay",host()).unwrap();
    assert_eq!(
        s.media.cues[0].start(s.animation.as_ref(), false, false),
        None
    );
    assert_eq!(s.animation.unwrap().normal.duration, 1.);
}
#[test]
fn invalid_playback_is_rejected_at_its_source() {
    for call in [
        "p.play(a,volume=2)",
        "p.play(a,delay=2)",
        "p.play(a,trim_start=2s)",
        "p.play(a,loop=1)",
        "p.play(a,trigger=\"click\")",
        "p.play(a,unknown=true)",
        "p.play(a,loop=true)",
    ] {
        let code = format!("p=canvas(size=(100,60))\na=audio(src=\"assets/music.wav\")\n{call}");
        let e = compile_source(&code, "/main.lay", host()).unwrap_err();
        assert_eq!(e.code, "E_MEDIA", "{call}: {e}");
        assert_eq!(e.loc.line, 3);
    }
}
#[test]
fn independent_video_instances_get_independent_bindings() {
    let code = "p=canvas(size=(100,60))\na=video(src=\"assets/movie.mp4\")\nv=p.add(a,size=(40,25))\nw=p.add(a,size=(40,25),offset=(45,0))\np.play(v)\np.play(w,trigger=\"click\",export_start=1s)";
    let s = compile_source(code, "/main.lay", host()).unwrap();
    assert_ne!(s.media.cues[0].target, s.media.cues[1].target);
    let e = compile_source(&(code.to_owned() + "\np.play(v)"), "/main.lay", host()).unwrap_err();
    assert_eq!(e.code, "E_MEDIA");
}
#[test]
fn background_music_is_shared_across_lazy_pages() {
    let mut h = host();
    h.files.insert(
        "/one.lay".into(),
        b"p=canvas(size=(100,60))\ns=p.step(duration=1s,hold=0s)".to_vec(),
    );
    let d=plan_source("d=document()\nd.play(audio(src=\"assets/music.wav\"),loop=true)\nd.add(page(src=\"one.lay\",id=\"a\"))\nd.add(page(src=\"one.lay\",id=\"b\"))","/deck.laydoc",h.clone()).unwrap();
    let a = laymesh_core::document::compile_page(d.page(1).unwrap(), h.clone()).unwrap();
    let b = laymesh_core::document::compile_page(d.page(2).unwrap(), h).unwrap();
    assert_eq!(a.media.cues[0].scope, "document");
    assert_eq!(a.animation.unwrap().normal.duration, 1.);
    let id = a.media.assets.keys().next().unwrap();
    assert!(std::sync::Arc::ptr_eq(
        &a.media.assets[id].data,
        &b.media.assets[id].data
    ));
}
