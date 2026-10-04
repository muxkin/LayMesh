use laymesh_core::model::resolve;

#[test]
fn resource_paths_match_portable_virtual_host_keys() {
    assert_eq!(
        resolve("/fixtures/main.lay", "./assets/font.ttf"),
        "/fixtures/assets/font.ttf"
    );
    assert_eq!(
        resolve("/fixtures/modules/card.lay", "../image.png"),
        "/fixtures/image.png"
    );
    assert_eq!(
        resolve("/fixtures/main.lay", "/shared/data.csv"),
        "/shared/data.csv"
    );
}

#[cfg(windows)]
#[test]
fn canonical_windows_paths_remain_readable_and_portable() {
    assert_eq!(
        resolve(r"\\?\C:\work\main.lay", "./image.png"),
        "C:/work/image.png"
    );
    assert_eq!(
        resolve(r"\\?\UNC\server\share\main.lay", "./image.png"),
        "//server/share/image.png"
    );
}
