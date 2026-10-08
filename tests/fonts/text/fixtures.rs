// Shared by test code and the standalone experimental gallery exporter only.
pub const TEXT_FAMILY: &[&str] = &[
    "LayMesh Formula Text Latin", "LayMesh Formula Text CJK",
    "LayMesh Formula Text Arabic", "LayMesh Formula Text Indic",
];
pub const TEXT_FONTS: &[(&str, &[u8])] = &[
    ("/text/Latin-Regular.ttf", include_bytes!("Latin-Regular.ttf")),
    ("/text/Latin-Bold.ttf", include_bytes!("Latin-Bold.ttf")),
    ("/text/Latin-Italic.ttf", include_bytes!("Latin-Italic.ttf")),
    ("/text/Latin-BoldItalic.ttf", include_bytes!("Latin-BoldItalic.ttf")),
    ("/text/CJK-Regular.otf", include_bytes!("CJK-Regular.otf")),
    ("/text/CJK-Bold.otf", include_bytes!("CJK-Bold.otf")),
    ("/text/Arabic-Regular.ttf", include_bytes!("Arabic-Regular.ttf")),
    ("/text/Indic-Regular.ttf", include_bytes!("Indic-Regular.ttf")),
];
