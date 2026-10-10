//! Generate deterministic native samples for browser/WASM conformance.
use laymesh_core::{Loc, animation::css, engine::compile_source, model::Host};
use serde_json::json;
fn main() {
    let css = include_str!("../../../tests/animation/vendor/animate.css");
    let (_, sheet) = css::extract(css, "/animate.css", Loc::default(), None).unwrap();
    let names: Vec<_> = sheet
        .keyframes
        .iter()
        .filter(|k| !k.prefixed)
        .map(|k| k.name.clone())
        .collect();
    let mut source = "p=canvas(size=(100,80),stylesheet=\"animate.css\")\n".to_string();
    for (i, name) in names.iter().enumerate() {
        source += &format!(
            "a{i}=p.add(rect(size=(25.4mm,19.05mm),fill=\"#d84436\"),offset=(30,25))\ns{i}=p.step(name=\"{name}\",hold=0s)\ns{i}.animate(a{i},class=\"animate__animated animate__{name}\")\n"
        );
    }
    let host = Host {
        files: [("/animate.css".into(), css.as_bytes().to_vec())]
            .into_iter()
            .collect(),
        ..Host::default()
    };
    let scene = compile_source(&source, "/main.lay", host).unwrap();
    let a = scene.animation.unwrap();
    let cases:Vec<_>=a.normal.steps.iter().enumerate().map(|(i,s)|{let target=format!("a{i}");json!({"name":s.name,"target":target,"start":s.start,"duration":s.duration,"samples":([0.,0.07,0.2,0.4,0.5,0.65,0.85,0.99,1.].map(|fraction|{let t=s.duration*fraction;json!({"time":t,"native":a.normal.sample(s.start+t)[&target]})}) )})}).collect();
    println!("{}", json!({"source":source,"css":css,"cases":cases}));
}
