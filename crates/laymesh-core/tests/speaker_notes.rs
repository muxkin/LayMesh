use laymesh_core::{engine::compile_source, model::Host};
#[test]
fn only_entry_top_level_triple_literals_are_notes() {
    let source = r#""""
        第一段
        保留换行
    """
import {thing} from "part.lay"
value = '''不是备注'''
p = canvas(size=(20,20))
function unused() { '''函数里的备注''' }
if true { """条件里的备注""" }
for i in [1] { '''循环里的备注''' }
'''第二段'''
"#;
    let scene=compile_source(source,"/main.lay",Host {files:[("/part.lay".into(), b"'''Imported note'''\nexport thing=1".to_vec())].into_iter().collect(),..Host::default()}).unwrap();
    assert_eq!(scene.speaker_notes.as_deref(),Some("第一段\n保留换行\n\n第二段"));
    let empty=compile_source("p=canvas(size=(20,20))", "/empty.lay",Host::default()).unwrap();
    assert!(empty.speaker_notes.is_none());
}
#[test]
fn assignment_argument_and_single_quotes_are_not_notes() {
    use laymesh_core::parser::{parse,StmtKind};
    let ast=parse("a='''assigned'''\np.add('''argument''')\n\"single\"\n'''note'''", "/notes.lay").unwrap();
    assert_eq!(ast.iter().filter(|s| matches!(s.kind,StmtKind::Note(_))).count(),1);
}
