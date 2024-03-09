use std::io::Error;
use std::os::unix::prelude::AsRawFd;
use std::fs::File;
use libc::{ioctl, c_ulong, mmap, munmap, MAP_FAILED, MAP_SHARED, MAP_ANONYMOUS, PROT_READ, PROT_WRITE};
use log::{info, error};

use crate::sys::core::SeimiParams;

use super::ioctl::vmpl_ioctl::VmplFile;

/// SEIMI Constants
const SEIMI_PGD_USER: u64 = 0x12345678; // Replace with actual value
const SEIMI_PGD_SUPER: u64 = 0x87654321; // Replace with actual value

/// SEIMI MMAP Constants
const SEIMI_MMAP_BASE_USER: *mut libc::c_void = 0x1000 as *mut _; // Replace with actual value
const SEIMI_MMAP_BASE_SUPER: *mut libc::c_void = 0x2000 as *mut _; // Replace with actual value

pub fn setup_seimi(dune_fd: &mut VmplFile) -> Result<(), Error> {
    info!("Setting up SEIMI");
    let mut seimi = SeimiParams::new(SEIMI_PGD_USER, SEIMI_PGD_SUPER);
    dune_fd.set_seimi(&mut seimi)?;
    info!("SEIMI setup complete");

    Ok(())
}

pub fn sa_alloc(length: usize, need_ro: bool) -> Result<*mut libc::c_void, Error> {
    let seimi_user = unsafe { mmap(SEIMI_MMAP_BASE_USER, length, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANONYMOUS, -1, 0) };
    if seimi_user == MAP_FAILED {
        return Err(Error::last_os_error());
    }

    if need_ro {
        let seimi_super = unsafe { mmap(SEIMI_MMAP_BASE_SUPER, length, PROT_READ, MAP_SHARED | MAP_ANONYMOUS, -1, 0) };
        if seimi_super == MAP_FAILED {
            return Err(Error::last_os_error());
        }
    }
    Ok(seimi_user)
}

pub fn sa_free(addr: *mut libc::c_void, length: usize) -> Result<(), std::io::Error> {
    let rc = unsafe { munmap(addr, length) };
    if rc < 0 {
        return Err(std::io::Error::last_os_error());
    }

    Ok(())
}