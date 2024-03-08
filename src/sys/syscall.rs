use std::io::{Error, ErrorKind};
use std::os::unix::io::RawFd;
use std::os::unix::prelude::AsRawFd;
use std::fs::File;
use libc::{ioctl, c_ulong, mmap, munmap, MAP_FAILED, MAP_SHARED, MAP_ANONYMOUS, PROT_READ, PROT_WRITE};
use log::{info, error};

use crate::start::dune::DUNE_FD;
use crate::start::dune::{__dune_syscall, __dune_syscall_end, __dune_vsyscall_page};
use crate::sys::ioctl::vmpl_ioctl::vmpl_ioctl_set_syscall;

#[cfg(not(feature = "dune"))]
pub fn setup_syscall() -> Result<(), i32> {
    info!("setup syscall");

    let syscall: u64 = &__dune_syscall as *const _ as u64;
    let rc = vmpl_ioctl_set_syscall(DUNE_FD, syscall);
    if rc != 0 {
        error!("dune: failed to set syscall");
        return Err(rc);
    }

    Ok(())
}

#[cfg(not(feature = "dune"))]
pub fn setup_vsyscall() -> i32 {
    info!("vsyscall is not supported");
    0
}