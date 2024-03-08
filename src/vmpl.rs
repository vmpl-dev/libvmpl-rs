extern crate nix;

use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet};
use nix::sys::signal::Signal;
use nix::errno::Errno;
use nix::Error;
use nix::libc;
use libc::{sched_getaffinity, sched_setaffinity, CPU_SETSIZE, CPU_ISSET, CPU_SET, CPU_ZERO};
use libc::{SIGCHLD, SIGINT, SIGKILL, SIGSTOP, SIGTERM, SIGTSTP};
use libc::{getrlimit, setrlimit, rlimit, RLIMIT_STACK, RLIMIT_DATA};
use libc::{signal, SIG_ERR};
use libc::{mmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE};
use std::process;
use std::arch::asm;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::mem;
use std::mem::{transmute,size_of};
use std::ptr;
use log::{info, error};
use num_cpus;

use crate::sys::core::{Base, Selector, VmplSegs, VmsaConfig};
use crate::sys::percpu::{PerCpu, GD_TSS, IDT};
use crate::sys::ioctl::{vmpl_ioctl_set_segs,vmpl_ioctl_set_syscall};
use crate::sys::apic::{apic_setup, apic_cleanup};
use crate::sys::vc::vc_init;
use crate::start::dune::{__dune_enter, __dune_go_dune, __dune_ret};
use crate::start::dune::{DUNE_RET_EXIT, DUNE_RET_INTERRUPT, DUNE_RET_SIGNAL, DUNE_RET_NOENTER};
use crate::start::dune::{DunePerCpu, DuneConfig, DUNE_FD, DUNE_SIGNAL_INTR_BASE};
use crate::start::dune::{__dune_syscall, __dune_intr, __dune_go_linux};
use crate::start::dune::{__dune_vsyscall_page};
use crate::start::dune::{dune_debug_handle_int, wrmsrl, MSR_FS_BASE, MSR_GS_BASE};

use crate::sys::mm::{setup_mm, vmpl_mm, vmpl_mm_exit, vmpl_mm_stats, vmpl_mm_test};
use crate::sys::seimi::setup_seimi;
use crate::sys::signal::setup_signal;
use crate::sys::syscall::setup_syscall;
use crate::sys::vsyscall::setup_vsyscall;
use crate::sys::idt::setup_idt;
use crate::sys::fpu::{xsave_begin, xsave_end};
use crate::sys::ghcb::Ghcb;
use crate::sys::percpu::{setup_gdt, dump_configs, vmpl_alloc_percpu, vmpl_free_percpu};


// declare global variables
static mut CURRENT_CPU: i32 = 0;
static mut CPU_COUNT: i32 = 0;
static mut VMPL_BOOTED: bool = false;
// define percpu variable for VMPL
static mut percpu: Option<Box<PerCpu>> = None;

fn setup_vmsa(percpu: &mut PerCpu) -> Result<(), i32> {
    let mut segs = Box::new(VmplSegsT {
        fs: Base { base: percpu.kfs_base },
        gs: Base { base: percpu as *mut _ as u64 },
        tr: Selector {
            selector: GD_TSS,
            base: &mut percpu.tss,
            limit: mem::size_of_val(&percpu.tss),
            attrib: 0x0089, // refer to linux-svsm
        },
        gdtr: Base {
            base: &mut percpu.gdt as *mut _ as u64,
            limit: mem::size_of_val(&percpu.gdt) - 1,
        },
        idtr: Base {
            base: &IDT as *const _ as u64,
            limit: mem::size_of_val(&IDT) - 1,
        },
    });

    let rc = vmpl_ioctl_set_segs(DUNE_FD, &mut segs);
    if rc != 0 {
        error!("dune: failed to set segs");
        return Err(rc);
    }

    Ok(())
}

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
            CURRENT_CPU = sched_getaffinity(id, mem::size_of::<libc::cpu_set_t>(), ptr::null_mut()) as i32;
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

fn vmpl_init(map_full: bool) -> Result<(), i32> {
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

fn vmpl_init_pre(percpu: &mut DunePerCpu) -> Result<(), i32> {
    info!("vmpl_init_pre");

    setup_gdt(percpu);

    if let Err(rc) = setup_vmsa(percpu) {
        error!("dune: failed to setup vmsa");
        return Err(rc);
    }

    if let Err(rc) = xsave_begin(percpu) {
        error!("dune: failed to setup xsave");
        return Err(rc);
    }

    Ok(())
}

#[cfg(feature = "dune")]
fn dune_boot(percpu: &mut DunePerCpu) -> Result<(), i32> {
    info!("dune_boot");

    let _gdtr = Gdtr {
        base: &percpu.gdt as *const _ as u64,
        limit: size_of_val(&percpu.gdt) as u16 - 1,
    };

    let _idtr = Idtr {
        base: &IDT as *const _ as u64,
        limit: size_of_val(&IDT) as u16 - 1,
    };

    unsafe {
        asm!(
            // STEP 1: load the new GDT
            "lgdt {0}",

            // STEP 2: initialize data segements
            "mov {1}, %ax",
            "mov %ax, %ds",
            "mov %ax, %es",
            "mov %ax, %ss",

            // STEP 3: long jump into the new code segment
            "mov {2}, %rax",
            "pushq %rax",
            "leaq 1f(%rip),%rax",
            "pushq %rax",
            "lretq",
            "1:",
            "nop",

            // STEP 4: load the task register (for safe stack switching)
            "mov {3}, %ax",
            "ltr %ax",

            // STEP 5: load the new IDT and enable interrupts
            "lidt {4}",
            "sti",

            in(reg) _gdtr,
            in(reg) GD_KD,
            in(reg) GD_KT,
            in(reg) GD_TSS,
            in(reg) _idtr,
            options(nostack, preserves_flags),
        );
    }

    // STEP 6: FS and GS require special initialization on 64-bit
    wrmsrl(MSR_FS_BASE, percpu.kfs_base);
    wrmsrl(MSR_GS_BASE, percpu as *mut _ as u64);

    Ok(())
}

#[cfg(not(feature = "dune"))]
fn dune_boot(_percpu: &mut DunePerCpu) -> Result<(), i32> {
    Ok(())
}

fn vmpl_init_post(percpu: &mut DunePerCpu) -> Result<(), i32> {
    info!("vmpl_init_post");

    percpu.in_usermode = 0;

    if let Err(rc) = xsave_end(percpu) {
        error!("dune: failed to setup xsave");
        return Err(rc);
    }

    percpu.ghcb = vc_init(dune_fd);

    serial_init();

    VMPL_BOOTED = true;

    Ok(())
}

fn vmpl_init_exit() {
    info!("vmpl_init_exit");
    vmpl_mm_exit(&mut vmpl_mm);
    vmpl_free_percpu(percpu);
    apic_cleanup();
}

#[cfg(feature = "dump")]
fn vmpl_init_stats() {
    info!("VMPL Stats:");
    vmpl_mm_stats(&vmpl_mm);
}

#[cfg(not(feature = "dump"))]
fn vmpl_init_stats() {}

#[cfg(feature = "test")]
fn vmpl_init_test() -> i32 {
    vmpl_mm_test(&vmpl_mm);
    0
}

#[cfg(not(feature = "test"))]
fn vmpl_init_test() -> i32 {
    0
}

fn vmpl_init_banner() {
    info!("**********************************************");
    info!("*                                            *");
    info!("*              Welcome to VMPL!              *");
    info!("*                                            *");
    info!("**********************************************");
}

fn vmpl_enter(argc: i32, argv: Vec<String>) -> Result<(), i32> {
    info!("vmpl_enter");

    vmpl_build_assert();

    let percpu = if let Some(percpu) = percpu {
        percpu
    } else {
        match vmpl_alloc_percpu() {
            Ok(percpu) => percpu,
            Err(rc) => {
                error!("dune: failed to allocate percpu struct");
                return Err(rc);
            }
        }
    };

    let mut vmsa_config = VmsaConfig::new(&__dune_ret as *const _ as u64, 0, 0x202);
    let conf = match Box::new(vmsa_config) {
        Ok(conf) => conf,
        Err(rc) => {
            error!("dune: failed to allocate config struct");
            return Err(rc);
        }
    };

    if let Err(rc) = vmpl_init_pre(&mut *percpu) {
        error!("dune: failed to initialize VMPL library");
        return Err(rc);
    }

    dump_configs(&*percpu);

    if let Err(rc) = __dune_enter(dune_fd, &*conf) {
        error!("dune: entry to Dune mode failed");
        return Err(rc);
    }

    dune_boot(&mut *percpu);
    vmpl_init_post(&mut *percpu);
    vmpl_init_test();
    vmpl_init_banner();
    vmpl_init_stats();

    Ok(())
}

fn on_dune_syscall(conf: &mut VmsaConfig) {
    conf.rax = unsafe { libc::syscall(conf.status, conf.rdi, conf.rsi, conf.rdx, conf.r10, conf.r8, conf.r9) };
    __dune_go_dune(dune_fd, conf);
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
            error!("dune: re-entry to Dune mode failed, status is {}", conf.status);
        }
        _ => {
            error!("dune: unknown exit from Dune, ret={}, status={}", conf.ret, conf.status);
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
            error!("dune: re-entry to Dune mode failed, status is {}", conf.status);
        }
        _ => {
            error!("dune: unknown exit from Dune, ret={}, status={}", conf.ret, conf.status);
        }
    }

    std::process::exit(libc::EXIT_FAILURE);
}