extern crate nix;

use std::fs::OpenOptions;
use std::mem::transmute;

#[cfg(feature = "dune")]
use std::{mem, process, ptr};
#[cfg(feature = "dune")]
use libc::{sched_getaffinity, sched_setaffinity, CPU_ISSET, CPU_SET, CPU_SETSIZE, CPU_ZERO};
use libc::{signal, SIG_ERR};
use log::{error, info};

use crate::globals::{DUNE_SIGNAL_INTR_BASE, RUN_VMPL_DEV_NAME};
use crate::mm::mm_init;
use crate::start::dune::{__dune_enter, __dune_ret};
use crate::start::dune_register_intr_handler;
use crate::sys::apic::{apic_cleanup, apic_setup};
use crate::sys::core::DuneConfig;

use crate::error::VmplError;
use crate::sys::idt::idt_init;
use crate::sys::signal::signal_init;
use crate::sys::syscall::{setup_syscall, setup_vsyscall};
use crate::sys::{seimi_init, DunePerCpu};

// declare global variables
static mut CURRENT_CPU: i32 = 0;
static mut CPU_COUNT: i32 = 0;
static mut VMPL_BOOTED: bool = false;
// define percpu variable for VMPL
static mut PERCPU: Option<Box<DunePerCpu>> = None;

struct VmplSystem {
    dune_fd: i32,
    percpu: Option<Box<DunePerCpu>>,
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

impl VmplSystem {
    #[cfg(feature = "dune")]
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

    #[cfg(feature = "dune")]
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
                CPU_COUNT = VmplSystem::get_cpu_count();
                assert!(CPU_COUNT > 0);
            }
            CURRENT_CPU = (CURRENT_CPU + 1) % CPU_COUNT;
        }
        unsafe { CURRENT_CPU }
    }

    #[cfg(not(feature = "dune"))]
    fn setup_cpuset() -> VmplError {
        todo!("setup cpuset")
    }

    #[cfg(feature = "dune")]
    fn setup_cpuset() -> i32 {
        info!("setup cpuset");
        let cpu = self.alloc_cpu();
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

        unsafe { dune_register_intr_handler(DUNE_SIGNAL_INTR_BASE + sig as u64, x) };

        Ok(())
    }

    fn init(&self, map_full: bool) -> Result<(), VmplError> {
        info!("vmpl_init");

        let dune_fd = OpenOptions::new()
            .read(true)
            .write(true)
            .open(RUN_VMPL_DEV_NAME)?;

        #[cfg(feature = "mm")]
        mm_init(self.dune_fd)?;
        #[cfg(feature = "seimi")]
        seimi_init(dune_fd)?;
        setup_syscall(dune_fd.as_raw_fd())?;
        setup_vsyscall()?;
        signal_init()?;
        idt_init()?;
        apic_setup()?;

        Ok(())
    }

    fn init_exit(&self) {
        info!("vmpl_init_exit");
        // mm_exit(&mut vmpl_mm);
        // free_percpu(percpu);
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

        let mut config = DuneConfig::new(&__dune_ret as *const _ as u64, 0, 0x202);
        let conf = match Box::new(config) {
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

        // dump_configs(&*percpu);

        unsafe {
            if let Err(rc) = __dune_enter(self.dune_fd, &*conf) {
                error!("dune: entry to Dune mode failed");
                return Err(rc);
            }
        }

        percpu.post_init();

        Ok(())
    }
}

#[cfg(feature = "dune")]
fn on_dune_exit(conf: &mut VmsaConfig) {
    match conf.ret {
        DUNE_RET_EXIT => conf.on_dune_exit(),
        DUNE_RET_INTERRUPT => conf.on_dune_interrupt(),
        DUNE_RET_SIGNAL => conf.on_dune_signal(),
        DUNE_RET_NOENTER => conf.on_dune_noenter(),
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
fn on_dune_exit(conf: &mut DuneConfig) {
    use libc::exit;

    match conf.ret() {
        DUNE_RET_EXIT => conf.on_dune_exit(),
        DUNE_RET_SYSCALL => conf.on_dune_syscall(),
        DUNE_RET_SIGNAL => conf.on_dune_signal(),
        DUNE_RET_NOENTER => conf.on_dune_noenter(),
        _ => {
            error!(
                "dune: unknown exit from Dune, ret={}, status={}",
                conf.ret(),
                conf.status()
            );
        }
    }

    unsafe { exit(libc::EXIT_FAILURE) };
}
