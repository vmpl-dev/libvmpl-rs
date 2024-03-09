use std::io::Error;
use log::{info, error};

use crate::start::dune::__dune_syscall;
use super::ioctl::vmpl_ioctl::VmplFile;

#[cfg(not(feature = "dune"))]
pub fn setup_syscall(dune_fd: &mut VmplFile) -> Result<(), Error> {
    info!("setup syscall");

    let mut syscall: u64 = &__dune_syscall as *const _ as u64;
    dune_fd.set_syscall(&mut syscall)?;
    Ok(())
}

#[cfg(not(feature = "dune"))]
pub fn setup_vsyscall() -> i32 {
    info!("vsyscall is not supported");
    0
}