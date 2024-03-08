
use std::io::{Error, ErrorKind};
use std::os::unix::io::RawFd;
use std::os::unix::prelude::AsRawFd;
use std::fs::File;
use libc::{ioctl, c_ulong, mmap, munmap, MAP_FAILED, MAP_SHARED, MAP_ANONYMOUS, PROT_READ, PROT_WRITE};
use log::{info, error};

pub fn setup_stack(stack_size: usize) -> Result<(), i32> {
    println!("setup stack");

    let mut rl: rlimit = unsafe { std::mem::zeroed() };
    let rc = unsafe { getrlimit(RLIMIT_STACK, &mut rl) };
    if rc != 0 {
        eprintln!("dune: failed to get stack size");
        return Err(rc);
    }

    if rl.rlim_cur < stack_size as u64 {
        rl.rlim_cur = stack_size as u64;
        let rc = unsafe { setrlimit(RLIMIT_STACK, &rl) };
        if rc != 0 {
            eprintln!("dune: failed to set stack size");
            return Err(rc);
        }
    }

    Ok(())
}

pub fn setup_heap(increase_size: usize) -> Result<(), i32> {
    println!("setup heap");

    let mut rl: rlimit = unsafe { std::mem::zeroed() };
    let rc = unsafe { getrlimit(RLIMIT_DATA, &mut rl) };
    if rc != 0 {
        eprintln!("dune: failed to get heap size");
        return Err(rc);
    }

    rl.rlim_cur += increase_size as u64;
    let rc = unsafe { setrlimit(RLIMIT_DATA, &rl) };
    if rc != 0 {
        eprintln!("dune: failed to set heap size");
        return Err(rc);
    }

    Ok(())
}

pub fn mm_init(fd: i32) -> Result<(), i32> {
    println!("mm init");

    let rc = page_init(fd);
    if rc.is_err() {
        eprintln!("dune: failed to init page");
        return Err(rc.err().unwrap());
    }

    let rc = pgtable_init(fd);
    if rc.is_err() {
        eprintln!("dune: failed to init pgtable");
        return Err(rc.err().unwrap());
    }

    Ok(())
}