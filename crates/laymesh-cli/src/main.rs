use laymesh_core::engine::compile_file;
use std::{env, path::Path, process};
mod export;
mod preview;

fn usage(status: i32) -> ! {
    let help = "用法:\n  laymesh validate <file.lay>\n  laymesh inspect <file.lay> --json\n  laymesh render <file.lay> -o <output.svg|pdf|png|jpg|tif|webp|bmp|gif|ico|pnm|pbm|pgm|ppm|pam|tga>\n    [--dpi <number>] [--quality <1–100 (JPEG) / 0–100 (WebP)>]\n    [--compression <fast|default|best (PNG) / none|lzw|deflate|packbits (TIFF)>]\n    [--background <#RRGGBB>] [--webp-lossless <true|false>]\n    [--webp-method <0–6>] [--webp-alpha-quality <0–100>] [--webp-near-lossless <0–100>]\n    [--config <file.json>] [--pdf-image-compression <auto|lossless|jpeg>]\n    [--pdf-jpeg-quality <1–100>] [--pdf-downsample <true|false>]\n    [--pdf-recompress-jpeg <true|false>] [--pdf-preserve-16bit <true|false>] [--pdf-preserve-alpha <true|false>] [--pdf-alpha-background <#RRGGBB>]\n    [--pdf-auto-palette-limit <0–16384>] [--pdf-auto-flatness-threshold <0–1>]\n  laymesh lsp --stdio\n  laymesh preview --stdio\n所有命令支持 --warnings show|hide（默认读取 LAYMESH_WARNINGS）\n几何默认 mm；字号与线宽默认 pt；样式表使用 .lcss";
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
    if args == ["preview", "--stdio"] {
        preview::serve()?;
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
    let mut config_path = None;
    let mut pdf_options = serde_json::json!({});
    let mut options = laymesh_render::ExportOptions::default();
    i = 0;
    while i < rest.len() {
        match rest[i] {
            "--config" => {
                i += 1;
                config_path = rest.get(i).map(Path::new);
                if config_path.is_none() {
                    usage(2);
                }
            }
            flag if flag.starts_with("--pdf-") => {
                i += 1;
                let value = rest.get(i).unwrap_or_else(|| usage(2));
                let key = flag.trim_start_matches("--").replace('-', "_");
                pdf_options[&key] = if matches!(
                    key.as_str(),
                    "pdf_image_compression" | "pdf_alpha_background"
                ) {
                    serde_json::json!(value)
                } else {
                    serde_json::from_str(value).unwrap_or_else(|_| usage(2))
                };
            }
            "-o" | "--output" => {
                i += 1;
                output = rest.get(i).copied()
            }
            flag @ ("--dpi"
            | "--quality"
            | "--compression"
            | "--background"
            | "--webp-lossless"
            | "--webp-method"
            | "--webp-alpha-quality"
            | "--webp-near-lossless") => {
                i += 1;
                let value = rest.get(i).unwrap_or_else(|| usage(2));
                match flag {
                    "--dpi" => {
                        let dpi: f64 = value.parse().unwrap_or_else(|_| usage(2));
                        if !dpi.is_finite() || dpi <= 0. {
                            usage(2);
                        }
                        options.dpi = Some(dpi);
                    }
                    "--quality" => {
                        options.quality = Some(value.parse().unwrap_or_else(|_| usage(2)))
                    }
                    "--compression" => options.compression = Some(value.to_string()),
                    "--background" => options.background = Some(value.to_string()),
                    "--webp-lossless" => {
                        options.webp_lossless = Some(value.parse().unwrap_or_else(|_| usage(2)))
                    }
                    "--webp-method" => {
                        options.webp_method = Some(value.parse().unwrap_or_else(|_| usage(2)))
                    }
                    "--webp-alpha-quality" => {
                        options.webp_alpha_quality =
                            Some(value.parse().unwrap_or_else(|_| usage(2)))
                    }
                    "--webp-near-lossless" => {
                        options.webp_near_lossless =
                            Some(value.parse().unwrap_or_else(|_| usage(2)))
                    }
                    _ => unreachable!(),
                }
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
    let mut explicit = serde_json::to_value(&options)?;
    laymesh_render::config::merge(&mut explicit, &pdf_options);
    options = serde_json::from_value(explicit).unwrap_or_else(|error| {
        eprintln!("{error}");
        usage(2)
    });
    if let Err(error) = options.validate(&format) {
        eprintln!("{error}");
        usage(2)
    }
    let absolute = if Path::new(file).is_absolute() {
        Path::new(file).to_path_buf()
    } else {
        env::current_dir()?.join(file)
    };
    let (defaults, _) = laymesh_render::config::resolve(
        &absolute,
        config_path,
        &serde_json::json!({}),
        &serde_json::json!({}),
    )?;
    options = laymesh_render::config::export_options(&defaults, &format, &options)?;
    if let Err(error) = options.validate(&format) {
        eprintln!("{error}");
        usage(2)
    }
    laymesh_core::asset_cache::preview_mode(format == "pdf");
    let scene = compile_file(file)?;
    if warning_mode == "show" {
        for w in &scene.warnings {
            eprintln!("{w}")
        }
    }
    if warning_mode == "show" && format == "pdf" {
        for w in laymesh_render::raster_policy::pdf_warnings(&scene, &options)? {
            eprintln!("{w}");
        }
    }
    let data = laymesh_render::render_export(&scene, &format, &options)?;
    laymesh_core::asset_cache::preview_mode(false);
    export::write_atomic(Path::new(output), &data)?;
    println!("已导出：{output}");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1)
    }
}
