//! Zero-allocation invariant: the perspective-correction scanning path
//! (finder detection → homography → grid sampling) must not touch the heap.
//!
//! The headline claim of the crate is "zero-allocation scanning"; this test
//! enforces it via a counting global allocator. Payload decoding does
//! allocate fixed-size scratch vectors by design — those are bounded, not
//! zero — so only the allocation-free stages are asserted here.

#![cfg(all(feature = "scan", feature = "alloc"))]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static A: Counting = Counting;

fn reset() -> usize {
    ALLOCS.swap(0, Ordering::Relaxed)
}

#[test]
fn perspective_correction_path_is_allocation_free() {
    // Build a synthetic binarized image containing three solid finder-like
    // squares so finder detection produces candidates.
    let w = 200usize;
    let h = 120usize;
    let mut binary = vec![0u8; w * h];
    let stamp = |img: &mut [u8], cx: usize, cy: usize| {
        for dy in 0..7 {
            for dx in 0..7 {
                let on_border = dy == 0 || dy == 6 || dx == 0 || dx == 6;
                let inner = (2..=4).contains(&dy) && (2..=4).contains(&dx);
                if on_border || inner {
                    img[(cy + dy) * w + cx + dx] = 255;
                }
            }
        }
    };
    stamp(&mut binary, 20, 20);
    stamp(&mut binary, 120, 20);
    stamp(&mut binary, 20, 90);
    // some data-ish noise
    for y in 40..80 {
        for x in 40..100 {
            binary[y * w + x] = if (x + y) % 3 == 0 { 255 } else { 0 };
        }
    }

    let candidates = tpt_barcode::image_scan::finder::find_candidates(&binary, w, h);
    assert!(candidates.len() >= 3);

    reset();

    // The allocation-free path: triple selection, homography, inversion, and
    // grid sampling.
    let (tl, tr, bl) = tpt_barcode::image_scan::finder::select_finder_triple(&candidates).unwrap();
    let module_pt = |x: f64, y: f64| tpt_math_geometry::Point2::from_array([x, y]);
    let src = [
        module_pt(3.5, 3.5),
        module_pt(10.5, 3.5),
        module_pt(3.5, 10.5),
        module_pt(10.5, 10.5),
    ];
    let dst = [
        module_pt(tl.cx as f64, tl.cy as f64),
        module_pt(tr.cx as f64, tr.cy as f64),
        module_pt(bl.cx as f64, bl.cy as f64),
        module_pt(tl.cx as f64 + 7.0, tl.cy as f64 + 7.0),
    ];
    let h_mat = tpt_barcode::image_scan::homography::compute_homography(&src, &dst).unwrap();
    let h_inv = tpt_barcode::image_scan::homography::invert_homography(&h_mat).unwrap();
    let mut grid = [0u8; 14 * 14];
    tpt_barcode::image_scan::homography::sample_grid(&binary, w, h, &h_inv, &mut grid, 14, 14);

    let allocs = reset();
    assert_eq!(
        allocs, 0,
        "perspective-correction path allocated {allocs} times"
    );
}
