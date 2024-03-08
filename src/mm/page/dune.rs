// -----------------------DUNE PAGE MANAGEMENT-----------------------
use x86_64::PhysAddr;
use core::sync::atomic::Ordering;
use crate::mm::page::common::*;
use crate::mm::pgtable::{PGSIZE, PGSHIFT, PGTABLE_MMAP_BASE};

fn dune_page_put(pg: *mut Page) {
    unsafe {
        put_page(&mut *pg);
        if (*pg).ref_count.load(Ordering::SeqCst) == 0 {
            dune_page_free(pg);
        }
    }
}

// Aliases for vmpl functions
pub fn dune_pa2page(pa: PhysAddr) -> *mut Page {
    vmpl_pa2page(pa)
}

pub fn dune_page2pa(pg: *mut Page) -> PhysAddr {
    vmpl_page2pa(pg)
}

pub fn dune_page_isfrompool(pa: PhysAddr) -> bool {
    vmpl_page_is_from_pool(pa)
}

pub fn dune_page_get(pg: *mut Page) {
    vmpl_page_get(pg);
}

pub fn dune_page_get_addr(pa: PhysAddr) -> *mut Page {
    if pa < PAGEBASE {
        return std::ptr::null_mut();
    }
    let pg = dune_pa2page(pa);
    if dune_page_isfrompool(pa) {
        dune_page_get(pg);
    }
    pg
}

pub fn dune_page_put_addr(pa: PhysAddr) {
    let pg = dune_pa2page(pa);
    if dune_page_isfrompool(pa) {
        dune_page_put(pg);
    }
}

pub fn dune_page_init(fd: i32) -> i32 {
    0
}

pub fn dune_page_alloc(fd: i32) -> *mut Page {
    unsafe {
        let pg = vmpl_page_alloc(fd);
        (*pg).vmpl.store(1, Ordering::SeqCst);
        (*pg).ref_count.store(0, Ordering::SeqCst);
        pg
    }
}

pub fn dune_page_free(pg: *mut Page) {
    vmpl_page_free(pg);
}

pub fn dune_page_stats() {
    info!("Dune Page Stats:");
}

pub fn dune_page_test(fd: i32) {
    info!("Dune Page Test");
    unsafe {
        let pg = dune_page_alloc(fd);
        dune_page_free(pg);
    }
}