use std::ops::ShrAssign;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::alloc::{alloc, dealloc, Layout};
use std::mem::size_of;
use std::ptr::null_mut;
use libc::sched_getcpu;
use x86_64::registers::model_specific::Msr;
use std::arch::asm;

use crate::error::VmplError;

/// APIC related constants
const MSR_APIC_BASE: u32 = 0x1B;
const MSR_APIC_ICR: u32 = 0x830;
const MSR_APIC_EOI: u32 = 0x80B;

const APIC_DM_FIXED: u32 = 0x00000;
const NMI_VECTOR: i32 = 0x02;
const APIC_DM_NMI: u32 = 0x00400;
const APIC_DEST_PHYSICAL: u32 = 0x00000;
const EOI_ACK: u32 = 0x0;

/// APIC routing table
static mut APIC_ROUTING: *mut u32 = null_mut();
static NUM_RT_ENTRIES: AtomicUsize = AtomicUsize::new(0);

pub fn apic_get_id() -> u32 {
    unsafe { 
        let mut value: u64 = Msr::new(MSR_APIC_BASE).read();
        value.shr_assign(24);
        value as u32
    }
}

pub fn apic_setup() -> Result<(), VmplError> {
    log::info!("setup apic");
    let num_rt_entries = num_cpus::get_physical();

    log::debug!("num rt entries: {}", num_rt_entries);
    let layout = Layout::from_size_align(num_rt_entries * size_of::<u32>(), size_of::<u32>()).unwrap();
    unsafe {
        APIC_ROUTING = alloc(layout) as *mut u32;
        if APIC_ROUTING.is_null() {
            log::error!("apic routing table allocation failed");
            return Err(VmplError::ApicSetupFailed(libc::ENOMEM));
        }

        NUM_RT_ENTRIES.store(num_rt_entries, Ordering::SeqCst);
        std::ptr::write_bytes(APIC_ROUTING, 0, num_rt_entries);
        asm!("mfence", options(nomem, nostack));
    }

    Ok(())
}

pub fn apic_cleanup() {
    let num_rt_entries = NUM_RT_ENTRIES.load(Ordering::SeqCst);
    let layout = Layout::from_size_align(num_rt_entries * size_of::<i32>(), size_of::<i32>()).unwrap();
    unsafe { dealloc(APIC_ROUTING as *mut u8, layout) };
}

pub fn apic_init_rt_entry() {
    unsafe { 
        let core_id = sched_getcpu(); 
         *APIC_ROUTING.offset(core_id as isize) = apic_get_id();
         asm!("mfence", options(nomem, nostack));
    };
}

pub fn apic_get_id_for_cpu(cpu: u32, error: &mut bool) -> u32 {
    let num_rt_entries = NUM_RT_ENTRIES.load(Ordering::SeqCst);
    if cpu >= num_rt_entries as u32 {
        *error = true;
        return 0;
    }
    unsafe { *APIC_ROUTING.offset(cpu as isize) }
}

fn __prepare_icr(shortcut: u32, vector: i32, dest: u32) -> u32 {
    let mut icr = shortcut | dest;
    match vector {
        NMI_VECTOR => icr |= APIC_DM_NMI,
        _ => icr |= APIC_DM_FIXED | vector as u32,
    }
    icr
}

pub fn apic_send_ipi(vector: u8, dest_apic_id: u32) {
    let low = __prepare_icr(0, vector as i32, APIC_DEST_PHYSICAL);
    let icr = ((dest_apic_id as u64) << 32) | low as u64;
    unsafe { asm!("wrmsr", in("ecx") MSR_APIC_ICR, in("eax") icr, options(nomem, nostack)) };
}

pub fn apic_eoi() {
    unsafe { asm!("wrmsr", in("ecx") MSR_APIC_EOI, in("eax") EOI_ACK, options(nomem, nostack)) };
}