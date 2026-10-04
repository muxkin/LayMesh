use laymesh_core::{
    colormap::{self, Colormap},
    engine::compile_source,
    model::Host,
};
fn check(body: &str) {
    let source = format!(
        "page=canvas(size=(100mm,80mm))\nfunction check(ok) {{ if not ok {{ return failed_assertion }} return true }}\n{body}"
    );
    compile_source(&source, "/main.lay", Host::default()).unwrap();
}
#[test]
fn ordered_values_copy_updates_and_membership() {
    check(
        r#"
key="z"
a={key:1mm,"a":{"x":2},"z":1cm,"":false}
b=a
b["a"]["x"]=4
b["extra"]=null
ok=check(a["a"]["x"]==2 and b["a"]["x"]==4)
ok=check(a["z"]==10mm and a.keys()==["z","a",""])
ok=check(len(a)==3 and a.get("missing")==null and a.get("missing",3)==3)
c=a.update({"z":2mm,"next":true})
ok=check(c.keys()==["z","a","","next"] and a["z"]==1cm)
ok=check("next" in c and "next" not in a)
ok=check({}.values()==[] and dict().items()==[])
"#,
    );
}
#[test]
fn snapshot_loops_unpack_enumerate_zip_and_control_flow() {
    check(
        r#"
d={"b":2,"a":3}
seen=[]
for k in d { seen=append(seen,k) d["new"]=9 }
ok=check(seen==["b","a"])
seen=[]
for i,(k,v) in enumerate(d.items(),start=4) {
 if k=="a" {continue}
 seen=append(seen,(i,k,v))
 if k=="new" {break}
}
ok=check(seen==[(4,"b",2),(6,"new",9)])
seen=[]
for a,b,c in zip([1,2,3],[4,5],[6,7,8]) {seen=append(seen,(a,b,c))}
ok=check(seen==[(1,4,6),(2,5,7)] and zip()==[])
for _ in {} {ok=check(false)}
"#,
    );
}
#[test]
fn json_and_table_order_remain_source_order() {
    let mut h = Host::default();
    h.files.insert(
        "/dict.json".into(),
        br#"{"z":{"a":[true,null,"x"]},"a":{}}"#.to_vec(),
    );
    h.files
        .insert("/data.json".into(), br#"{"z":[1,2],"a":[3,4]}"#.to_vec());
    h.files
        .insert("/data.csv".into(), b"z,a\n1,3\n2,4\n".to_vec());
    let source = r#"page=canvas(size=(20,20))
d=dict(src="dict.json")
t=table(src="data.json")
c=table(src="data.csv")
if d.keys()!=["z","a"] or t.keys()!=["z","a"] or c.keys()!=["z","a"] {bad=missing}
if d["z"]["a"]!=[true,null,"x"] {bad=missing}
"#;
    compile_source(source, "/main.lay", h).unwrap();
}
#[test]
fn invalid_keys_unpacking_arguments_and_readonly_imports() {
    for (body, code) in [
        ("d={1:2}", "E_TYPE"),
        ("d={}\nx=d[1]", "E_TYPE"),
        ("d={}\nx=d[\"missing\"]", "E_INDEX"),
        ("d={}\nd[\"missing\"][\"nested\"]=1", "E_INDEX"),
        ("d={}\nx=d.update([])", "E_TYPE"),
        ("d={}\nx=d.keys(1)", "E_ARG"),
        ("for a,b in [(1,2,3)] {}", "E_ARG"),
        ("for a,b in [1] {}", "E_TYPE"),
        ("for a,a in [(1,2)] {}", "E_SYNTAX"),
        ("x=enumerate([1],start=1mm)", "E_UNIT"),
        ("x=zip([1],other=[2])", "E_ARG"),
        ("x=enumerate(range(10001))", "E_LIMIT"),
        ("x=palette(\"viridis\",n=-1)", "E_ARG"),
        ("x=palette(n=10001)", "E_ARG"),
        ("x=cmap(\"missing\")", "E_COLOR"),
        ("x=cmap_names(category=\"missing\")", "E_COLOR"),
        (
            "p=plot(size=(20,20))\np.heatmap(z=[[1,2],[3,4]],cmap=\"missing\")",
            "E_PLOT",
        ),
        (
            "p=plot(size=(20,20))\np.heatmap(z=[[1,2],[3,4]],cmap=42)",
            "E_PLOT",
        ),
        (
            "p=plot(size=(20,20))\np.heatmap(z=[[1,2],[3,4]],cmap=[\"red\"])",
            "E_PLOT",
        ),
    ] {
        let error = compile_source(
            &format!("page=canvas(size=(20,20))\n{body}"),
            "/main.lay",
            Host::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, code, "{body}: {error}");
        assert!(error.loc.line >= 2);
    }
    let mut h = Host::default();
    h.files
        .insert("/module.lay".into(), b"export d={\"x\":1}".to_vec());
    let error = compile_source(
        "import {d} from \"module.lay\"\npage=canvas(size=(20,20))\nd[\"x\"]=2",
        "/main.lay",
        h,
    )
    .unwrap_err();
    assert_eq!(error.code, "E_BINDING");
}
#[test]
fn palettes_have_expected_sampling_and_value_methods() {
    check(
        r#"
cm=cmap("viridis")
rev=cm.reversed()
ok=check(cm.name=="viridis" and rev.name=="viridis_r" and rev.reversed().name==cm.name)
ok=check(cm.sample(-1)==cm.sample(0) and cm.sample(2)==cm.sample(1))
ok=check(len(cmap_names())==87 and len(cmap_names(reversed=true))==174)
ok=check(len(palette())==10 and palette("tab10",12)[10]==palette()[0])
ok=check(cm.colors(0)==[] and cm.colors(1)[0]==cm.sample(0.5))
cyclic=cmap("twilight")
ok=check(cyclic.colors(5)[4]==cyclic.sample(0.8))
ok=check(cyclic.colors(1)[0]==cyclic.sample(0))
colors=palette("tab10",2)
series={"A":[1,2,3],"B":[2,1,4]}
p=plot(size=(70,55),style=plot_style(colors=colors))
for i,(name,ys) in enumerate(series.items()) {p.line(x=[0,1,2],y=ys,label=name,color=colors[i])}
page.add(p)
"#,
    );
    assert_eq!(
        Colormap::get("viridis").unwrap().sample(0.).unwrap().css(),
        "#440154"
    );
    assert_eq!(
        Colormap::get("tab10").unwrap().sample(0.).unwrap().css(),
        "#1f77b4"
    );
    assert_eq!(Colormap::get("rdbu").unwrap().name(), "RdBu");
    assert!(Colormap::get("gray").unwrap().sample(f64::NAN).is_err());
}
#[test]
fn every_runtime_lut_entry_matches_pinned_data_including_reversals() {
    let data: serde_json::Value = serde_json::from_str(include_str!("../cmaps.json")).unwrap();
    for p in data["presets"].as_array().unwrap() {
        for (suffix, field) in [("", "colors"), ("_r", "reverse_colors")] {
            let name = format!("{}{suffix}", p["name"].as_str().unwrap());
            let cm = Colormap::get(&name).unwrap();
            let raw = p[field].as_str().unwrap();
            let n = raw.len() / 6;
            for i in 0..n {
                // Center each native bin; do not depend on endpoint rounding.
                assert_eq!(
                    cm.sample((i as f64 + 0.5) / n as f64).unwrap().css(),
                    format!("#{}", &raw[i * 6..i * 6 + 6]),
                    "{name}[{i}]"
                );
            }
        }
    }
    assert_eq!(colormap::VERSION, "3.11.2");
}

#[test]
fn dictionary_geometry_values_replay_with_inherited_style() {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    let scene = compile_source(
        r##"
style { group.grow { font-size:20pt; font-family:"/font.ttf"; } }
page=canvas(size=(180,100))
g=group()
a=g.add(text("AAAA",font_family="/font.ttf"))
q={"width":a.bounds.width,"anchor":a.bounds.bottom_right,
   "paint":rgb(18,52,86),"node":rect(size=(1,1))}
g.add(rect(size=(1mm,1mm),fill=q["paint"]),size=(q["width"],1mm),target=q["anchor"])
g.add(q["node"],offset=(0,30mm))
parent=group(class="grow")
parent.add(g)
page.add(parent)
page.add(g,offset=(0,60))
"##,
        "/main.lay",
        host,
    )
    .unwrap();
    let json = serde_json::to_value(scene).unwrap();
    let large = &json["nodes"][0]["children"][0]["children"];
    let small = &json["nodes"][1]["children"];
    for nodes in [large, small] {
        let width = nodes[0]["width"].as_f64().unwrap();
        assert!(
            (nodes[1]["width"].as_f64().unwrap() - width).abs() < 1e-6,
            "{nodes}"
        );
        assert!((nodes[1]["x"].as_f64().unwrap() - width).abs() < 1e-6);
        assert_eq!(nodes[1]["fill"], "#123456");
        assert_eq!(nodes[2]["y"], 30.);
    }
    assert_ne!(large[0]["width"], small[0]["width"]);
}

#[test]
fn cmap_objects_names_and_shared_scale_produce_the_same_scene() {
    let source = r#"
page=canvas(size=(150,100))
cm=CMAP
s=color_scale(cmap=cm,vmin=-1,vmax=1)
p=plot(size=(100,70))
p.heatmap(z=[[-1,0],[0,1]],color_scale=s)
p.heatmap(z=[[-1,0],[0,1]],cmap=cm,vmin=-1,vmax=1)
p.scatter(x=[0,1],y=[0,1],c=[-1,1],color_scale=s)
p.contourf(z=[[-1,0],[0,1]],levels=[-1,0],color_scale=s)
p.colorbar(scale=s)
page.add(p)
page.add(colorbar(scale=s,length=50))
"#;
    let compile = |expr| {
        let mut scene = serde_json::to_value(
            compile_source(&source.replace("CMAP", expr), "/main.lay", Host::default()).unwrap(),
        )
        .unwrap();
        // Diagnostic offsets differ because the two source expressions have different lengths.
        scene.as_object_mut().unwrap().remove("warnings");
        scene
    };
    assert_eq!(compile("cmap(\"coolwarm_r\")"), compile("\"coolwarm_r\""));
}
