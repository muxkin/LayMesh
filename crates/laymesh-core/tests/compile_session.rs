use laymesh_core::{engine::compile_source, model::Host, session::CompileSession};
use serde_json::Value;
fn scene(value: &laymesh_core::model::Scene) -> Value {
    fn strip(v: &mut Value) {
        match v {
            Value::Object(m) => {
                m.remove("_identity");
                for v in m.values_mut() {
                    strip(v)
                }
            }
            Value::Array(a) => {
                for v in a {
                    strip(v)
                }
            }
            _ => {}
        }
    }
    let mut value = serde_json::to_value(value).unwrap();
    strip(&mut value);
    value
}
fn compare(session: &mut CompileSession, source: &str, revision: u64) {
    session
        .apply_changes(
            revision,
            [("/main.lay".into(), Some(source.as_bytes().to_vec()))],
        )
        .unwrap();
    let actual = session.evaluate(source, "/main.lay", None);
    let expected = compile_source(source, "/main.lay", Host::default());
    if revision == 1 {
        assert!(expected.is_ok(), "baseline must compile: {expected:?}");
    }
    match (actual, expected) {
        (Ok(a), Ok(b)) => assert_eq!(scene(&a), scene(&b), "{source}"),
        (Err(a), Err(b)) => assert_eq!(a, b, "{source}"),
        (a, b) => panic!("different compile results {a:?} / {b:?}"),
    }
}
#[test]
fn mutations_recursion_control_flow_and_error_recovery_match_full() {
    let source = r##"
page=canvas(size=(100,100))
count=0
factor=2
function recur(n) { if n<1 { return factor } return recur(n-1)+1 }
function change(x) { count=count+x return count }
function draw(n) {
 values=()
 for i in range(n) { values=append(values,recur(i)) }
 return values
}
values=draw(4)
a=change(2)
b=change(3)
d={"width":values[2]}
d["width"]=d["width"]+a+b
if count>3 { page.add(rect(size=(d["width"],10),fill="#ff0000")) }
i=0
while i<2 { i=i+1
 page.add(ellipse(size=(i,2))) }
"##;
    let mut session = CompileSession::new(false);
    let edits = [
        source.to_owned(),
        source.to_owned(),
        source.replace("factor=2", "factor=3"),
        source.replace("draw(4)", "draw(5)"),
        source.replace("count>3", "count>9"),
        source.replace("return count", "return count*2"),
        source.replace("change(2)", "change(4)"),
        source.replace("i<2", "i<3"),
        source.replace("count=count+x", "count=count+unknown"),
        source.replace("factor=2", "factor=0"),
        format!("# inserted\n{source}"),
        source.into(),
    ];
    for (i, source) in edits.iter().enumerate() {
        compare(&mut session, source, i as u64 + 1);
    }
    assert!(session.metrics().reused_expressions > 0);
    let last = session.snapshot("/main.lay").unwrap();
    compare(&mut session, "page=canvas(size=(10,10))\na=unknown", 99);
    assert_eq!(last.0, session.snapshot("/main.lay").unwrap().0);
}
#[test]
fn loop_growth_reuses_existing_value_computations() {
    let source = r##"
page=canvas(size=(100,100))
function calc(n) { if n<1 { return 1 } return calc(n-1)+1 }
for i in range(8) { x=calc(i) page.add(rect(size=(x,2))) }
"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    let cold = session.metrics();
    compare(&mut session, source, 2);
    let warm = session.metrics();
    compare(&mut session, &source.replace("range(8)", "range(9)"), 3);
    let grown = session.metrics();
    assert!(warm.reused_expressions >= 8, "{warm:?}");
    assert!(
        warm.evaluated_expressions < cold.evaluated_expressions,
        "{cold:?} / {warm:?}"
    );
    assert!(grown.reused_expressions >= 8, "{grown:?}");
    assert!(
        grown.evaluated_expressions < cold.evaluated_expressions,
        "{cold:?} / {grown:?}"
    );
}
#[test]
fn changed_binding_ownership_and_functions_do_not_replay_stale_values() {
    let source = r##"page=canvas(size=(50,50))
x=2
function f(n) { return x+n }
a=f(3)
page.add(rect(size=(a,10)))"##;
    let mut session = CompileSession::new(false);
    for (i, source) in [
        source.into(),
        source.replace("return x+n", "x=9 return x+n"),
        source.replace("x=2", "x=7"),
        source.replace("return x+n", "return x*n"),
        source.into(),
    ]
    .iter()
    .enumerate()
    {
        compare(&mut session, source, i as u64 + 1);
    }
}
#[test]
fn legacy_budget_is_ignored_and_disposal_closes_the_session() {
    let mut session = CompileSession::new(false);
    session.set_budget(0);
    compare(
        &mut session,
        "page=canvas(size=(30,30))\na=(1+2)*3\npage.add(rect(size=(a,a)))",
        1,
    );
    assert!(session.metrics().cache_bytes > 0);
    session.dispose();
    assert_eq!(session.metrics().cache_bytes, 0);
    assert!(session.snapshot("/main.lay").is_none());
    assert!(
        session
            .evaluate("page=canvas(size=(1,1))", "/main.lay", None)
            .is_err()
    );
}

#[test]
fn font_catalog_reuse_preserves_assets_requests_diagnostics_and_replacement() {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    let source = "page=canvas(size=(100,100))\npage.add(text(\"ABC中\",font_family=[\"/font.ttf\"],font_size=4))";
    let mut session = CompileSession::new(false);
    for revision in 1..=6 {
        if revision == 3 {
            host.files.remove("/font.ttf");
        }
        if revision == 5 {
            host.files.insert(
                "/font.ttf".into(),
                include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
            );
        }
        session.sync_host(&host, revision).unwrap();
        let expected = compile_source(source, "/main.lay", host.clone()).unwrap();
        let actual = session.evaluate(source, "/main.lay", None).unwrap();
        assert_eq!(scene(&actual), scene(&expected), "revision {revision}");
        assert_eq!(
            actual.font_requests, expected.font_requests,
            "revision {revision}"
        );
        assert_eq!(
            session.metrics().font_catalog_builds,
            usize::from(revision % 2 == 1)
        );
    }
}

#[test]
fn read_write_replay_observes_module_updates_and_readonly_bindings() {
    let source = "import {n,f} from \"dep.lay\"\npage=canvas(size=(100,100))\na=f(3)\npage.add(rect(size=(a,10)))";
    let mut host = Host::default();
    let mut session = CompileSession::new(false);
    for (i, module) in [
        "export n=2\nexport function f(x) { return n+x }",
        "export n=2\nexport function f(x) { return n+x }",
        "export n=7\nexport function f(x) { return n+x }",
        "export n=7\nexport function f(x) { n=n+1 return n+x }",
        "export n=7\nexport function f(x) { n=n+1 return n+x }",
    ]
    .iter()
    .enumerate()
    {
        host.files
            .insert("/dep.lay".into(), module.as_bytes().to_vec());
        session.sync_host(&host, i as u64 + 1).unwrap();
        let expected = compile_source(source, "/main.lay", host.clone()).unwrap();
        let actual = session.evaluate(source, "/main.lay", None).unwrap();
        assert_eq!(scene(&actual), scene(&expected));
    }
}

#[test]
fn paint_and_placement_do_not_rebuild_existing_geometry() {
    let source = r##"page=canvas(size=(100,100))
shape=path(commands=[move_to(0,0),cubic_to(10,30,25,-10,40,20)],fill="#ff0000")
page.add(shape,offset=(3,4))"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    assert_eq!(session.metrics().rebuilt_geometry, 1);
    compare(&mut session, &source.replace("#ff0000", "#00ff00"), 2);
    assert_eq!(session.metrics().rebuilt_geometry, 0);
    assert!(session.metrics().reused_geometry + session.metrics().updated_appearance_nodes > 0);
    compare(
        &mut session,
        &source.replace("offset=(3,4)", "offset=(7,8)"),
        3,
    );
    assert_eq!(session.metrics().rebuilt_geometry, 0);
    compare(
        &mut session,
        &source.replace("10,30,25,-10,40,20", "10,35,25,-10,40,20"),
        4,
    );
    assert_eq!(session.metrics().rebuilt_geometry, 1);
}

#[test]
fn cached_definitions_rebase_all_argument_origins_after_whitespace_edits() {
    let source = r##"page=canvas(size=(100,100))
function curve(x) {return path(commands=[move_to(0,0),cubic_to(10,x,25,-10,40,20)],fill="#ff0000")}
shape=curve(30)
page.add(shape,offset=(3,4))"##;
    let mut session = CompileSession::new(false);
    for (i, source) in [
        source.into(),
        source.into(),
        format!("# inserted\n{source}"),
        source.replace("10,x,25,-10,40,20", "10,\n x,25,-10,40,20"),
        source.replace("curve(30)", "curve(35)"),
        source.into(),
    ]
    .iter()
    .enumerate()
    {
        compare(&mut session, source, i as u64 + 1);
    }
}

#[test]
fn incremental_style_context_preserves_cascade_and_parent_inheritance() {
    let source = r##"page=canvas(size=(100,100))
style { .a {color:#ff0000; --w:4mm} .a .b {border-width:var(--w);fill:currentColor} .b {border-color:#000000} .c {color:#00ff00} }
g=group(class="a")
g.add(rect(size=(10,20),class="b"))
g.add(rect(size=(30,10),class="b"))
page.add(g)
page.add(rect(size=(10,10),class="b c"))"##;
    let mut session = CompileSession::new(false);
    for (i, source) in [
        source.into(),
        source.into(),
        source.replace("#ff0000", "#0000ff"),
        source.replace("--w:4mm", "--w:2mm"),
        source.replace("group(class=\"a\")", "group(class=\"c\")"),
        source.into(),
    ]
    .iter()
    .enumerate()
    {
        compare(&mut session, source, i as u64 + 1);
    }
}

#[test]
fn cached_constructors_reserve_ids_for_unbound_nested_placements() {
    let source = "page=canvas(size=(100,100))\nfunction f(n) {unused=rect(size=(1,1))\nreturn n+2}\na=f(3)\nitems=[page.add(rect(size=(a,10))),page.add(ellipse(size=(3,4)))]\n";
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    compare(&mut session, source, 2);
    compare(&mut session, &source.replace("f(3)", "f(4)"), 3);
    compare(&mut session, source, 4);
}

#[test]
fn local_group_contributions_reuse_and_rebase_internal_anchors() {
    let source = r##"page=canvas(size=(100,100))
style { .shape {fill:#336699} }
function item(n) {
 g=group()
 a=g.add(rect(size=(n,3),class="shape"))
 g.add(ellipse(size=(2,2)),target=a.bottom_right)
 return g
}
function wrap(n) { return item(n) }
unused=rect(size=(1,1))
shape=wrap(8)
page.add(shape)
page.add(rect(size=(4,4)))"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    compare(&mut session, source, 2);
    assert!(
        session.metrics().reused_constructions > 0,
        "{:?}",
        session.metrics()
    );
    for (i, source) in [
        source.replace(
            "unused=rect(size=(1,1))",
            "unused=[rect(size=(1,1)),rect(size=(2,2))]",
        ),
        source.replace("wrap(8)", "wrap(9)"),
        source.replace("#336699", "#339966"),
        source.replace("a.bottom_right", "a.center"),
        format!("# moved\n{source}"),
        source.into(),
        source.into(),
    ]
    .iter()
    .enumerate()
    {
        compare(&mut session, source, i as u64 + 3);
    }
    assert!(session.metrics().reused_constructions > 0);
}

#[test]
fn source_transactions_and_cancelled_evaluation_keep_last_success() {
    use std::sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    };
    let source = "page=canvas(size=(20,20))\npage.add(rect(size=(5,5)))";
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    assert!(
        session
            .apply_changes(1, [("/main.lay".into(), Some(b"invalid".to_vec()))])
            .is_err()
    );
    assert_eq!(session.source("/main.lay").unwrap(), source);
    let latest = Arc::new(AtomicU64::new(1));
    session.set_cancellation(latest.clone(), 1);
    session
        .apply_changes(
            2,
            [(
                "/main.lay".into(),
                Some(source.replace("(5,5)", "(6,6)").into_bytes()),
            )],
        )
        .unwrap();
    latest.store(2, Ordering::Relaxed);
    assert_eq!(
        session.evaluate_file("/main.lay", None).unwrap_err().code,
        "E_CANCELLED"
    );
    assert_eq!(session.snapshot("/main.lay").unwrap().0, 1);
    session.set_cancellation(latest, 2);
    session.evaluate_file("/main.lay", None).unwrap();
    assert_eq!(session.snapshot("/main.lay").unwrap().0, 2);
}
#[test]
fn line_changes_and_anonymous_insertion_preserve_geometry_and_object_identity() {
    let source = "page=canvas(size=(80,50))\npage.add(path(commands=[move_to(0,0),cubic_to(1,2,3,4,5,6)]))\npage.add(ellipse(size=(5,5)))";
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    let old = session.snapshot("/main.lay").unwrap().1;
    compare(&mut session, &format!("# moved\n{source}"), 2);
    assert_eq!(session.metrics().rebuilt_geometry, 0);
    let edited = source.replace("page.add(path", "page.add(rect(size=(2,2)))\npage.add(path");
    compare(&mut session, &edited, 3);
    let new = session.snapshot("/main.lay").unwrap().1;
    assert_eq!(old.nodes[0]["_identity"], new.nodes[1]["_identity"]);
    assert_eq!(old.nodes[1]["_identity"], new.nodes[2]["_identity"]);
    assert_eq!(session.metrics().rebuilt_geometry, 1);
}

#[test]
fn recorded_resource_reads_follow_replacement_missing_and_recovery() {
    let source = "page=canvas(size=(50,50))\nfunction width() {d=dict(src=\"values.json\") return d[\"width\"]}\na=width()\npage.add(rect(size=(a,5)))";
    let mut session = CompileSession::new(false);
    let mut host = Host::default();
    for (i, data) in [
        Some("{\"width\":8}"),
        Some("{\"width\":8}"),
        Some("{\"width\":12}"),
        None,
        Some("{\"width\":8}"),
        Some("{\"width\":8}"),
    ]
    .iter()
    .enumerate()
    {
        host.files.clear();
        if let Some(data) = data {
            host.files
                .insert("/values.json".into(), data.as_bytes().to_vec());
        }
        session.sync_host(&host, i as u64 + 1).unwrap();
        let actual = session.evaluate(source, "/main.lay", None);
        let expected = compile_source(source, "/main.lay", host.clone());
        match (actual, expected) {
            (Ok(a), Ok(b)) => assert_eq!(scene(&a), scene(&b)),
            (Err(a), Err(b)) => assert_eq!(a, b),
            (a, b) => panic!("{a:?} / {b:?}"),
        }
        if i == 1 || i == 5 {
            assert_eq!(session.metrics().evaluated_builtin_calls.get("dict"), None);
        }
    }
}
#[test]
fn independent_group_iterations_reuse_existing_constructions() {
    let source = "page=canvas(size=(80,50))\nfunction item(i) {g=group()\ng.add(path(commands=[move_to(0,0),cubic_to(2,i,3,4,5,6)]))\nreturn g}\nfor i in range(8) {page.add(item(i),offset=(i*8,0))}";
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    compare(&mut session, source, 2);
    assert_eq!(session.metrics().evaluated_builtin_calls.get("path"), None);
    compare(&mut session, &source.replace("range(8)", "range(9)"), 3);
    assert_eq!(
        session.metrics().evaluated_builtin_calls.get("path"),
        Some(&1)
    );
    assert_eq!(session.metrics().reused_constructions, 8);
    assert_eq!(session.metrics().rebuilt_geometry, 1);
}

#[test]
fn changing_a_curve_formula_recomputes_its_iterations_and_reuses_other_functions() {
    let source = r##"p=canvas(size=(100,80))
function curve(i) {
 g=group()
 g.add(path(commands=[move_to(0,0),cubic_to(2,i+1,3,4,5,6)]))
 return g
}
function other(i) {g=group() g.add(ellipse(size=(i+1,2))) return g}
for i in range(12) {p.add(curve(i),offset=(i*5,0))}
for i in range(12) {p.add(other(i),offset=(i*5,20))}
"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    compare(&mut session, &source.replace("i+1,3,4", "i+101,3,4"), 2);
    let metrics = session.metrics();
    assert_eq!(metrics.evaluated_builtin_calls.get("path"), Some(&12));
    assert_eq!(metrics.evaluated_builtin_calls.get("ellipse"), None);
    assert_eq!(metrics.rebuilt_geometry, 12);
    assert!(metrics.reused_constructions >= 12);
    compare(&mut session, source, 3);
    assert_eq!(
        session.metrics().evaluated_builtin_calls.get("ellipse"),
        None
    );
}

#[test]
fn cached_nested_group_contributions_rebase_unbound_instance_names() {
    let source = r##"page=canvas(size=(100,100))
function item() {
 a=group()
 instances=[a.add(rect(size=(3,4)))]
 b=group()
 instances=[b.add(a)]
 return b
}
unused=rect(size=(1,1))
shape=item()
page.add(shape)"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    compare(&mut session, source, 2);
    compare(
        &mut session,
        &source.replace(
            "unused=rect(size=(1,1))",
            "unused=[rect(size=(1,1)),rect(size=(2,2))]",
        ),
        3,
    );
    assert!(
        session.metrics().reused_constructions > 0,
        "{:?}",
        session.metrics()
    );
}

#[test]
fn unrelated_background_edit_reuses_group_construction() {
    let source = r##"page=canvas(size=(100,100),background="#eeeeee")
function item() {
 g=group()
 for i in range(9) {g.add(path(commands=[move_to(0,0),cubic_to(1,i,3,4,5,6)]))}
 return g
}
page.add(item())"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    compare(&mut session, &source.replace("#eeeeee", "#ffffff"), 2);
    assert!(
        session.metrics().reused_constructions > 0,
        "{:?}",
        session.metrics()
    );
    assert_eq!(session.metrics().evaluated_builtin_calls.get("path"), None);
    assert_eq!(session.metrics().rebuilt_geometry, 0);
}

#[test]
fn sibling_insertions_reuse_existing_group_contributions_and_rebase_names() {
    let module = r##"export function item() {
 g=group()
 a=g.add(path(commands=[move_to(0,0),cubic_to(2,3,5,7,9,1)]))
 g.add(ellipse(size=(2,2)),target=a.bottom_right)
 return g
}"##;
    let source =
        "import {item} from \"shape.lay\"\np=canvas(size=(100,100))\nshape=item()\np.add(shape)";
    let mut host = Host::default();
    host.files
        .insert("/shape.lay".into(), module.as_bytes().to_vec());
    let mut session = CompileSession::new(false);
    session.set_profiling(true);
    for (i, next) in [
        source.to_owned(),
        source.replace(
            "shape=item()",
            "a=p.add(rect(size=(1,1)))\np.add(rect(size=(1,1)))\nshape=item()",
        ),
        source.to_owned(),
    ]
    .iter()
    .enumerate()
    {
        session.sync_host(&host, i as u64 + 1).unwrap();
        let actual = session.evaluate(next, "/main.lay", None).unwrap();
        let expected = compile_source(next, "/main.lay", host.clone()).unwrap();
        assert_eq!(scene(&actual), scene(&expected), "revision {i}");
        if i > 0 {
            assert!(
                session.metrics().reused_constructions > 0,
                "{:?}",
                session.metrics()
            );
            assert_eq!(session.metrics().evaluated_builtin_calls.get("path"), None);
        }
    }
}

#[test]
fn list_insertions_preserve_existing_constructor_identities_and_computations() {
    let source = "p=canvas(size=(80,50))\nshapes=[path(commands=[move_to(0,0),cubic_to(2,3,5,7,9,1)]),ellipse(size=(4,4))]\nfor shape in shapes {p.add(shape)}";
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    compare(
        &mut session,
        &source.replace("shapes=[", "shapes=[rect(size=(2,2)),"),
        2,
    );
    assert_eq!(session.metrics().evaluated_builtin_calls.get("path"), None);
    assert_eq!(
        session.metrics().evaluated_builtin_calls.get("ellipse"),
        None
    );
}

#[test]
fn unchanged_placed_group_shares_scene_payload_and_detaches_later_writes() {
    let source = r##"p=canvas(size=(80,50),background="#eeeeee")
function item() {g=group()
 for i in range(20) {g.add(path(commands=[move_to(0,0),cubic_to(2,i,5,7,9,1)]))}
 return g}
a=item()
x=p.add(a,target=p.top_left,offset=(5,5))"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    let before = session.snapshot("/main.lay").unwrap().1;
    compare(&mut session, &source.replace("#eeeeee", "#ffffff"), 2);
    let after = session.snapshot("/main.lay").unwrap().1;
    assert!(
        std::sync::Arc::ptr_eq(&before.nodes[0], &after.nodes[0]),
        "Unchanged placed group must keep its scene payload"
    );
    compare(
        &mut session,
        &source.replace("x=p.add", "a.add(rect(size=(3,3)))\nx=p.add"),
        3,
    );
    compare(&mut session, source, 4);
    assert_eq!(
        scene(&before),
        scene(&session.snapshot("/main.lay").unwrap().1)
    );
}

#[test]
fn long_edit_and_undo_sequence_releases_obsolete_derivations() {
    let mut session = CompileSession::new(false);
    session.set_budget(64 * 1024);
    for revision in 1..=160 {
        let value = (revision % 13) as usize;
        let source = format!(
            "p=canvas(size=(80,50))\nfunction item(i) {{g=group()\ng.add(path(commands=[move_to(0,0),cubic_to(2,i+{value},5,7,9,1)]))\nreturn g}}\nfor i in range({}) {{p.add(item(i),offset=(i*4,0))}}",
            5 + value % 4
        );
        compare(&mut session, &source, revision);
    }
    session.set_budget(0);
    compare(&mut session, "p=canvas(size=(80,50))", 161);
    assert_eq!(session.metrics().geometry_cache_bytes, 0);
    assert_eq!(session.metrics().node_cache_bytes, 0);
    session.dispose();
    assert_eq!(session.metrics().cache_bytes, 0);
}

#[test]
fn cooperative_yield_retains_completed_queries_and_commits_only_finished_versions() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static TICKS: AtomicU64 = AtomicU64::new(0);
    fn clock() -> f64 {
        TICKS.fetch_add(1, Ordering::Relaxed) as f64
    }
    let mut session = CompileSession::new(false);
    compare(&mut session, "p=canvas(size=(80,50))", 1);
    let previous = session.snapshot("/main.lay").unwrap();
    let source = "p=canvas(size=(80,50))\nfunction item(i) {g=group()\ng.add(rect(size=(2,3)))\nreturn g}\nfor i in range(40) {p.add(item(i),offset=(i,0))}";
    session
        .apply_changes(2, [("/main.lay".into(), Some(source.as_bytes().to_vec()))])
        .unwrap();
    session.set_profile_clock(clock);
    TICKS.store(0, Ordering::Relaxed);
    session.set_deadline(Some(1.));
    assert_eq!(
        session
            .evaluate(source, "/main.lay", None)
            .unwrap_err()
            .code,
        "E_YIELD"
    );
    assert!(std::sync::Arc::ptr_eq(
        &previous.1,
        &session.snapshot("/main.lay").unwrap().1
    ));
    session.set_deadline(None);
    compare(&mut session, source, 2);
    assert!(session.metrics().reused_expressions > 0);
}

#[test]
fn nested_styled_effects_survive_size_edits() {
    let source = r##"
style { @keyframes fade { from { opacity:0; } to { opacity:1; } } .paper { fill:#ffffff; border-width:0pt; border-radius:2.5mm; effects:[shadow(color="#0e3344",blur=1.4mm,offset=(0mm,1.1mm),opacity=0.08)]; } }
p=canvas(size=(320mm,180mm))
board=group()
main_group=group()
plate=main_group.add(rect(size=(184mm,104mm),class="paper"))
main=board.add(main_group)
detail=group()
detail_plate=detail.add(rect(size=(91mm,63mm),class="paper"))
top=board.add(detail,target=main.top_right,offset=(12mm,0mm))
body=p.add(board)
s=p.step()
s.animate(main,name="fade",duration=1s)
"##;
    let mut session = CompileSession::new(false);
    for (i, width) in [91, 92, 93, 91, 92].into_iter().enumerate() {
        compare(
            &mut session,
            &source.replace("91mm", &format!("{width}mm")),
            i as u64 + 1,
        );
    }
}

#[test]
fn yielded_literal_style_constructors_never_poison_retained_nodes() {
    use std::cell::Cell;
    thread_local! {static TICKS:Cell<usize>=const{Cell::new(0)};}
    fn clock() -> f64 {
        TICKS.with(|n| {
            let v = n.get();
            n.set(v + 1);
            v as f64
        })
    }
    let source = r##"
style { .paper { fill:#ffffff; effects:[shadow(color="#334455",blur=1mm,offset=(0mm,1mm))]; } }
p=canvas(size=(200mm,100mm))
function pieces() {
 g=group()
 for i in range(30) { g.add(rect(size=(3mm,4mm),class="paper "+str(i)),offset=(i*5mm,0mm)) }
 return g
}
p.add(pieces())
"##;
    let expected = scene(&compile_source(source, "/main.lay", Host::default()).unwrap());
    for boundary in 0..48 {
        let mut session = CompileSession::new(false);
        session.set_profile_clock(clock);
        TICKS.with(|n| n.set(0));
        session.set_deadline(Some(boundary as f64));
        match session.evaluate(source, "/main.lay", None) {
            Err(error) => assert_eq!(error.code, "E_YIELD"),
            Ok(actual) => assert_eq!(
                scene(&actual),
                expected,
                "An interrupted CSS constructor committed at checkpoint {boundary}"
            ),
        }
        session.set_deadline(None);
        let actual = session.evaluate(source, "/main.lay", None).unwrap();
        assert_eq!(
            scene(&actual),
            expected,
            "Style lost after yielding at checkpoint {boundary}"
        );
    }
}

#[test]
fn unrelated_lcss_rule_reuses_curve_constructors_and_new_matching_rules_invalidate() {
    let source = r##"
style { .heading {fill:#ff0000} .strand {border-color:#000000; border-width:0.2mm} }
p=canvas(size=(100mm,100mm))
function curves() {
 g=group(class="curves")
 for i in range(20) { g.add(path(commands=(move_to(0mm,i*1mm),cubic_to(10mm,2mm,20mm,4mm,30mm,i*1mm)),class="strand")) }
 return g
}

p.add(rect(size=(4mm,4mm),class="heading"))
p.add(curves(),offset=(10mm,10mm))
"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    let title = source.replace("#ff0000", "#0000ff");
    compare(&mut session, &title, 2);
    assert_eq!(
        session.metrics().evaluated_builtin_calls.get("path"),
        None,
        "Unrelated heading rule invalidated curve construction"
    );
    assert_eq!(session.metrics().rebuilt_geometry, 0);
    assert!(
        session.metrics().materialized_nodes < 5,
        "Unrelated heading rule rematerialized the curve group: {:?}",
        session.metrics()
    );
    for (i, style) in [
        ".curves .strand {border-color:#00ff00}",
        "@media (min-width:50mm) {.strand {border-width:1mm}}",
        ".curves {color:#334455} .strand {border-color:currentColor}",
    ]
    .into_iter()
    .enumerate()
    {
        let changed = title.replace(".heading {", &format!("{style} .heading {{"));
        compare(&mut session, &changed, i as u64 + 3);
    }
}

#[test]
fn lcss_paint_updates_reuse_construction_and_geometry() {
    let source = r##"
style { canvas { --ink:#224466 } .strand {border-color:var(--ink);border-width:0.2mm;fill:none} }
p=canvas(size=(100mm,100mm))
function curves() {
 g=group(class="curves")
 origin=g.add(rect(size=(50mm,50mm),fill="none"))
 for i in range(32) { g.add(path(commands=[move_to(0mm,0mm),cubic_to(2mm,i*0.2mm,4mm,3mm,8mm,5mm)],class="strand"),anchor=self.path.start,target=origin.top_left,offset=(i*0.1mm,0mm)) }
 return g
}
p.add(curves(),offset=(10mm,10mm))
"##;
    let mut session = CompileSession::new(false);
    compare(&mut session, source, 1);
    for (index, source) in [
        source.replace("#224466", "#e08020"),
        source.replace("#224466", "#338811"),
        source.replace(
            ".strand {",
            ".curves .strand {border-color:#ff0088} .strand {",
        ),
        source
            .replace("border-color:var(--ink)", "border-color:currentColor")
            .replace(".strand {", ".curves {color:#774411} .strand {"),
        source.to_owned(),
    ]
    .iter()
    .enumerate()
    {
        compare(&mut session, source, index as u64 + 2);
        let m = session.metrics();
        assert_eq!(
            m.evaluated_builtin_calls.get("path").copied().unwrap_or(0),
            0,
            "{m:?}"
        );
        assert_eq!(m.rebuilt_geometry, 0, "{m:?}");
        assert!(
            m.materialized_nodes <= 1,
            "Paint-only changes rebuilt material nodes: {m:?}"
        );
        if index < 4 {
            assert!(
                m.updated_appearance_nodes >= 32,
                "Paint changes must reach retained paths: {m:?}"
            );
        }
    }
}

#[test]
fn style_changes_observed_by_geometry_queries_preserve_control_flow() {
    let source = r##"
style { .strand {border-color:#224466;border-width:2mm;fill:none} }
p=canvas(size=(100mm,100mm))
function draw() {
 g=group()
 a=g.add(rect(size=(10mm,10mm),class="strand"))
 if a.ink.bounds.width>11mm { g.add(ellipse(size=(3mm,3mm)),offset=(20mm,0mm)) }
 return g
}
p.add(draw())
"##;
    let mut session = CompileSession::new(false);
    for (i, s) in [
        source.to_owned(),
        source.replace("#224466", "none"),
        source.replace("2mm;fill", "0.1mm;fill"),
        source.to_owned(),
    ]
    .iter()
    .enumerate()
    {
        compare(&mut session, s, i as u64 + 1);
    }
}

#[test]
fn text_paint_retains_layout_fonts_and_color_run_boundaries() {
    let mut host = Host::default();
    host.files.insert(
        "/font.ttf".into(),
        include_bytes!("../../../tests/fonts/DejaVuSans.ttf").to_vec(),
    );
    let original = r##"style {.label {color:#224466}}
p=canvas(size=(100mm,100mm))
p.add(text("AV office",class="label",font_family="/font.ttf",font_size=4mm))
p.add(text(spans=[span("A",color="#224466"),span("V",color="#bb2288")],font_family="/font.ttf",font_size=4mm),offset=(0mm,20mm))"##;
    let mut session = CompileSession::new(false);
    for (i, source) in [
        original.to_owned(),
        original.replace("#224466", "#44aa99"),
        original.replace("#224466", "#bb2288"),
        original.replace("#224466", "#000000"),
        format!("\n\n{original}"),
        original.to_owned(),
    ]
    .iter()
    .enumerate()
    {
        host.files
            .insert("/main.lay".into(), source.as_bytes().to_vec());
        session.sync_host(&host, i as u64 + 1).unwrap();
        let full = compile_source(source, "/main.lay", host.clone()).unwrap();
        let updated = session.evaluate(source, "/main.lay", None).unwrap();
        assert_eq!(scene(&updated), scene(&full), "revision {i}");
        assert_eq!(updated.font_requests, full.font_requests);
        if i == 1 {
            assert_eq!(session.metrics().rebuilt_text_layouts, 0);
            assert!(session.metrics().reused_text_layouts >= 2);
        }
    }
    let retained = session.metrics().text_layout_cache_bytes;
    session.set_budget(0);
    assert_eq!(session.metrics().text_layout_cache_bytes, retained);
    session.dispose();
    assert_eq!(session.metrics().text_layout_cache_bytes, 0);
}
