#[cfg(feature = "mm")]
pub mod common {
    use lazy_static::lazy_static;
    use libc::{mmap, MAP_FAILED, MAP_SHARED, PROT_READ, PROT_WRITE};
    use log::warn;
    use std::fs::File;
    use std::io::Error;
    use std::mem;
    use std::os::unix::io::AsRawFd;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicU64, Ordering};
    use x86_64::PhysAddr;

    use crate::mm::pgtable::{PGSIZE, PGTABLE_MMAP_BASE};
    use std::fmt;

    use crate::mm::page::dune::*;
    use crate::mm::page::vmpl::*;

    pub const PAGEBASE: PhysAddr = PhysAddr::zero(); // Replace with actual value
    pub const MAX_PAGES: usize = 0x100000;
    pub const PAGE_FLAG_POOL: u64 = 1 << 0;
    pub const PAGE_FLAG_MAPPED: u64 = 1 << 1;

    #[derive(Default)]
    pub struct Page {
        link: Option<Box<Page>>,
        ref_count: AtomicU64,
        flags: AtomicU64,
        vmpl: AtomicU64,
    }

    impl fmt::Debug for Page {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("Page")
                .field("ref_count", &self.ref_count)
                .field("flags", &self.flags)
                .field("vmpl", &self.vmpl)
                .finish()
        }
    }

    lazy_static! {
        pub static ref PAGES: [Page; MAX_PAGES] = {
            let mut pages = [Page {
                link: None,
                ref_count: AtomicU64::new(0),
                flags: AtomicU64::new(0),
                vmpl: AtomicU64::new(0),
            }; MAX_PAGES];
            pages
        };
    }

    pub static mut NUM_DUNE_PAGES: i32 = 0;
    pub static mut NUM_VMPL_PAGES: i32 = 0;

    pub fn do_mapping(fd: &File, phys: PhysAddr, len: usize) -> Result<*mut libc::c_void, Error> {
        let addr = unsafe {
            mmap(
                (PGTABLE_MMAP_BASE + phys.as_u64()).as_u64() as *mut libc::c_void,
                len,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd.as_raw_fd(),
                phys.as_u64() as libc::off_t,
            )
        };

        if addr == MAP_FAILED {
            return Err(Error::last_os_error());
        }

        log::debug!("Marking page {:x}-{:x} as mapped", phys, phys + len as u64);
        for i in (0..len).step_by(PGSIZE) {
            let pg = unsafe { &mut *vmpl_pa2page(phys + i as u64) };
            pg.flags.store(PAGE_FLAG_MAPPED, Ordering::SeqCst);
        }

        Ok(addr)
    }

    pub fn get_page(pg: &mut Page) {
        assert!(!PAGES.is_none());
        assert!(pg as *mut _ >= PAGES.unwrap().as_ptr());
        assert!(pg as *mut _ < unsafe { PAGES.unwrap().as_ptr().offset(MAX_PAGES as isize) });
        assert_eq!(pg.vmpl.load(Ordering::SeqCst), 1);

        pg.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn put_page(pg: &mut Page) {
        assert!(!PAGES.is_none());
        assert!(pg as *mut _ >= PAGES.unwrap().as_ptr());
        assert!(pg as *mut _ < unsafe { PAGES.unwrap().as_ptr().offset(MAX_PAGES as isize) });
        assert_eq!(pg.vmpl.load(Ordering::SeqCst), 1);
        assert!(pg.ref_count.load(Ordering::SeqCst) > 0);

        pg.ref_count.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn page_init(fd: i32) -> Result<(), i32> {
        unsafe {
            PAGES = Some(
                NonNull::new(libc::malloc(mem::size_of::<Page>() * MAX_PAGES) as *mut Page)
                    .ok_or(libc::ENOMEM)?,
            );

            if vmpl_page_init(fd) != 0 {
                return Err(libc::ENOMEM);
            }

            if dune_page_init(fd) != 0 {
                return Err(libc::ENOMEM);
            }
        }

        Ok(())
    }

    pub fn page_exit() {
        unsafe {
            libc::free(PAGES.unwrap().as_ptr() as *mut libc::c_void);
            vmpl_page_exit();
            dune_page_exit();
        }
    }

    pub fn page_stats() {
        println!("Page Stats:");
        vmpl_page_stats();
        dune_page_stats();
    }

    #[test]
    pub fn page_test(vmpl_fd: i32) {
        log::info!("Page Test");
        unsafe {
            vmpl_page_test(vmpl_fd);
            dune_page_test(vmpl_fd);
        }
        log::info!("Page Test Passed");
    }
}
