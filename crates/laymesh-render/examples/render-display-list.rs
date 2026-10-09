//! Render a protocol-3 snapshot using the public native RGBA interface.
//! Used by the native/WASM parity check with identical encoded resources.
use laymesh_render::{DisplayList, raster::render_rgba};
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 4 {
        return Err("Usage: render-display-list FRAME.json OUTPUT.rgba DPI".into());
    }
    let frame: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let mut list: DisplayList = serde_json::from_value(frame["displayList"].clone())?;
    for resource in frame["resources"].as_array().ok_or("Missing resources")? {
        let id = resource["id"].as_str().ok_or("Missing resource ID")?;
        let path = PathBuf::from(resource["path"].as_str().ok_or("Missing resource path")?);
        list.resources
            .get_mut(id)
            .ok_or("Unknown resource ID")?
            .data = std::fs::read(path)?;
    }
    let raster = render_rgba(&list, args[3].parse()?)?;
    std::fs::write(&args[2], raster.rgba)?;
    Ok(())
}
