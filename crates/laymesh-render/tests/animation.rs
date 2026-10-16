use kurbo::Rect;
use laymesh_core::{engine::compile_source, model::Host};
use laymesh_render::{animation::AnimationRenderer, raster, to_display_list};
#[test]
fn animation_opacity_replaces_instance_and_static_is_restored() {
    let scene=compile_source("style {@keyframes f {from{opacity:0}to{opacity:1}}}\np=canvas(size=(20,20),background=\"none\")\na=p.add(rect(size=(10,10),fill=\"#ff0000\",border_color=\"none\"),offset=(5,5),opacity=0.4)\ns=p.step()\ns.animate(a,name=\"f\",duration=1s,easing=\"linear\",fill=\"both\")","/main.lay",Host::default()).unwrap();
    let list = to_display_list(&scene).unwrap();
    let rect = Rect::new(0., 0., 20., 20.);
    let baseline = raster::render_region(&list, rect, 40, 40).unwrap();
    let mut r = AnimationRenderer::new(list).unwrap();
    let middle = r.render(rect, 40, 40, Some(0.5), false).unwrap();
    assert!((middle.rgba[(20 * 40 + 20) * 4 + 3] as i32 - 128).abs() <= 1);
    assert_eq!(
        r.render(rect, 40, 40, None, false).unwrap().rgba,
        baseline.rgba
    );
}
#[test]
fn all_97_effects_render_and_seek_reproducibly() {
    let lib = include_str!("../../../tests/animation/vendor/animate.css");
    let (_, sheet) =
        laymesh_core::animation::css::extract(lib, "/animate.css", Default::default(), None)
            .unwrap();
    for k in sheet.keyframes.iter().filter(|k| !k.prefixed) {
        let src = format!(
            "p=canvas(size=(70,55),stylesheet=\"animate.css\")\na=p.add(rect(size=(25.4,19.05),fill=\"#3978d4\",border_color=\"none\"),offset=(22,18))\ns=p.step()\ns.animate(a,class=\"animate__animated animate__{}\")",
            k.name
        );
        let scene = compile_source(
            &src,
            "/main.lay",
            Host {
                files: [("/animate.css".into(), lib.as_bytes().to_vec())]
                    .into_iter()
                    .collect(),
                ..Host::default()
            },
        )
        .unwrap();
        let duration = scene.animation.as_ref().unwrap().normal.steps[0].duration;
        let mut r = AnimationRenderer::new(to_display_list(&scene).unwrap()).unwrap();
        let rect = Rect::new(0., 0., 70., 55.);
        let a = r
            .render(rect, 140, 110, Some(duration * 0.4), false)
            .unwrap();
        for t in [0., 0.8, 0.2, 1.] {
            r.render(rect, 140, 110, Some(duration * t), false)
                .unwrap_or_else(|e| panic!("{}: {e}", k.name));
        }
        let b = r
            .render(rect, 140, 110, Some(duration * 0.4), false)
            .unwrap();
        assert_eq!(a.rgba, b.rgba, "{}", k.name);
    }
}
#[test]
fn plot_reveal_preserves_axes_and_restores_all_data() {
    let source = "p=canvas(size=(70,50),background=\"none\")\nc=plot(size=(60,40),plot_area=box(offset=(10,5),size=(45,25)),x=axis(range=(0,2)),y=axis(range=(0,3)))\nc.line(x=[0,1,2],y=[1,2,1],color=\"#ff0000\")\na=p.add(c,offset=(5,5))\ns=p.step()\ns.animate(a,name=\"reveal\",duration=1s,fill=\"both\")";
    let scene = compile_source(source, "/main.lay", Host::default()).unwrap();
    let list = to_display_list(&scene).unwrap();
    let rect = Rect::new(0., 0., 70., 50.);
    let full = raster::render_region(&list, rect, 280, 200).unwrap();
    let mut renderer = AnimationRenderer::new(list).unwrap();
    let hidden = renderer.render(rect, 280, 200, Some(0.), false).unwrap();
    let complete = renderer.render(rect, 280, 200, Some(1.), false).unwrap();
    assert_eq!(full.rgba, complete.rgba);
    let red = |rgba: &[u8]| {
        rgba.chunks_exact(4)
            .filter(|p| p[0] > 200 && p[1] < 80 && p[2] < 80 && p[3] > 128)
            .count()
    };
    assert!(red(&full.rgba) > 20);
    assert_eq!(red(&hidden.rgba), 0);
    assert!(hidden.rgba.chunks_exact(4).any(|p| p[3] > 128));
}

#[test]
fn parent_reveal_keeps_child_animation_ids_and_sibling_opacity() {
    use kurbo::Shape;
    use laymesh_render::display_list::{DrawKind, DrawNode, Paint};
    let scene=compile_source("style {@keyframes fade {from{opacity:0}to{opacity:1}}}\np=canvas(size=(20,20),background=\"none\")\na=p.add(rect(size=(20,20)))\nb=p.add(rect(size=(4,8)))\ns=p.step()\ns.animate(a,name=\"reveal\",duration=1s,fill=\"both\",easing=\"linear\")\ns.animate(b,name=\"fade\",duration=1s,fill=\"both\",easing=\"linear\")","/nested.lay",Host::default()).unwrap();
    let mut list = to_display_list(&scene).unwrap();
    let clips = &scene.animation.as_ref().unwrap().normal.steps[0].clips;
    let shape = |rect: Rect, rgba| {
        DrawNode::new(DrawKind::Path {
            path: rect.to_path(0.01),
            fill: Paint::Solid { rgba },
            stroke: None,
            even_odd: false,
        })
    };
    let mut red = shape(Rect::new(1., 1., 4., 9.), [1., 0., 0., 1.]);
    red.name = clips[1].target.clone();
    let blue = shape(Rect::new(5., 1., 9., 9.), [0., 0., 1., 1.]);
    let mut group = DrawNode::new(DrawKind::Group {
        children: vec![red, blue],
        clip: None,
        mask: None,
    });
    group.name = clips[0].target.clone();
    group.hint = serde_json::json!({"animationSize":[20.,20.]});
    list.nodes = vec![group];
    let mut renderer = AnimationRenderer::new(list).unwrap();
    let frame = renderer
        .render(Rect::new(0., 0., 20., 20.), 20, 20, Some(0.5), false)
        .unwrap();
    assert!((frame.rgba[(4 * 20 + 2) * 4 + 3] as i32 - 128).abs() <= 1);
    assert_eq!(frame.rgba[(4 * 20 + 7) * 4 + 3], 255);
}

#[test]
fn draw_endpoints_follow_front_and_finish_exactly() {
    for shape in ["triangle", "open", "stealth", "dot", "diamond", "bar"] {
        for stroke in ["solid", "dashed"] {
            let source = format!(
                "p=canvas(size=(80,40),background=\"none\")\na=p.add(line(length=50mm,line_width=1mm,line_style=\"{stroke}\",start_head=head(shape=\"{shape}\",size=(5mm,4mm)),end_head=head(shape=\"{shape}\",size=(5mm,4mm))),offset=(10,20))\ns=p.step()\ns.animate(a,name=\"draw\",duration=1s,easing=\"linear\",fill=\"both\")"
            );
            let scene = compile_source(&source, "/draw.lay", Host::default()).unwrap();
            let list = to_display_list(&scene).unwrap();
            let rect = Rect::new(0., 0., 80., 40.);
            let full = raster::render_region(&list, rect, 320, 160).unwrap();
            let mut renderer = AnimationRenderer::new(list).unwrap();
            let start = renderer.render(rect, 320, 160, Some(0.), false).unwrap();
            assert!(start.rgba.chunks_exact(4).all(|p| p[3] == 0), "{shape}");
            let mid = renderer.render(rect, 320, 160, Some(0.5), false).unwrap();
            let right = mid
                .rgba
                .chunks_exact(4)
                .enumerate()
                .filter(|(_, p)| p[3] > 100)
                .map(|(i, _)| i % 320)
                .max()
                .unwrap();
            assert!((125..155).contains(&right), "{shape}: front={right}");
            assert_eq!(
                full.rgba,
                renderer
                    .render(rect, 320, 160, Some(1.), false)
                    .unwrap()
                    .rgba,
                "{shape}"
            );
        }
    }
}
#[test]
fn unsuitable_draw_warns_once_without_changing_timing_or_other_tracks() {
    for shape in [
        "rect(size=(10,10),fill=\"#ff0000\")",
        "line(length=0mm,angle=0deg)",
        "line(length=30mm,line_width=0mm)",
        "line(length=30mm,line_color=\"none\")",
    ] {
        let src = format!(
            "style {{@keyframes mixed {{from{{-laymesh-draw:0;opacity:0}}to{{-laymesh-draw:1;opacity:1}}}}}}\np=canvas(size=(80,40))\na=p.add({shape})\ns=p.step(hold=1s)\ns.animate(a,name=\"mixed\",duration=2s,delay=0.5s,fill=\"both\")\ns.animate(a,name=\"reveal\",duration=1s,start=\"after-previous\")"
        );
        let scene = compile_source(&src, "/warn.lay", Host::default()).unwrap();
        assert_eq!(
            scene
                .warnings
                .iter()
                .filter(|w| w.code == "W_ANIMATION")
                .count(),
            1,
            "{shape}"
        );
        let timeline = &scene.animation.as_ref().unwrap().normal;
        assert_eq!(timeline.steps[0].duration, 3.5);
        assert_eq!(timeline.steps[0].clips[1].delay, 2.5);
        assert!(
            timeline.steps[0].clips[0]
                .tracks
                .iter()
                .any(|t| t.property == "opacity")
        );
        let mut r = AnimationRenderer::new(to_display_list(&scene).unwrap()).unwrap();
        r.render(Rect::new(0., 0., 80., 40.), 160, 80, Some(0.5), false)
            .unwrap();
    }
}

#[test]
fn draw_closed_compound_and_nested_transforms_restore_static() {
    for object in [
        "path(commands=[move_to(0,0),line_to(40,0),line_to(40,20),close()],fill=\"none\",border_color=\"#000\",border_width=1mm)",
        "line(length=40mm,line_width=2mm,line_style=\"double\")",
        "path(commands=[move_to(0,10),cubic_to(c1x=10,c1y=0,c2x=25,c2y=20,x=40,y=10)],fill=\"none\",border_color=\"#000\",end_head=head(size=(6mm,4mm)))",
        "path(commands=[move_to(0,0),line_to(40,0),move_to(0,20),line_to(40,20)],fill=\"none\",border_color=\"#000\",start_head=head(size=(6mm,4mm)),end_head=head(size=(6mm,4mm)))",
    ] {
        let source = format!(
            "p=canvas(size=(100,60),background=\"none\")\ng=group()\na=g.add({object})\nb=p.add(g,offset=(15,15),size=(65,30),rotation=15deg)\ns=p.step()\ns.animate(b,name=\"draw\",duration=1s,fill=\"both\",easing=\"linear\")"
        );
        let scene = compile_source(&source, "/nested-draw.lay", Host::default()).unwrap();
        let list = to_display_list(&scene).unwrap();
        let rect = Rect::new(0., 0., 100., 60.);
        let full = raster::render_region(&list, rect, 400, 240).unwrap();
        let mut r = AnimationRenderer::new(list).unwrap();
        for time in [0., 0.25, 0.5, 0.75] {
            r.render(rect, 400, 240, Some(time), false).unwrap();
        }
        assert_eq!(
            full.rgba,
            r.render(rect, 400, 240, Some(1.), false).unwrap().rgba,
            "{object}"
        );
    }
}
