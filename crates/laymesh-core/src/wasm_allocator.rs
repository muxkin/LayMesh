//! Grow linear memory geometrically, returning all new space to dlmalloc.
//!
//! WebKit can exhaust its process-wide fast-memory reservations when compiler
//! and renderer instances coexist. In its fallback, every memory.grow copies
//! the existing heap. Growing by one 64 KiB page then becomes quadratic. This
//! changes allocation granularity, not capacity: reuse freed blocks, grow only
//! on demand, and retry the minimum request if the larger growth cannot fit.
use std::alloc::{GlobalAlloc, Layout};
use std::arch::wasm32::{memory_grow, memory_size};
use std::ptr;
use std::sync::Mutex;

const PAGE: usize = 65536; // WebAssembly's linear-memory page size.
struct Memory;

// SAFETY: memory.grow supplies fresh, disjoint, zeroed and page-aligned regions.
// Regions remain valid for the instance lifetime; dlmalloc owns their reuse.
unsafe impl dlmalloc::Allocator for Memory {
    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        let minimum = size.div_ceil(PAGE);
        let mut pages = minimum.max(memory_size::<0>());
        let mut previous = memory_grow::<0>(pages);
        if previous == usize::MAX && pages != minimum {
            pages = minimum;
            previous = memory_grow::<0>(pages);
        }
        if previous == usize::MAX {
            return (ptr::null_mut(), 0, 0);
        }
        let start = previous * PAGE;
        let mut bytes = pages * PAGE;
        // Rust allocations cannot wrap past the wasm32 address space.
        if start.wrapping_add(bytes) == 0 {
            bytes -= 16;
        }
        (start as *mut u8, bytes, 0)
    }
    fn remap(&self, _: *mut u8, _: usize, _: usize, _: bool) -> *mut u8 {
        ptr::null_mut()
    }
    fn free_part(&self, _: *mut u8, _: usize, _: usize) -> bool {
        false
    }
    fn free(&self, _: *mut u8, _: usize) -> bool {
        false
    }
    fn can_release_part(&self, _: u32) -> bool {
        false
    }
    fn allocates_zeros(&self) -> bool {
        true
    }
    fn page_size(&self) -> usize {
        PAGE
    }
}

static HEAP: Mutex<dlmalloc::Dlmalloc<Memory>> =
    Mutex::new(dlmalloc::Dlmalloc::new_with_allocator(Memory));
struct Allocator;
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

// SAFETY: the mutex serializes access. Each operation delegates its original
// pointer, size and alignment to the same dlmalloc instance; no host callbacks.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { HEAP.lock().unwrap().malloc(layout.size(), layout.align()) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        unsafe { HEAP.lock().unwrap().calloc(layout.size(), layout.align()) }
    }
    unsafe fn dealloc(&self, value: *mut u8, layout: Layout) {
        unsafe {
            HEAP.lock()
                .unwrap()
                .free(value, layout.size(), layout.align())
        }
    }
    unsafe fn realloc(&self, value: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        unsafe {
            HEAP.lock()
                .unwrap()
                .realloc(value, layout.size(), layout.align(), size)
        }
    }
}
