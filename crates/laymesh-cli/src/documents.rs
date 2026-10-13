use laymesh_core::{
    document::DocumentPlan,
    model::{Host, Scene},
};
use laymesh_render::{ExportOptions, JpegPreviewOptions};
use std::path::Path;

pub fn native_host() -> Host {
    configured_host(serde_json::json!({}),Path::new(".").to_path_buf())
}
pub fn configured_host(defaults:serde_json::Value,file:std::path::PathBuf)->Host{
    Host {
        native: true,
        media_ffmpeg_resolver:Some(std::rc::Rc::new(move||super::ffmpeg::require(&defaults,&file,"mp4").map(|p|p.to_string_lossy().into_owned()).map_err(|e|laymesh_core::Diagnostic::new("E_FFMPEG_MISSING",e.to_string(),"",Default::default())))),
        ..Host::default()
    }
}
pub fn scene_warnings(scene: &Scene, mode: &str) {
    if mode == "show" {
        for warning in &scene.warnings {
            eprintln!("{warning}");
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub fn render(
    plan: &DocumentPlan,
    output: &str,
    format: &str,
    options: &ExportOptions,
    all: bool,
    preview: Option<&str>,
    preview_page: usize,
    preview_options: &JpegPreviewOptions,
    warnings: &str,
    host: Host,
) -> Result<(), Box<dyn std::error::Error>> {
    if !all && format != "pdf" {
        return Err("多页文档导出此格式需要 --page N 或 --all-pages；完整文档目前支持 PDF".into());
    }
    if let Some(preview) = preview {
        if super::output_identity(Path::new(output))? == super::output_identity(Path::new(preview))?
        {
            return Err("导出文件与预览文件不能共用路径".into());
        }
    }
    let mut preview_data = None;
    if all {
        let target = Path::new(output);
        let stem = target
            .file_stem()
            .ok_or("输出文件缺少名称")?
            .to_string_lossy();
        let extension = target
            .extension()
            .ok_or("输出文件缺少后缀")?
            .to_string_lossy();
        let stage = BatchStage::new(
            target
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new(".")),
        )?;
        let mut outputs = Vec::new();
        for (i, page) in plan.pages.iter().enumerate() {
            let scene = laymesh_core::document::compile_page(page, host.clone())?;
            scene_warnings(&scene, warnings);
            if warnings == "show" && format == "pdf" {
                for note in laymesh_render::raster_policy::pdf_warnings(&scene, options)? {
                    eprintln!("{note}");
                }
            }
            let path = target.with_file_name(format!("{stem}-{:03}.{extension}", i + 1));
            let staged = stage.0.join(path.file_name().unwrap());
            let data = if format == "pptx" {
                let (data, notes) = laymesh_render::pptx::render_pptx(&scene, options)?;
                if warnings == "show" {
                    for note in notes {
                        eprintln!("{note}");
                    }
                }
                data
            } else {
                laymesh_render::render_export(&scene, format, options)?
            };
            super::export::write_atomic(&staged, &data)?;
            outputs.push((staged, path));
        }
        // All pages have encoded successfully. Each replacement is atomic;
        // publishing the entire batch is not a cross-file transaction.
        for (staged, path) in outputs {
            std::fs::rename(staged, &path)?;
            println!("已导出：{}", path.display());
        }
    } else {
        let mut pdf = laymesh_render::PdfDocument::new();
        for (i, page) in plan.pages.iter().enumerate() {
            let scene = laymesh_core::document::compile_page(page, host.clone())?;
            scene_warnings(&scene, warnings);
            if warnings == "show" {
                for note in laymesh_render::raster_policy::pdf_warnings(&scene, options)? {
                    eprintln!("{note}");
                }
            }
            if preview.is_some() && i + 1 == preview_page {
                preview_data = Some(laymesh_render::render_jpeg_preview(
                    &scene,
                    preview_options,
                )?);
            }
            pdf.append(&scene, options)?;
            // Release layout/font buffers before compiling the next page.
        }
        let data = pdf.finish()?;
        super::export::write_atomic(Path::new(output), &data)?;
        println!("已导出：{output}（{} 页）", plan.pages.len());
    }
    if let (Some(path), Some(data)) = (preview, preview_data) {
        super::export::write_atomic(Path::new(path), &data)?;
        println!("已生成预览：{path}");
    }
    Ok(())
}

struct BatchStage(std::path::PathBuf);
impl BatchStage {
    fn new(parent: &Path) -> std::io::Result<Self> {
        let path = parent.join(format!(
            ".laymesh-pages-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }
}
impl Drop for BatchStage {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
