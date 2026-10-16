//! Standalone allocator contract probe, including a simulated engine limit.
#[path = "../../crates/laymesh-core/src/wasm_allocator.rs"]
mod allocator;
use std::alloc::{Layout, alloc, alloc_zeroed, dealloc, realloc};
static mut HELD: Vec<(*mut u8, Layout)> = Vec::new();
#[unsafe(no_mangle)]
pub extern "C" fn hold(bytes: usize) -> bool {
    unsafe {
        let layout = Layout::from_size_align(bytes, 16).unwrap();
        let value = alloc(layout);
        if value.is_null() {
            return false;
        }
        value.write(93);
        value.add(bytes - 1).write(17);
        (&mut *std::ptr::addr_of_mut!(HELD)).push((value, layout));
        true
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn release() -> bool {
    unsafe {
        for (value, layout) in (&mut *std::ptr::addr_of_mut!(HELD)).drain(..) {
            if value.read() != 93 || value.add(layout.size() - 1).read() != 17 {
                return false;
            }
            dealloc(value, layout);
        }
    }
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn exercise() -> bool {
    unsafe {
        for alignment in [1, 8, 16, 64, 4096, 65536] {
            let layout = Layout::from_size_align(16387, alignment).unwrap();
            let value = alloc(layout);
            if value.is_null() || value as usize % alignment != 0 {
                return false;
            }
            for i in 0..layout.size() {
                value.add(i).write((i % 251) as u8);
            }
            let grown = realloc(value, layout, 131073);
            if grown.is_null() || grown as usize % alignment != 0 {
                return false;
            }
            for i in 0..layout.size() {
                if grown.add(i).read() != (i % 251) as u8 {
                    return false;
                }
            }
            dealloc(grown, Layout::from_size_align(131073, alignment).unwrap());
            let zeroed = alloc_zeroed(layout);
            if zeroed.is_null() {
                return false;
            }
            for i in 0..layout.size() {
                if zeroed.add(i).read() != 0 {
                    return false;
                }
            }
            dealloc(zeroed, layout);
        }
    }
    true
}
