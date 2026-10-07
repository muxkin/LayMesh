//! Dependency-free JSON-RPC 2.0 / Language Server Protocol transport over stdio.
use crate::{LanguageService, offset, position};
use serde_json::{Value as J, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, BufRead, Read, Write},
};
fn send(out: &mut impl Write, message: J) -> io::Result<()> {
    let body = serde_json::to_vec(&message)?;
    write!(out, "Content-Length: {}\r\n\r\n", body.len())?;
    out.write_all(&body)?;
    out.flush()
}
fn path(uri: &str) -> String {
    let raw = if let Some(raw) = uri.strip_prefix("file://") {
        if let Some(local) = raw.strip_prefix("localhost/") {
            format!("/{local}")
        } else if !raw.starts_with('/') {
            format!("//{raw}")
        } else {
            raw.to_owned()
        }
    } else {
        uri.to_owned()
    };
    let mut bytes = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw.as_bytes()[i] == b'%' && i + 2 < raw.len() {
            if let Some(v) = std::str::from_utf8(&raw.as_bytes()[i + 1..i + 3])
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
            {
                bytes.push(v);
                i += 3;
                continue;
            }
        }
        bytes.push(raw.as_bytes()[i]);
        i += 1
    }
    let p = String::from_utf8_lossy(&bytes).into_owned();
    if cfg!(windows) && p.starts_with('/') && p.as_bytes().get(2) == Some(&b':') {
        p[1..].into()
    } else {
        p
    }
}
fn range(service: &LanguageService, uri: &str, a: usize, b: usize) -> J {
    let s = service.documents.get(uri).map(String::as_str).unwrap_or("");
    json!({"start":position(s,a),"end":position(s,b)})
}
fn roots(params: &J) -> BTreeSet<String> {
    let mut roots: BTreeSet<_> = params["workspaceFolders"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|folder| folder["uri"].as_str())
        .filter(|uri| uri.starts_with("file://"))
        .map(str::to_string)
        .collect();
    if roots.is_empty() {
        if let Some(uri) = params["rootUri"]
            .as_str()
            .filter(|uri| uri.starts_with("file://"))
        {
            roots.insert(uri.into());
        }
    }
    roots
}
fn workspace_files(root_uri: &str) -> Vec<(String, String)> {
    fn walk(
        root: &std::path::Path,
        directory: &std::path::Path,
        root_uri: &str,
        files: &mut Vec<(String, String)>,
    ) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() {
                continue;
            }
            let filename = entry.file_name();
            let name = filename.to_string_lossy();
            let path = entry.path();
            if kind.is_dir() {
                if [
                    ".git",
                    "target",
                    "node_modules",
                    ".venv",
                    "venv",
                    "dist",
                    "build",
                    "bin",
                    "licenses",
                    "__pycache__",
                    ".ipynb_checkpoints",
                    "coverage",
                ]
                .contains(&name.as_ref())
                {
                    continue;
                }
                if directory.file_name().is_some_and(|n| n == "release")
                    && ["staging", "history", "comparison", "verification"].contains(&name.as_ref())
                {
                    continue;
                }
                walk(root, &path, root_uri, files);
            } else if kind.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext == "lay" || ext == "lcss")
            {
                if let (Ok(relative), Ok(source)) =
                    (path.strip_prefix(root), std::fs::read_to_string(&path))
                {
                    let relative = relative.to_string_lossy().replace('\\', "/");
                    let uri = crate::resolve(
                        &format!("{}/.laymesh-index.lay", root_uri.trim_end_matches('/')),
                        &relative,
                    );
                    files.push((uri, source));
                }
            }
        }
    }
    let root = std::path::PathBuf::from(path(root_uri));
    let mut files = vec![];
    if std::fs::symlink_metadata(&root).is_ok_and(|meta| !meta.file_type().is_symlink()) {
        walk(&root, &root, root_uri, &mut files);
    }
    files
}
fn workspace(
    service: &mut LanguageService,
    roots: &BTreeSet<String>,
    opened: &BTreeMap<String, J>,
    indexed: &mut BTreeSet<String>,
) {
    let mut current = BTreeSet::new();
    for root in roots {
        for (uri, source) in workspace_files(root) {
            if !opened.contains_key(&uri) {
                service.update(&uri, &source);
            }
            current.insert(uri);
        }
    }
    for uri in indexed.difference(&current) {
        if !opened.contains_key(uri) {
            service.remove(uri);
        }
    }
    *indexed = current;
}
fn locations(service: &LanguageService, items: J, highlight: bool) -> J {
    json!(
        items
            .as_array()
            .unwrap()
            .iter()
            .map(|o| {
                let r = range(
                    service,
                    o["uri"].as_str().unwrap(),
                    o["from"].as_u64().unwrap() as usize,
                    o["to"].as_u64().unwrap() as usize,
                );
                if highlight {
                    json!({"range":r,"kind":o["kind"]})
                } else {
                    json!({"uri":o["uri"],"range":r})
                }
            })
            .collect::<Vec<_>>()
    )
}
fn dependencies(
    service: &mut LanguageService,
    uri: &str,
    opened: &BTreeMap<String, J>,
    seen: &mut BTreeSet<String>,
) {
    if !seen.insert(uri.into()) {
        return;
    }
    if !opened.contains_key(uri) && uri.starts_with("file:") {
        if let Ok(text) = std::fs::read_to_string(path(uri)) {
            service.update(uri, &text)
        } else {
            service.remove(uri);
            return;
        }
    }
    let Some(text) = service.documents.get(uri).cloned() else {
        return;
    };
    for token in crate::index::tokens(&text)
        .into_iter()
        .filter(|t| t.kind == "string")
    {
        let Ok(expr) = laymesh_core::parser::expression(&token.text, uri) else {
            continue;
        };
        let laymesh_core::parser::ExprKind::String(source, _, false) = expr.kind else {
            continue;
        };
        if !source.ends_with(".lay") && !source.ends_with(".lcss") {
            continue;
        }
        let target = crate::resolve(uri, &source);
        dependencies(service, &target, opened, seen)
    }
}
fn diagnostics(service: &LanguageService, uri: &str, version: J) -> J {
    let ds=service.diagnostics(uri).as_array().unwrap().iter().map(|d|json!({"range":range(service,uri,d["from"].as_u64().unwrap_or(0)as usize,d["to"].as_u64().unwrap_or(0)as usize),"severity":if d["severity"]=="warning"{2}else{1},"code":d["code"],"source":"LayMesh","message":d["message"],"data":d})).collect::<Vec<_>>();
    json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"version":version,"diagnostics":ds}})
}
/// Open buffers are authoritative; all other resources are refreshed from disk.
/// Revalidate open dependents after file events so clients cannot retain stale diagnostics.
fn refresh(
    service: &mut LanguageService,
    opened: &BTreeMap<String, J>,
    out: &mut impl Write,
) -> io::Result<()> {
    let mut seen = BTreeSet::new();
    for uri in opened.keys() {
        dependencies(service, uri, opened, &mut seen);
    }
    for (uri, version) in opened {
        send(out, diagnostics(service, uri, version.clone()))?;
    }
    Ok(())
}
fn chosen_locale(ui: &str, initial: &str, configured: &str) -> String {
    let explicit = if ["en", "zh-CN", "zh"].contains(&configured) {
        configured
    } else if configured == "auto" {
        ui
    } else if ["en", "zh-CN", "zh"].contains(&initial) {
        initial
    } else {
        ui
    };
    if explicit.to_lowercase().starts_with("zh") {
        "zh-CN".into()
    } else {
        "en".into()
    }
}
fn plain(s: &str) -> String {
    let s = regex::Regex::new(r"(?s)<[^>]+>")
        .unwrap()
        .replace_all(s, "");
    let s = regex::Regex::new(r"(?m)^```[^\n]*\n?")
        .unwrap()
        .replace_all(&s, "");
    let s = regex::Regex::new(r"\[([^]]+)\]\(([^)]+)\)")
        .unwrap()
        .replace_all(&s, "$1 ($2)");
    regex::Regex::new(r"(?m)^\s*[-*+]\s+")
        .unwrap()
        .replace_all(&s, "• ")
        .replace("**", "")
        .replace('`', "")
        .replace("*", "")
}
fn docs(s: &str, markdown: bool) -> J {
    json!({"kind":if markdown{"markdown"}else{"plaintext"},"value":if markdown{s.to_string()}else{plain(s)}})
}
pub fn serve() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut service = LanguageService::new("en");
    let mut opened = BTreeMap::<String, J>::new();
    let mut workspace_roots = BTreeSet::new();
    let mut workspace_documents = BTreeSet::new();
    let mut versioned_edits = false;
    let mut closed = false;
    let mut ui = "en".to_string();
    let mut initial = String::new();
    let mut markdown = [true; 3];
    let mut labels = true;
    let mut pull_configuration = false;
    let mut register_configuration = false;
    let mut waiting_configuration = false;
    let mut queued = std::collections::VecDeque::<J>::new();
    loop {
        let msg = if !waiting_configuration && !queued.is_empty() {
            queued.pop_front().unwrap()
        } else {
            let mut length = None;
            loop {
                let mut line = String::new();
                if input.read_line(&mut line)? == 0 {
                    return Ok(());
                }
                if line.trim().is_empty() {
                    break;
                }
                if let Some((key, value)) = line.split_once(':') {
                    if key.eq_ignore_ascii_case("Content-Length") {
                        length = value.trim().parse::<usize>().ok()
                    }
                }
            }
            let Some(n) = length else { continue };
            if n > 16 * 1024 * 1024 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "LSP message exceeds 16 MiB",
                ));
            }
            let mut bytes = vec![0; n];
            input.read_exact(&mut bytes)?;
            let msg: J = match serde_json::from_slice(&bytes) {
                Ok(j) => j,
                Err(_) => {
                    send(
                        &mut output,
                        json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}),
                    )?;
                    continue;
                }
            };
            msg
        };
        if msg.get("method").is_none() {
            if msg["id"] == "laymesh/configuration" {
                waiting_configuration = false;
                service.locale = chosen_locale(
                    &ui,
                    &initial,
                    msg["result"][0]["language"].as_str().unwrap_or(""),
                );
                refresh(&mut service, &opened, &mut output)?;
            }
            continue;
        }
        if waiting_configuration {
            queued.push_back(msg);
            continue;
        }
        let id = msg.get("id").cloned();
        let method = msg["method"].as_str().unwrap_or("");
        let p = &msg["params"];
        let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
        if matches!(
            method,
            "textDocument/references"
                | "textDocument/documentHighlight"
                | "textDocument/prepareRename"
                | "textDocument/rename"
        ) {
            workspace(
                &mut service,
                &workspace_roots,
                &opened,
                &mut workspace_documents,
            );
            let documents: Vec<_> = service.documents.keys().cloned().collect();
            let mut seen = BTreeSet::new();
            for document in documents {
                dependencies(&mut service, &document, &opened, &mut seen);
            }
        }
        let at = offset(
            service.documents.get(uri).map(String::as_str).unwrap_or(""),
            &p["position"],
        );
        let mut result = J::Null;
        let mut unknown = false;
        let mut response_error = None;
        match method{"initialize"=>{workspace_roots=roots(p);workspace(&mut service,&workspace_roots,&opened,&mut workspace_documents);versioned_edits=p["capabilities"]["workspace"]["workspaceEdit"]["documentChanges"].as_bool().unwrap_or(false);pull_configuration=p["capabilities"]["workspace"]["configuration"].as_bool().unwrap_or(false);register_configuration=p["capabilities"]["workspace"]["didChangeConfiguration"]["dynamicRegistration"].as_bool().unwrap_or(false);ui=p["locale"].as_str().unwrap_or("en").into();initial=p["initializationOptions"]["language"].as_str().unwrap_or("").into();service.locale=chosen_locale(&ui,&initial,"");for(i,v)in ["completion","hover","signatureHelp"].iter().enumerate(){let options=if *v=="completion"{&p["capabilities"]["textDocument"][v]["completionItem"]["documentationFormat"]}else if *v=="signatureHelp"{&p["capabilities"]["textDocument"][v]["signatureInformation"]["documentationFormat"]}else{&p["capabilities"]["textDocument"][v]["contentFormat"]};markdown[i]=options.as_array().and_then(|a|a.iter().find(|v|*v=="markdown"||*v=="plaintext")).is_some_and(|v|v=="markdown");}labels=p["capabilities"]["textDocument"]["signatureHelp"]["signatureInformation"]["parameterInformation"]["labelOffsetSupport"].as_bool().unwrap_or(false);result=json!({"capabilities":{"positionEncoding":"utf-16","textDocumentSync":{"openClose":true,"change":2,"save":true},"completionProvider":{"triggerCharacters":[".","(",":","_","\"","'","=",","]},"documentFormattingProvider":true,"documentRangeFormattingProvider":true,"hoverProvider":true,"colorProvider":true,"signatureHelpProvider":{"triggerCharacters":["(",",","="]},"definitionProvider":true,"referencesProvider":true,"documentHighlightProvider":true,"renameProvider":{"prepareProvider":true},"workspace":{"workspaceFolders":{"supported":true,"changeNotifications":true}},"codeActionProvider":{"codeActionKinds":["quickfix"]}},"serverInfo":{"name":"LayMesh Rust","version":env!("CARGO_PKG_VERSION")}})},
 "shutdown"=>{closed=true},"exit"=>return if closed{Ok(())}else{Err(io::Error::new(io::ErrorKind::Other,"exit without shutdown"))},"initialized"=>{if register_configuration{send(&mut output,json!({"jsonrpc":"2.0","id":"laymesh/register","method":"client/registerCapability","params":{"registrations":[{"id":"laymesh-configuration","method":"workspace/didChangeConfiguration"}]}}))?;}if pull_configuration{waiting_configuration=true;send(&mut output,json!({"jsonrpc":"2.0","id":"laymesh/configuration","method":"workspace/configuration","params":{"items":[{"section":"laymesh"}]}}))?;}},"$/cancelRequest"|"$/setTrace"=>{},
 "workspace/didChangeConfiguration"=>{if pull_configuration{waiting_configuration=true;send(&mut output,json!({"jsonrpc":"2.0","id":"laymesh/configuration","method":"workspace/configuration","params":{"items":[{"section":"laymesh"}]}}))?;}service.locale=chosen_locale(&ui,&initial,p["settings"]["laymesh"]["language"].as_str().unwrap_or(""));refresh(&mut service, &opened, &mut output)?;},
 "textDocument/didOpen"=>{opened.insert(uri.into(),p["textDocument"]["version"].clone());service.update(uri,p["textDocument"]["text"].as_str().unwrap_or(""));refresh(&mut service,&opened,&mut output)?;},
 "textDocument/didChange"=>{let mut text=service.documents.get(uri).cloned().unwrap_or_default();if let Some(changes)=p["contentChanges"].as_array(){for change in changes{let value=change["text"].as_str().unwrap_or("");if change["range"].is_object(){let a=offset(&text,&change["range"]["start"]);let b=offset(&text,&change["range"]["end"]);if a<=b{text.replace_range(a..b,value)}}else{text=value.into()}}}opened.insert(uri.into(),p["textDocument"]["version"].clone());service.update(uri,&text);refresh(&mut service,&opened,&mut output)?;},
 "textDocument/didClose"=>{opened.remove(uri);service.remove(uri);dependencies(&mut service,uri,&opened,&mut BTreeSet::new());send(&mut output,json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":[]}}))?;refresh(&mut service,&opened,&mut output)?;},"textDocument/didSave"=>{refresh(&mut service,&opened,&mut output)?;},
 "workspace/didChangeWorkspaceFolders"=>{if let Some(removed)=p["event"]["removed"].as_array(){for folder in removed{if let Some(uri)=folder["uri"].as_str(){workspace_roots.remove(uri);}}}if let Some(added)=p["event"]["added"].as_array(){for folder in added{if let Some(uri)=folder["uri"].as_str().filter(|uri|uri.starts_with("file://")){workspace_roots.insert(uri.into());}}}workspace(&mut service,&workspace_roots,&opened,&mut workspace_documents);refresh(&mut service,&opened,&mut output)?;},
 "workspace/didChangeWatchedFiles"=>{if let Some(changes)=p["changes"].as_array(){for c in changes{if let Some(uri)=c["uri"].as_str(){if !opened.contains_key(uri){if let Ok(source)=std::fs::read_to_string(path(uri)){service.update(uri,&source);}else{service.remove(uri);workspace_documents.remove(uri);}}dependencies(&mut service,uri,&opened,&mut BTreeSet::new());}}}refresh(&mut service,&opened,&mut output)?;},
 "textDocument/completion"=>{dependencies(&mut service,uri,&opened,&mut BTreeSet::new());result=json!(service.completions(uri,at).as_array().unwrap().iter().map(|c|json!({"label":c["label"],"detail":c["detail"],"kind":match c["type"].as_str().unwrap_or(""){"function"=>3,"property"=>10,"variable"=>6,"keyword"=>14,"class"=>7,_=>1},"documentation":docs(c["info"].as_str().unwrap_or(""),markdown[0]),"textEdit":{"range":range(&service,uri,c["from"].as_u64().unwrap_or(0)as usize,c["to"].as_u64().map(|n| n as usize).unwrap_or(at)),"newText":c["apply"]}})).collect::<Vec<_>>())},
 "textDocument/formatting"=>{result=service.formatting(uri,&p["options"],None)},
 "textDocument/rangeFormatting"=>{result=service.formatting(uri,&p["options"],Some(&p["range"]))},
 "textDocument/documentColor"=>{result=json!(service.document_colors(uri).as_array().unwrap().iter().map(|c|json!({"range":range(&service,uri,c["from"].as_u64().unwrap()as usize,c["to"].as_u64().unwrap()as usize),"color":{"red":c["rgba"][0],"green":c["rgba"][1],"blue":c["rgba"][2],"alpha":c["rgba"][3]}})).collect::<Vec<_>>())},
 "textDocument/colorPresentation"=>{let source=service.documents.get(uri).map(String::as_str).unwrap_or("");let from=offset(source,&p["range"]["start"]);let to=offset(source,&p["range"]["end"]);let c=&p["color"];let rgba=[c["red"].as_f64().unwrap_or(0.),c["green"].as_f64().unwrap_or(0.),c["blue"].as_f64().unwrap_or(0.),c["alpha"].as_f64().unwrap_or(1.)];result=json!(service.color_presentations(uri,from,to,rgba).as_array().unwrap().iter().map(|c|json!({"label":c["label"],"textEdit":{"range":p["range"],"newText":c["text"]}})).collect::<Vec<_>>())},
 "laymesh/colorConvert"=>{let c=laymesh_core::color::Color::new(p["space"].as_str().unwrap_or("rgb"),[p["channels"][0].as_f64().unwrap_or(f64::NAN),p["channels"][1].as_f64().unwrap_or(f64::NAN),p["channels"][2].as_f64().unwrap_or(f64::NAN)],p["alpha"].as_f64().unwrap_or(1.));result=match c {Ok(c)=>json!({"rgba":c.rgba,"mapped":c.mapped,"hex":c.hex(),"rgb":c.channels_in("rgb"),"hsv":c.channels_in("hsv"),"oklch":c.channels_in("oklch")}),Err(m)=>json!({"error":m})}},
 "laymesh/documentColors"=>{result=service.document_colors(uri);let source=service.documents.get(uri).map(String::as_str).unwrap_or("");for v in result.as_array_mut().unwrap(){for key in ["from","to"]{v[key]=json!(crate::utf16_offset(source,v[key].as_u64().unwrap()as usize));}}},
 "textDocument/hover"=>{dependencies(&mut service,uri,&opened,&mut BTreeSet::new());let h=service.hover(uri,at);if !h.is_null(){result=json!({"contents":docs(h["contents"].as_str().unwrap_or(""),markdown[1]),"range":range(&service,uri,h["from"].as_u64().unwrap_or(0)as usize,h["to"].as_u64().unwrap_or(0)as usize)})}},
 "textDocument/signatureHelp"=>{dependencies(&mut service,uri,&opened,&mut BTreeSet::new());let s=service.signature(uri,at);if !s.is_null(){let label=s["label"].as_str().unwrap();let params:Vec<J>=s["parameters"].as_array().unwrap().iter().map(|v|{let label=if labels{v["label"].clone()}else{let a=crate::byte_offset(label,v["label"][0].as_u64().unwrap()as usize);let b=crate::byte_offset(label,v["label"][1].as_u64().unwrap()as usize);json!(&label[a..b])};json!({"label":label,"documentation":docs(v["documentation"].as_str().unwrap_or(""),markdown[2])})}).collect();result=json!({"signatures":[{"label":label,"documentation":docs(s["documentation"].as_str().unwrap_or(""),markdown[2]),"parameters":params}],"activeSignature":0,"activeParameter":s["activeParameter"]})}},
 "textDocument/definition"=>{dependencies(&mut service,uri,&opened,&mut BTreeSet::new());let d=service.definition(uri,at);if !d.is_null(){result=json!({"uri":d["uri"],"range":range(&service,d["uri"].as_str().unwrap(),d["from"].as_u64().unwrap()as usize,d["to"].as_u64().unwrap()as usize)})}},
 "textDocument/references"=>{result=locations(&service,service.references(uri,at,p["context"]["includeDeclaration"].as_bool().unwrap_or(false)),false)},
 "textDocument/documentHighlight"=>{result=locations(&service,service.highlights(uri,at),true)},
 "textDocument/prepareRename"=>{let prepared=service.prepare_rename(uri,at);if !prepared.is_null(){result=json!({"range":range(&service,uri,prepared["from"].as_u64().unwrap()as usize,prepared["to"].as_u64().unwrap()as usize),"placeholder":prepared["placeholder"]})}},
 "textDocument/rename"=>{
  if p["textDocument"]["version"].is_number()&&opened.get(uri)!=Some(&p["textDocument"]["version"]){response_error=Some(json!({"code":-32801,"message":"Document changed; request rename again"}));}
  else {match service.rename(uri,at,p["newName"].as_str().unwrap_or("")){
   Err(message)=>response_error=Some(json!({"code":-32602,"message":message})),
   Ok(edits)=>{
    let mut changes=BTreeMap::<String,Vec<J>>::new();
    for edit in edits.as_array().unwrap(){let file=edit["uri"].as_str().unwrap();changes.entry(file.into()).or_default().push(json!({"range":range(&service,file,edit["from"].as_u64().unwrap()as usize,edit["to"].as_u64().unwrap()as usize),"newText":p["newName"]}));}
    result=if versioned_edits {json!({"documentChanges":changes.into_iter().map(|(uri,edits)|json!({"textDocument":{"version":opened.get(&uri).cloned().unwrap_or(J::Null),"uri":uri},"edits":edits})).collect::<Vec<_>>()})}else{json!({"changes":changes})};
   }
  }}
 },
 "textDocument/codeAction"=>{result=json!(p["context"]["diagnostics"].as_array().map(|a|a.iter().filter(|d|d["data"]["replacement"].is_string()).map(|d|json!({"title":d["message"],"kind":"quickfix","diagnostics":[d],"edit":{"changes":{uri:[{"range":d["range"],"newText":d["data"]["replacement"]}]}}})).collect::<Vec<_>>()).unwrap_or_default())},_=>unknown=true}
        if let Some(id) = id {
            send(
                &mut output,
                if let Some(error) = response_error {
                    json!({"jsonrpc":"2.0","id":id,"error":error})
                } else if unknown {
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"Method not found"}})
                } else {
                    json!({"jsonrpc":"2.0","id":id,"result":result})
                },
            )?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_uris_preserve_import_identity_and_decode_native_paths() {
        for (base, relative, expected) in [
            ("file:///tmp/main.lay", "./card.lay", "file:///tmp/card.lay"),
            (
                "file:///C:/work/main.lay",
                "../lib/card.lay",
                "file:///C:/lib/card.lay",
            ),
            (
                "file:///C:/main.lay",
                "../../card.lay",
                "file:///C:/card.lay",
            ),
            (
                "file://server/share/main.lay",
                "./card.lay",
                "file://server/share/card.lay",
            ),
            (
                "file:///tmp/my%20project/main.lay",
                "./中文 #%.lay",
                "file:///tmp/my%20project/%E4%B8%AD%E6%96%87%20%23%25.lay",
            ),
        ] {
            assert_eq!(crate::resolve(base, relative), expected);
        }
        assert_eq!(
            path("file:///tmp/%E4%B8%AD%E6%96%87%20%23%25.lay"),
            "/tmp/中文 #%.lay"
        );
        assert_eq!(path("file://localhost/tmp/main.lay"), "/tmp/main.lay");
        assert_eq!(
            path("file://server/share/main.lay"),
            "//server/share/main.lay"
        );
        assert_eq!(path("file:///tmp/%中文.lay"), "/tmp/%中文.lay");
        assert_eq!(
            path("file:///C:/work/main.lay"),
            if cfg!(windows) {
                "C:/work/main.lay"
            } else {
                "/C:/work/main.lay"
            }
        );
    }
    #[test]
    fn locale_precedence_legacy() {
        for (ui, initial, configured, expected) in [
            ("", "", "", "en"),
            ("fr", "", "", "en"),
            ("zh-TW", "", "", "zh-CN"),
            ("ZH_cn", "auto", "", "zh-CN"),
            ("zh-CN", "en", "", "en"),
            ("en", "zh-CN", "", "zh-CN"),
            ("en", "zh-CN", "auto", "en"),
            ("zh-CN", "auto", "en", "en"),
            ("en", "auto", "zh-CN", "zh-CN"),
            ("zh-CN", "unsupported", "invalid", "zh-CN"),
        ] {
            assert_eq!(chosen_locale(ui, initial, configured), expected);
        }
    }
    #[test]
    fn plaintext_documentation_legacy() {
        let value = plain(
            "**Title**\n\nA *paragraph* with `code`.\n\n- One\n- [Guide](https://example.com)\n\n```lay\nrect(size=(4,2))\n```\n\n<img src=\"https://example.com/private\">",
        );
        assert!(value.contains("Title\n\nA paragraph with code"));
        assert!(value.contains("• Guide (https://example.com)"));
        assert!(value.contains("rect(size="));
        assert!(!value.contains("```"));
        assert!(!value.contains("<img"));
    }
}
