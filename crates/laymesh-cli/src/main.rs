use laymesh_core::engine::compile_file;
use std::{env, path::Path, process};
mod animation;
mod documents;
mod export;
mod preview;

fn usage(status: i32) -> ! {
    let help = "用法:\n  laymesh animate <file.lay|laydoc> -o <output.mp4|gif> [--width N] [--fps N] [--reduced-motion reduce|no-preference]\n  laymesh render <file.lay|laydoc> -o frame.png --time SECONDS|--step N [--width N]\n  laymesh validate <file.lay>\n  laymesh inspect <file.lay> --json\n  laymesh render <file.lay> -o <output.svg|pdf|pptx|png|jpg|tif|webp|bmp|gif|ico|pnm|pbm|pgm|ppm|pam|tga>\n    [--dpi <number>] [--quality <1–100 (JPEG) / 0–100 (WebP)>]\n    [--compression <fast|default|best (PNG) / none|lzw|deflate|packbits (TIFF)>]\n    [--background <#RRGGBB>] [--webp-lossless <true|false>]\n    [--webp-method <0–6>] [--webp-alpha-quality <0–100>] [--webp-near-lossless <0–100>]\n    [--config <file.json>] [--pdf-image-compression <auto|lossless|jpeg>]\n    [--pdf-jpeg-quality <1–100>] [--pdf-downsample <true|false>]\n    [--pdf-recompress-jpeg <true|false>] [--pdf-preserve-16bit <true|false>] [--pdf-preserve-alpha <true|false>] [--pdf-alpha-background <#RRGGBB>]\n    [--pdf-auto-palette-limit <0–16384>] [--pdf-auto-flatness-threshold <0–1>]\n    [--preview-output <preview.jpg> [--preview-dpi <number>] [--preview-quality <1–100>] [--preview-background <#RRGGBB>]]\n  laymesh render <file.lay> --preview-output <preview.jpg>\n  laymesh lsp --stdio\n  laymesh preview --stdio\n所有命令支持 --warnings show|hide（默认读取 LAYMESH_WARNINGS）\n几何默认 mm；字号与线宽默认 pt；样式表使用 .lcss";
    let document_help = "多页文档：validate/inspect/render 支持 .laydoc；--page N 从 1 开始。\n  laymesh render report.laydoc -o report.pdf\n  laymesh render report.laydoc -o page.png --page 2\n  laymesh render report.laydoc -o page.png --all-pages\n  inspect report.laydoc --json 列出页面；--preview-page N 只选择 JPEG 预览页。";
    if status == 0 {
        println!("{help}\n{document_help}")
    } else {
        eprintln!("{help}\n{document_help}")
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
    if args[0] == "animate"
        || (args[0] == "render" && args.iter().any(|a| a == "--time" || a == "--step"))
    {
        animation::install_cancel();
        return animation::run(&args);
    }
    let (command, file) = (&args[0], &args[1]);
    if !["validate", "render", "inspect"].contains(&command.as_str())
        || !(file.ends_with(".lay") || file.ends_with(".laydoc"))
    {
        usage(2)
    }
    let mut warning_mode = env::var("LAYMESH_WARNINGS").unwrap_or("show".into());
    let mut rest = Vec::new();
    let mut page_number = None;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--warnings" {
            i += 1;
            warning_mode = args.get(i).ok_or("--warnings requires show|hide")?.clone()
        } else if args[i] == "--page" {
            i += 1;
            page_number = Some(
                args.get(i)
                    .ok_or("--page requires a number")?
                    .parse::<usize>()?,
            );
        } else {
            rest.push(args[i].as_str())
        }
        i += 1;
    }
    if !["show", "hide"].contains(&warning_mode.as_str()) {
        return Err("--warnings / LAYMESH_WARNINGS 须为 show 或 hide".into());
    }
    let plan = if file.ends_with(".laydoc") {
        Some(laymesh_core::document::plan_file(file)?)
    } else {
        None
    };
    if let Some(plan) = &plan {
        if let Some(number) = page_number {
            plan.page(number)?;
        }
    } else if page_number.is_some_and(|n| n != 1) {
        return Err("单页 .lay 的页码只能为 1".into());
    }
    if command == "inspect" {
        if rest != ["--json"] {
            usage(2)
        };
        if let Some(plan) = &plan {
            let value = if let Some(number) = page_number {
                let scene = laymesh_core::document::compile_page(
                    plan.page(number)?,
                    documents::native_host(),
                )?;
                serde_json::json!({"document":plan,"pageNumber":number,"inspection":laymesh_language::inspect::inspect_scene(&scene)})
            } else {
                serde_json::json!({"document":plan})
            };
            println!("{}", serde_json::to_string_pretty(&value)?);
            return Ok(());
        }
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
        if let Some(plan) = &plan {
            if let Some(number) = page_number {
                let scene = laymesh_core::document::compile_page(
                    plan.page(number)?,
                    documents::native_host(),
                )?;
                documents::scene_warnings(&scene, &warning_mode);
            } else {
                for page in &plan.pages {
                    let scene =
                        laymesh_core::document::compile_page(page, documents::native_host())?;
                    documents::scene_warnings(&scene, &warning_mode);
                }
            }
            println!("有效：{file}（{} 页）", plan.pages.len());
            return Ok(());
        }
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
    let mut preview_output = None;
    let mut preview_options = laymesh_render::JpegPreviewOptions::default();
    let mut preview_settings = false;
    let mut preview_page = None;
    let mut all_pages = false;
    let mut config_path = None;
    let mut pdf_options = serde_json::json!({});
    let mut options = laymesh_render::ExportOptions::default();
    i = 0;
    while i < rest.len() {
        match rest[i] {
            "--all-pages" => all_pages = true,
            "--preview-page" => {
                i += 1;
                preview_page = Some(
                    rest.get(i)
                        .ok_or("--preview-page requires a number")?
                        .parse::<usize>()?,
                );
            }
            "--preview-output" => {
                i += 1;
                preview_output = Some(*rest.get(i).ok_or("--preview-output requires a JPEG path")?);
            }
            flag @ ("--preview-dpi" | "--preview-quality" | "--preview-background") => {
                i += 1;
                let value = rest.get(i).ok_or("preview option requires a value")?;
                preview_settings = true;
                match flag {
                    "--preview-dpi" => preview_options.dpi = value.parse()?,
                    "--preview-quality" => preview_options.quality = value.parse()?,
                    "--preview-background" => preview_options.background = value.to_string(),
                    _ => unreachable!(),
                }
            }
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
                output = Some(*rest.get(i).ok_or("-o requires an output path")?)
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
    if output.is_none() && preview_output.is_none() {
        usage(2);
    }
    if preview_settings && preview_output.is_none() {
        return Err("--preview-* 参数需要 --preview-output".into());
    }
    if preview_page.is_some() && preview_output.is_none() {
        return Err("--preview-page 需要 --preview-output".into());
    }
    if all_pages
        && (page_number.is_some() || plan.is_none() || output.is_none() || preview_output.is_some())
    {
        return Err(
            "--all-pages 需要 .laydoc 与 -o，且不能与 --page 或 --preview-output 同用".into(),
        );
    }
    if let Some(number) = preview_page {
        if let Some(plan) = &plan {
            plan.page(number)?;
        } else if number != 1 {
            return Err("单页 .lay 的预览页码只能为 1".into());
        }
        if page_number.is_some_and(|p| p != number) {
            return Err("--page 与 --preview-page 必须指向同一页".into());
        }
    }
    if let Some(path) = preview_output {
        let extension = Path::new(path)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if !matches!(extension.to_ascii_lowercase().as_str(), "jpg" | "jpeg") {
            return Err("--preview-output 须为 .jpg 或 .jpeg".into());
        }
        preview_options.validate()?;
    }
    let format = output
        .and_then(|path| Path::new(path).extension().and_then(|s| s.to_str()))
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut explicit = serde_json::to_value(&options)?;
    laymesh_render::config::merge(&mut explicit, &pdf_options);
    options = serde_json::from_value(explicit).unwrap_or_else(|error| {
        eprintln!("{error}");
        usage(2)
    });
    if output.is_some() {
        if let Err(error) = options.validate(&format) {
            eprintln!("{error}");
            usage(2);
        }
    } else if serde_json::to_value(&options)?
        .as_object()
        .unwrap()
        .values()
        .any(|v| !v.is_null())
    {
        return Err("导出参数需要 -o；预览请使用 --preview-dpi/quality/background".into());
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
    if output.is_some() {
        options = laymesh_render::config::export_options(&defaults, &format, &options)?;
        if let Err(error) = options.validate(&format) {
            eprintln!("{error}");
            usage(2);
        }
    }
    laymesh_core::asset_cache::preview_mode(format == "pdf");
    if let Some(plan) = &plan {
        if page_number.is_none() && output.is_some() {
            return documents::render(
                plan,
                output.unwrap(),
                &format,
                &options,
                all_pages,
                preview_output,
                preview_page.unwrap_or(1),
                &preview_options,
                &warning_mode,
            );
        }
    }
    let scene = if let Some(plan) = &plan {
        laymesh_core::document::compile_page(
            plan.page(page_number.or(preview_page).unwrap_or(1))?,
            documents::native_host(),
        )?
    } else {
        compile_file(file)?
    };
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
    let reuse_jpeg = preview_output.is_some()
        && matches!(format.as_str(), "jpg" | "jpeg")
        && preview_options.matches_export(&scene, &options)?;
    let same_path = match (output, preview_output) {
        (Some(a), Some(b)) => output_identity(Path::new(a))? == output_identity(Path::new(b))?,
        _ => false,
    };
    if same_path {
        return Err("导出文件与预览文件不能共用路径".into());
    }
    // Complete both encodings before replacing either destination.
    let preview_data = if preview_output.is_some() && !reuse_jpeg {
        Some(laymesh_render::render_jpeg_preview(
            &scene,
            &preview_options,
        )?)
    } else {
        None
    };
    let data = if output.is_none() {
        Vec::new()
    } else if format == "pptx" {
        let (data, warnings) = laymesh_render::pptx::render_pptx(&scene, &options)?;
        if warning_mode == "show" {
            for warning in warnings {
                eprintln!("{warning}");
            }
        }
        data
    } else {
        laymesh_render::render_export(&scene, &format, &options)?
    };
    laymesh_core::asset_cache::preview_mode(false);
    if let Some(output) = output {
        export::write_atomic(Path::new(output), &data)?;
        println!("已导出：{output}");
    }
    if let Some(preview) = preview_output {
        export::write_atomic(Path::new(preview), preview_data.as_deref().unwrap_or(&data))?;
        println!("已生成预览：{preview}");
    }
    Ok(())
}

fn output_identity(path: &Path) -> std::io::Result<std::path::PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()?.join(path)
    };
    let mut resolved = std::path::PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                resolved.pop();
            }
            _ => {
                resolved.push(component);
                if resolved.exists() {
                    resolved = resolved.canonicalize()?;
                }
            }
        }
    }
    Ok(resolved)
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1)
    }
}
