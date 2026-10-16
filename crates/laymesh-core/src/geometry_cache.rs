//! Session-owned memoization of pure geometry during interactive compilation.
//! Keys contain the actual physical geometry, never instance IDs or filenames.
use kurbo::{Affine, BezPath, PathEl};
use sha2::{Digest, Sha256};
use std::cell::Cell;
thread_local! { static INTERACTIVE: Cell<bool> = const { Cell::new(false) }; }
pub(crate) fn enabled() -> bool {
    crate::asset_cache::is_preview() || INTERACTIVE.get()
}
pub(crate) fn interactive<T>(compute: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            INTERACTIVE.set(self.0);
        }
    }
    let _reset = Reset(INTERACTIVE.replace(true));
    compute()
}

pub(crate) fn memo(
    operation: &[u8],
    paths: &[&BezPath],
    settings: &[u8],
    transform: Affine,
    compute: impl FnOnce() -> BezPath,
) -> BezPath {
    if !enabled() {
        return compute();
    }
    let mut hash = Sha256::new();
    hash.update(operation);
    hash.update(settings.len().to_le_bytes());
    hash.update(settings);
    for v in transform.as_coeffs() {
        hash.update(v.to_bits().to_le_bytes());
    }
    for path in paths {
        hash.update(path.elements().len().to_le_bytes());
        for el in path.iter() {
            let zero = kurbo::Point::ZERO;
            let (kind, points, count) = match el {
                PathEl::MoveTo(a) => (0, [a, zero, zero], 1),
                PathEl::LineTo(a) => (1, [a, zero, zero], 1),
                PathEl::QuadTo(a, b) => (2, [a, b, zero], 2),
                PathEl::CurveTo(a, b, c) => (3, [a, b, c], 3),
                PathEl::ClosePath => (4, [zero; 3], 0),
            };
            hash.update([kind]);
            for p in &points[..count] {
                hash.update(p.x.to_bits().to_le_bytes());
                hash.update(p.y.to_bits().to_le_bytes());
            }
        }
    }
    crate::session::geometry::memo_path(hash.finalize().into(), compute)
}
