use std::mem::size_of;
use std::os::raw::c_int;
use std::os::raw::c_char;
use x86_64::structures::tss::TaskStateSegment;
use libc::mmap;
use libc::PROT_READ;
use libc::PROT_WRITE;
use libc::MAP_PRIVATE;
use libc::MAP_ANONYMOUS;
use libc::MAP_FAILED;

use crate::ghcb::Ghcb;
use crate::globals::NR_GDT_ENTRIES;

const NR_GDT_ENTRIES: usize = 0; // 需要根据实际的值来修改

#[repr(C)]
pub struct DunePerCpu {
    percpu_ptr: u64,
    tmp: u64,
    kfs_base: u64,
    ufs_base: u64,
    in_usermode: u64,
    tss: TaskStateSegment,
    gdt: [u64; NR_GDT_ENTRIES],
    ghcb: *mut Ghcb,
    xsave_area: *mut c_char,
    xsave_mask: u64,
    pkey: c_int,
}

pub fn dune_get_user_fs() -> u64 {
    let ptr: *mut u8;
    unsafe {
        asm!(
            "movq gs:{}, {}",
            in(reg) offset_of!(DunePerCpu, ufs_base),
            out(reg) ptr,
            options(nostack, preserves_flags),
        );
    }
    ptr as u64
}

pub fn dune_set_user_fs(fs_base: u64) {
    unsafe {
        asm!(
            "movq {}, gs:{}",
            in(reg) fs_base,
            in(reg) offset_of!(DunePerCpu, ufs_base),
            options(nostack, preserves_flags),
        );
    }
}

fn setup_safe_stack(percpu: &mut DunePerCpu) -> Result<(), i32> {
    println!("setup safe stack");
    let safe_stack = unsafe {
        mmap(
            std::ptr::null_mut(),
            PGSIZE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };

    if safe_stack == MAP_FAILED {
        return Err(libc::ENOMEM);
    }
    use x86_64::structures::tss::TaskStateSegment;
    let safe_stack = unsafe { safe_stack.offset(PGSIZE as isize) };
    percpu.tss.tss_iomb = size_of::<TaskStateSegment>() as u16;

    for i in 0..7 {
        percpu.tss.tss_ist[i] = safe_stack as u64;
    }

    percpu.tss.tss_rsp[0] = safe_stack as u64;

    Ok(())
}

pub fn vmpl_alloc_percpu() -> Result<Box<DunePerCpu>, i32> {
    info!("vmpl_alloc_percpu");

    let fs_base: u64;
    let gs_base: u64;

    unsafe {
        asm!(
            "rdfsbase {}",
            out(reg) fs_base,
            options(nostack, preserves_flags),
        );
        info!("dune: FS base at 0x{:x} with rdfsbase", fs_base);

        asm!(
            "rdgsbase {}",
            out(reg) gs_base,
            options(nostack, preserves_flags),
        );
        info!("dune: GS base at 0x{:x} with rdgsbase", gs_base);
    }

    let percpu = unsafe {
        mmap(
            std::ptr::null_mut(),
            PGSIZE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut DunePerCpu
    };

    if percpu == MAP_FAILED as *mut DunePerCpu {
        return Err(libc::ENOMEM);
    }

    unsafe {
        (*percpu).kfs_base = fs_base;
        (*percpu).ufs_base = fs_base;
        (*percpu).in_usermode = 1;
        (*percpu).ghcb = std::ptr::null_mut();
    }

    if setup_safe_stack(&mut *percpu).is_err() {
        error!("dune: failed to setup safe stack");
        unsafe { libc::munmap(percpu as *mut _, PGSIZE) };
        return Err(libc::ENOMEM);
    }

    Ok(unsafe { Box::from_raw(percpu) })
}