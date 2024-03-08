use core::error::Error;
use std::os::unix::prelude::AsRawFd;
use std::fs::File;
use libc::{ioctl, c_ulong, mmap, munmap, MAP_FAILED, MAP_SHARED, MAP_ANONYMOUS, PROT_READ, PROT_WRITE};
use log::{info, error};

const VMPL_IOCTL_SET_SEIMI: c_ulong = 0x4008_c001;
const SEIMI_PGD_USER: u64 = 0x12345678; // Replace with actual value
const SEIMI_PGD_SUPER: u64 = 0x87654321; // Replace with actual value
const SEIMI_MMAP_BASE_USER: *mut libc::c_void = 0x1000 as *mut _; // Replace with actual value
const SEIMI_MMAP_BASE_SUPER: *mut libc::c_void = 0x2000 as *mut _; // Replace with actual value

struct SeimiParams {
    pgd_user: u64,
    pgd_super: u64,
}

pub fn setup_seimi(dune_fd: &File) -> Result<(), Error> {
    let mut seimi = SeimiParams {
        pgd_user: SEIMI_PGD_USER,
        pgd_super: SEIMI_PGD_SUPER,
    };

    info!("Setting up SEIMI");
    let rc = unsafe { ioctl(dune_fd.as_raw_fd(), VMPL_IOCTL_SET_SEIMI, &mut seimi) };
    if rc < 0 {
        error!("Failed to setup SEIMI: {}", Error::last_os_error());
        return Err(Error::last_os_error());
    }

    Ok(())
}

pub fn sa_alloc(length: usize, need_ro: bool) -> Result<*mut libc::c_void, Error> {
    let seimi_user = unsafe { mmap(SEIMI_MMAP_BASE_USER, length, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANONYMOUS, -1, 0) };
    if seimi_user == MAP_FAILED {
        return Err(Error::last_os_error());
    }

    if !need_ro {
        return Ok(seimi_user);
    }

    let seimi_super = unsafe { mmap(SEIMI_MMAP_BASE_SUPER, length, PROT_READ, MAP_SHARED | MAP_ANONYMOUS, -1, 0) };
    if seimi_super == MAP_FAILED {
        return Err(Error::last_os_error());
    }

    Ok(seimi_user)
}

pub fn sa_free(addr: *mut libc::c_void, length: usize) -> Result<(), Error> {
    let rc = unsafe { munmap(addr, length) };
    if rc < 0 {
        return Err(Error::last_os_error());
    }

    Ok(())
}