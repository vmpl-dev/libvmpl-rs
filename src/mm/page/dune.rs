// -----------------------DUNE PAGE MANAGEMENT-----------------------
#[cfg(feature = "mm")]
pub mod dune {
    use crate::mm::page::common::*;
    use crate::mm::pgtable::{PGSHIFT, PGSIZE, PGTABLE_MMAP_BASE};
    use core::sync::atomic::Ordering;
    use log::info;
    use x86_64::PhysAddr;

    use super::vmpl::*;


    pub fn dune_page_put(pg: *mut Page) {
        unsafe {
            put_page(&mut *pg);
            if (*pg).ref_count.load(Ordering::SeqCst) == 0 {
                dune_page_free(pg);
            }
        }
    }

    #[cfg(feature = "mm")]
    pub fn dune_pa2page(pa: PhysAddr) -> *mut Page {
        vmpl_pa2page(pa)
    }

    #[cfg(feature = "mm")]
    pub fn dune_page2pa(pg: *mut Page) -> PhysAddr {
        vmpl_page2pa(pg)
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_is_from_pool(pa: PhysAddr) -> bool {
        vmpl_page_is_from_pool(pa)
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_get(pg: *mut Page) {
        vmpl_page_get(pg);
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_get_addr(pa: PhysAddr) -> *mut Page {
        if pa < PAGEBASE {
            return std::ptr::null_mut();
        }
        let pg = dune_pa2page(pa);
        if dune_page_is_from_pool(pa) {
            dune_page_get(pg);
        }
        pg
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_mark_addr(pa: PhysAddr) {
        if pa >= PAGEBASE {
            vmpl_page_mark_addr(pa);
        }
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_put_addr(pa: PhysAddr) {
        let pg = dune_pa2page(pa);
        if dune_page_is_from_pool(pa) {
            dune_page_put(pg);
        }
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_init(fd: i32) -> i32 {
        let _ = fd;
        0
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_alloc(fd: i32) -> *mut Page {
        unsafe {
            let pg = vmpl_page_alloc(fd);
            (*pg).vmpl.store(1, Ordering::SeqCst);
            (*pg).ref_count.store(0, Ordering::SeqCst);
            pg
        }
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_free(pg: *mut Page) {
        vmpl_page_free(pg);
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_stats() {
        info!("Dune Page Stats:");
    }

    #[cfg(feature = "mm")]
    pub fn dune_page_test(fd: i32) {
        info!("Dune Page Test");
        let pg = dune_page_alloc(fd);
        dune_page_free(pg);
    }
}