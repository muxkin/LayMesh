use laymesh_core::engine::compile_file;
use std::{env, fs, path::Path, process};

fn usage(status: i32) -> ! {
    let help = "用法:\n  laymesh validate <file.lay>\n  laymesh inspect <file.lay> --json\n  laymesh render <file.lay> -o <output.svg|pdf|png> [--dpi <number>]\n  laymesh lsp --stdio\n所有命令支持 --warnings show|hide（默认读取 LAYMESH_WARNINGS）\n几何默认 mm；字号与线宽默认 pt；样式表使用 .lcss";
    if status == 0 {
        println!("{help}")
    } else {
        eprintln!("{help}")
    };
    process::exit(status)
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args == ["--version"] || args == ["-V"] {
        println!("laymesh {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args == ["--help"] || args == ["-h"] {
        usage(0)
    }
    if args == ["lsp", "--stdio"] {
        laymesh_language::lsp::serve()?;
        return Ok(());
    }
    if args.len() < 2 || args.iter().any(|s| s == "--help" || s == "-h") {
        usage(2)
    }
    let (command, file) = (&args[0], &args[1]);
    if !["validate", "render", "inspect"].contains(&command.as_str()) || !file.ends_with(".lay") {
        usage(2)
    }
    let mut warning_mode = env::var("LAYMESH_WARNINGS").unwrap_or("show".into());
    let mut rest = Vec::new();
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--warnings" {
            i += 1;
            warning_mode = args.get(i).ok_or("--warnings requires show|hide")?.clone()
        } else {
            rest.push(args[i].as_str())
        }
        i += 1;
    }
    if !["show", "hide"].contains(&warning_mode.as_str()) {
        return Err("--warnings / LAYMESH_WARNINGS 须为 show 或 hide".into());
    }
    if command == "inspect" {
        if rest != ["--json"] {
            usage(2)
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&laymesh_language::inspect::inspect_scene(
                &compile_file(file)?
            ))?
        );
        return Ok(());
    }
    if command == "validate" {
        if !rest.is_empty() {
            usage(2)
        };
        let scene = compile_file(file)?;
        if warning_mode == "show" {
            for w in &scene.warnings {
                eprintln!("{w}")
            }
        }
        println!(
            "有效：{file}（{} × {} mm，{} 个顶层实例）",
            scene.width,
            scene.height,
            scene.nodes.len()
        );
        return Ok(());
    }
    let mut output = None;
    let mut dpi = None;
    i = 0;
    while i < rest.len() {
        match rest[i] {
            "-o" | "--output" => {
                i += 1;
                output = rest.get(i).copied()
            }
            "--dpi" => {
                i += 1;
                dpi = Some(
                    rest.get(i)
                        .and_then(|value| value.parse::<f64>().ok())
                        .unwrap_or_else(|| usage(2)),
                )
            }
            _ => usage(2),
        }
        i += 1
    }
    let Some(output) = output else { usage(2) };
    let format = Path::new(output)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !["svg", "pdf", "png"].contains(&format.as_str())
        || dpi.is_some_and(|v| !v.is_finite() || v <= 0. || format != "png")
    {
        usage(2)
    }
    let scene = compile_file(file)?;
    if warning_mode == "show" {
        for w in &scene.warnings {
            eprintln!("{w}")
        }
    }
    let data = match format.as_str() {
        "svg" => laymesh_render::render_svg(&scene)?.into_bytes(),
        "png" => laymesh_render::render_png(&scene, dpi.unwrap_or(scene.export_dpi))?,
        "pdf" => laymesh_render::render_pdf(&scene)?,
        _ => unreachable!(),
    };
    let temporary = format!("{output}.laymesh-{}.tmp", process::id());
    let result = (|| {
        fs::write(&temporary, data)?;
        fs::rename(&temporary, output)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    println!("已导出：{output}");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1)
    }
}
