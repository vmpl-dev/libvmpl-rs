/// Convert a physical address to a page pointer.
/// VMPL PAGE MANAGEMENT
/// @mbs0221 - 2021-08-10
use std::sync::atomic::{AtomicU64, Ordering};
use std::ptr;

use x86_64::PhysAddr;

use crate::mm::page::common::{MAX_PAGES, PAGEBASE, PAGES};
use crate::mm::pgtable::PGSHIFT;

use super::common::{get_page, put_page, Page, PAGE_FLAG_MAPPED, PAGE_FLAG_POOL};

pub fn vmpl_pa2page(pa: PhysAddr) -> *mut Page {
    assert!(pa >= PAGEBASE);
    assert!(pa < (PAGEBASE + ((MAX_PAGES as u64) << PGSHIFT)));
    unsafe { PAGES.unwrap().as_ptr().offset((pa - PAGEBASE) as isize) }
}

pub fn vmpl_page2pa(pg: *mut Page) -> PhysAddr {
    assert!(pg >= unsafe { PAGES.unwrap().as_ptr() });
    PAGEBASE + ((pg as usize - unsafe { PAGES.unwrap().as_ptr() as usize }) << PGSHIFT)
}

pub fn vmpl_page_is_from_pool(pa: PhysAddr) -> bool {
    let pg = vmpl_pa2page(pa);
    unsafe { (*pg).flags.load(Ordering::SeqCst) & PAGE_FLAG_POOL != 0 }
}

pub fn vmpl_page_is_mapped(pa: PhysAddr) -> bool {
    let pg = vmpl_pa2page(pa);
    unsafe { (*pg).flags.load(Ordering::SeqCst) & PAGE_FLAG_MAPPED != 0 }
}

pub fn vmpl_page_mark(pg: *mut Page) {
    unsafe {
        (*pg).vmpl.store(1, Ordering::SeqCst);
        (*pg).ref_count.store(0, Ordering::SeqCst);
    }
}

pub fn vmpl_page_mark_addr(pa: PhysAddr) {
    if pa >= PAGEBASE {
        vmpl_page_mark(vmpl_pa2page(pa));
    }
}

pub fn vmpl_page_get(pg: *mut Page) {
    get_page(unsafe { &mut *pg });
}

pub fn vmpl_page_get_addr(pa: PhysAddr) -> *mut Page {
    if pa < PAGEBASE {
        return std::ptr::null_mut();
    }
    let pg = vmpl_pa2page(pa);
    if vmpl_page_is_from_pool(pa) {
        vmpl_page_get(pg);
    }
    pg
}

pub fn vmpl_page_put(pg: *mut Page) {
    put_page(unsafe { &mut *pg });
    if unsafe { (*pg).ref_count.load(Ordering::SeqCst) } == 0 {
        unsafe { vmpl_page_free(pg) };
    }
}

pub fn vmpl_page_put_addr(pa: PhysAddr) {
    let pg = vmpl_pa2page(pa);
    if vmpl_page_is_from_pool(pa) {
        vmpl_page_put(pg);
    }
}

pub fn vmpl_page_init(fd: i32) -> i32 {
    todo!("vmpl_page_init: {}", fd);
}

pub fn vmpl_page_alloc(fd: i32) -> *mut Page {
    todo!("vmpl_page_alloc: {}", fd);
}

pub fn vmpl_page_free(pg: *mut Page) {
    todo!("vmpl_page_free: {:?}", pg);
}

pub fn vmpl_page_stats() {
    todo!("vmpl_page_stats");
}

pub fn vmpl_page_test(vmpl_fd: i32) {
    todo!("vmpl_page_test: {}", vmpl_fd);
}