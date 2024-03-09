extern crate nix;

use libc::{getrlimit, rlimit, setrlimit, RLIMIT_DATA, RLIMIT_STACK};
use libc::{mmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use libc::{sched_getaffinity, sched_setaffinity, CPU_ISSET, CPU_SET, CPU_SETSIZE, CPU_ZERO};
use libc::{signal, SIG_ERR};
use libc::{SIGCHLD, SIGINT, SIGKILL, SIGSTOP, SIGTERM, SIGTSTP};
use log::{error, info};
use nix::errno::Errno;
use nix::libc;
use nix::sys::signal::Signal;
use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet};
use nix::Error;
use num_cpus;
use std::arch::asm;
use std::fs::OpenOptions;
use std::mem;
use std::mem::{size_of, transmute};
use std::os::unix::io::AsRawFd;
use std::process;
use std::ptr;

use crate::start::dune::__dune_vsyscall_page;
use crate::start::dune::{DuneConfig, DunePerCpu, DUNE_FD, DUNE_SIGNAL_INTR_BASE};
use crate::start::dune::{__dune_enter, __dune_go_dune, __dune_ret};
use crate::start::dune::{__dune_go_linux, __dune_intr, __dune_syscall};
use crate::start::dune::{dune_debug_handle_int, wrmsrl, MSR_FS_BASE, MSR_GS_BASE};
use crate::start::dune::{DUNE_RET_EXIT, DUNE_RET_INTERRUPT, DUNE_RET_NOENTER, DUNE_RET_SIGNAL};
use crate::sys::apic::{apic_cleanup, apic_setup};
use crate::sys::core::{Base, Selector, VmplSegs, VmsaConfig};
use crate::sys::ioctl::{vmpl_ioctl_set_segs, vmpl_ioctl_set_syscall};
use crate::sys::percpu::{PerCpu, GD_TSS, IDT};
use crate::sys::vc::vc_init;

use crate::sys::fpu::{xsave_begin, xsave_end};
use crate::sys::ghcb::Ghcb;
use crate::sys::idt::setup_idt;
use crate::sys::mm::{setup_mm, vmpl_mm, vmpl_mm_exit, vmpl_mm_stats, vmpl_mm_test};
use crate::sys::percpu::{dump_configs, setup_gdt, vmpl_alloc_percpu, vmpl_free_percpu};
use crate::sys::seimi::setup_seimi;
use crate::sys::signal::setup_signal;
use crate::sys::syscall::setup_syscall;
use crate::sys::vsyscall::setup_vsyscall;
use crate::sys::DunePerCpu;

// declare global variables
static mut CURRENT_CPU: i32 = 0;
static mut CPU_COUNT: i32 = 0;
static mut VMPL_BOOTED: bool = false;
// define percpu variable for VMPL
static mut PERCPU: Option<Box<PerCpu>> = None;

struct VmplSystem {
    dune_fd: i32,
    percpu: Option<Box<PerCpu>>,
}

impl VmplSystem {
    fn new() -> VmplSystem {
        VmplSystem {
            dune_fd: 0,
            percpu: None,
        }
    }
}

impl Default for VmplSystem {
    fn default() -> VmplSystem {
        VmplSystem::new()
    }
}

impl Drop for VmplSystem {
    fn drop(&mut self) {
        info!("VmplSystem drop");
        self.exit();
    }
}

// fn vmpl_build_assert() {
//     assert!(size_of::<Base>() == 16);
//     assert!(size_of::<Selector>() == 8);
//     assert!(size_of::<VmplSegs>() == 16);
//     assert!(size_of::<VmsaConfig>() == 64);
//     assert!(size_of::<DuneConfig>() == 64);
//     assert!(size_of::<DunePerCpu>() == 64);
//     assert!(size_of::<Ghcb>() == 64);
//     assert!(size_of::<PerCpu>() == 64);
// }

impl VmplSystem {
    fn get_cpu_count() -> i32 {
        info!("get cpu count");
        let nprocs = num_cpus::get() as i32;
        if nprocs < 0 {
            error!("failed to get cpu count");
            return -1;
        }
        info!("{} cpus online", nprocs);
        nprocs
    }

    fn alloc_cpu() -> i32 {
        info!("alloc cpu");
        unsafe {
            if CURRENT_CPU == 0 {
                if let Some(id) = process::id() {
                    CURRENT_CPU =
                        sched_getaffinity(id, mem::size_of::<libc::cpu_set_t>(), ptr::null_mut())
                            as i32;
                }
            }
            if CPU_COUNT == 0 {
                CPU_COUNT = get_cpu_count();
                assert!(CPU_COUNT > 0);
            }
            CURRENT_CPU = (CURRENT_CPU + 1) % CPU_COUNT;
        }
        unsafe { CURRENT_CPU }
    }
    #[cfg(feature = "dune")]
    fn setup_cpuset() -> i32 {
        info!("setup cpuset");
        let cpu = alloc_cpu();
        let mut cpuset: libc::cpu_set_t = unsafe { mem::zeroed() };
        unsafe {
            CPU_ZERO(&mut cpuset);
            CPU_SET(cpu as usize, &mut cpuset);
        }
        if unsafe { sched_setaffinity(0, mem::size_of::<libc::cpu_set_t>(), &cpuset) } == -1 {
            error!("sched_setaffinity");
            return 1;
        }
        info!("running on CPU {}", cpu);
        info!("Thread {} bound to CPU {}", process::id(), cpu);
        0
    }

    fn dune_signal(sig: i32, cb: extern "C" fn(i32)) -> Result<(), i32> {
        println!("dune_signal: register signal {}", sig);
        let x: extern "C" fn(i32) = unsafe { transmute(cb) };

        if unsafe { signal(sig, cb) } == SIG_ERR {
            return Err(-1);
        }

        dune_register_intr_handler(DUNE_SIGNAL_INTR_BASE + sig as u64, x);

        Ok(())
    }

    fn init(&self, map_full: bool) -> Result<(), i32> {
        info!("vmpl_init");

        let dune_fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open(RUN_VMPL_DEV_NAME);

        let dune_fd = match dune_fd {
            Ok(fd) => fd,
            Err(_) => {
                error!("Failed to open {}", RUN_VMPL_DEV_NAME);
                return Err(libc::errno());
            }
        };

        if let Err(rc) = setup_cpuset() {
            error!("dune: unable to setup CPU set");
            return Err(rc);
        }

        if let Err(rc) = setup_mm() {
            error!("dune: unable to setup memory management");
            return Err(rc);
        }

        if let Err(rc) = setup_seimi(dune_fd.as_raw_fd()) {
            error!("dune: unable to setup SEIMI");
            return Err(rc);
        }

        if let Err(rc) = setup_syscall() {
            error!("dune: unable to setup syscall handler");
            return Err(rc);
        }

        if map_full {
            if let Err(rc) = setup_vsyscall() {
                error!("dune: unable to setup vsyscall handler");
                return Err(rc);
            }
        }

        setup_signal();
        setup_idt();

        if let Err(rc) = apic_setup() {
            error!("dune: failed to setup APIC");
            apic_cleanup();
            return Err(libc::ENOMEM);
        }

        Ok(())
    }

    fn init_exit(&self) {
        info!("vmpl_init_exit");
        vmpl_mm_exit(&mut vmpl_mm);
        vmpl_free_percpu(percpu);
        apic_cleanup();
    }

    #[cfg(feature = "dump")]
    fn init_stats(&self) {
        info!("VMPL Stats:");
        vmpl_mm_stats(&vmpl_mm);
    }

    #[cfg(not(feature = "dump"))]
    fn init_stats(&self) {}

    #[cfg(feature = "test")]
    fn init_test(&self) -> i32 {
        vmpl_mm_test(&vmpl_mm);
        0
    }

    #[cfg(not(feature = "test"))]
    fn init_test(&self) -> i32 {
        0
    }

    fn init_banner(&self) {
        info!("**********************************************");
        info!("*                                            *");
        info!("*              Welcome to VMPL!              *");
        info!("*                                            *");
        info!("**********************************************");
    }

    fn enter(&self, percpu: &DunePerCpu) -> Result<(), i32> {
        info!("vmpl_enter");

        self.build_assert();

        let mut vmsa_config = VmsaConfig::new(&__dune_ret as *const _ as u64, 0, 0x202);
        let conf = match Box::new(vmsa_config) {
            Ok(conf) => conf,
            Err(rc) => {
                error!("dune: failed to allocate config struct");
                return Err(rc);
            }
        };

        if let Err(rc) = self.pre_init(&mut *percpu) {
            error!("dune: failed to initialize VMPL library");
            return Err(rc);
        }

        dump_configs(&*percpu);

        unsafe {
            if let Err(rc) = __dune_enter(dune_fd, &*conf) {
                error!("dune: entry to Dune mode failed");
                return Err(rc);
            }
        }

        percpu.post_init();

        Ok(())
    }


}

fn on_dune_syscall(conf: &mut VmsaConfig) {
    conf.rax = unsafe {
        libc::syscall(
            conf.status,
            conf.rdi,
            conf.rsi,
            conf.rdx,
            conf.r10,
            conf.r8,
            conf.r9,
        )
    };

    unsafe {
        __dune_go_dune(dune_fd, conf);
    }
}

#[cfg(feature = "dune")]
fn on_dune_exit(conf: &mut VmsaConfig) {
    match conf.ret {
        DUNE_RET_EXIT => {
            info!("on_dune_exit()");
            unsafe { libc::syscall(libc::SYS_exit, conf.status) };
        }
        DUNE_RET_INTERRUPT => {
            dune_debug_handle_int(conf);
            error!("dune: exit due to interrupt {}", conf.status);
        }
        DUNE_RET_SIGNAL => {
            info!("on_dune_exit()");
            __dune_go_dune(dune_fd, conf);
        }
        DUNE_RET_NOENTER => {
            error!(
                "dune: re-entry to Dune mode failed, status is {}",
                conf.status
            );
        }
        _ => {
            error!(
                "dune: unknown exit from Dune, ret={}, status={}",
                conf.ret, conf.status
            );
        }
    }

    std::process::exit(libc::EXIT_FAILURE);
}

#[cfg(not(feature = "dune"))]
#[no_mangle]
fn on_dune_exit(conf: &mut VmsaConfig) {
    match conf.ret {
        DUNE_RET_EXIT => {
            info!("on_dune_exit()");
            unsafe { libc::syscall(libc::SYS_exit, conf.status) };
        }
        DUNE_RET_SYSCALL => {
            on_dune_syscall(conf);
        }
        DUNE_RET_SIGNAL => {
            info!("on_dune_exit()");
            __dune_go_dune(dune_fd, conf);
        }
        DUNE_RET_NOENTER => {
            error!(
                "dune: re-entry to Dune mode failed, status is {}",
                conf.status
            );
        }
        _ => {
            error!(
                "dune: unknown exit from Dune, ret={}, status={}",
                conf.ret, conf.status
            );
        }
    }

    std::process::exit(libc::EXIT_FAILURE);
}
