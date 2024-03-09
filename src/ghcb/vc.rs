/* SPDX-License-Identifier: MIT */
/*
 * Copyright (C) 2022 Advanced Micro Devices, Inc.
 *
 * Authors: Carlos Bilbao <carlos.bilbao@amd.com> and
 *          Tom Lendacky <thomas.lendacky@amd.com>
 */

use crate::globals::*;
use crate::*;

use std::arch::asm;
use std::mem::size_of;
use libc::memset;
use x86_64::addr::PhysAddr;
use x86_64::addr::VirtAddr;
use x86_64::instructions::hlt;
use x86_64::registers::control::Cr2;
use x86_64::registers::model_specific::Msr;
use x86_64::structures::idt::*;
use x86_64::structures::paging::frame::PhysFrame;

use std::cmp::max;
use std::cmp::min;

use x86_64::registers::control::Cr4;
use x86_64::registers::control::Cr4Flags;
use x86_64::registers::xcontrol::XCr0;

use self::mm::mem_allocate_frames;
use self::mm::mem_free_frames;
use self::mm::pgtable_make_pages_private;
use self::mm::pgtable_make_pages_shared;
use self::mm::pgtable_pa_to_va;
use self::mm::pgtable_va_to_pa;
use self::mm::PAGE_2MB_SIZE;
use self::mm::PAGE_SIZE;
use self::sys::ioctl::vmpl_ioctl::VmplFile;

use super::ghcb::GHCB_USAGE;
use super::ghcb::GHCB_VERSION_1;
use super::globals::*;
use super::ghcb::get_early_ghcb;
use super::ghcb::SHARED_BUFFER_SIZE;
use super::vmsa::Vmsa;
use super::Ghcb;

/// 2
const GHCB_PROTOCOL_MIN: u64 = 2;
/// 2
const GHCB_PROTOCOL_MAX: u64 = 2;

/// Bits zero, one and four
const GHCB_VMPL_FEATURES: u64 = BIT!(0) | BIT!(1) | BIT!(4);

/// 0xfff
const GHCB_MSR_INFO_MASK: u64 = 0xfff;

macro_rules! GHCB_MSR_INFO {
    ($x: expr) => {
        $x & GHCB_MSR_INFO_MASK
    };
}

macro_rules! GHCB_MSR_DATA {
    ($x: expr) => {
        $x & !GHCB_MSR_INFO_MASK
    };
}

// MSR protocol: SEV Information
/// 0x2
const GHCB_MSR_SEV_INFO_REQ: u64 = 0x002;
/// 0x1
const GHCB_MSR_SEV_INFO_RES: u64 = 0x001;
macro_rules! GHCB_MSR_PROTOCOL_MIN {
    ($x: expr) => {
        (($x) >> 32) & 0xffff
    };
}
macro_rules! GHCB_MSR_PROTOCOL_MAX {
    ($x: expr) => {
        (($x) >> 48) & 0xffff
    };
}

// MSR protocol: GHCB registration
/// 0x12
const GHCB_MSR_REGISTER_GHCB_REQ: u64 = 0x12;
macro_rules! GHCB_MSR_REGISTER_GHCB {
    ($x: expr) => {
        (($x) | GHCB_MSR_REGISTER_GHCB_REQ)
    };
}
/// 0x13
const GHCB_MSR_REGISTER_GHCB_RES: u64 = 0x13;

// MSR protocol: Hypervisor feature support
/// 0x80
const GHCB_MSR_HV_FEATURE_REQ: u64 = 0x080;
/// 0x81
const GHCB_MSR_HV_FEATURE_RES: u64 = 0x081;
macro_rules! GHCB_MSR_HV_FEATURES {
    ($x: expr) => {
        (GHCB_MSR_DATA!($x) >> 12)
    };
}

// MSR protocol: Termination request
/// 0x100
const GHCB_MSR_TERMINATE_REQ: u64 = 0x100;

/// 0
const RESCIND: u32 = 0;
/// 1
const VALIDATE: u32 = 1;

// VMGEXIT exit codes
/// 0x72
const GHCB_NAE_CPUID: u64 = 0x72;
/// 0x7b
const GHCB_NAE_IOIO: u64 = 0x7b;
/// 0x80000010
const GHCB_NAE_PSC: u64 = 0x80000010;
/// 0x80000013
const GHCB_NAE_SNP_AP_CREATION: u64 = 0x80000013;
/// 1
const SNP_AP_CREATE_IMMEDIATE: u64 = 1;
/// 0x80000017
const GHCB_NAE_GET_APIC_IDS: u64 = 0x80000017;
/// 0x80000018
const GHCB_NAE_RUN_VMPL: u64 = 0x80000018;

macro_rules! GHCB_NAE_SNP_AP_CREATION_REQ {
    ($op: expr, $vmpl: expr, $apic: expr) => {
        (($op) | ((($vmpl) as u64) << 16) | ((($apic) as u64) << 32))
    };
}

// GHCB IN/OUT instruction constants
/// Bit 9
const IOIO_ADDR_64: u64 = BIT!(9);
/// Bit 6
const IOIO_SIZE_32: u64 = BIT!(6);
/// Bit 5
const IOIO_SIZE_16: u64 = BIT!(5);
/// Bit 4
const IOIO_SIZE_8: u64 = BIT!(4);
/// Bit 0
const IOIO_TYPE_IN: u64 = BIT!(0);

static mut HV_FEATURES: u64 = 0;

fn vc_vmgexit() {
    unsafe {
        asm!("rep vmmcall");
    }
}

/// Terminate execution of SVSM
pub fn vc_terminate(reason_set: u64, reason_code: u64) -> ! {
    let mut value: u64;

    value = GHCB_MSR_TERMINATE_REQ;
    value |= reason_set << 12;
    value |= reason_code << 16;

    let mut msr = Msr::new(MSR_GHCB);
    unsafe { msr.write(value) };
    vc_vmgexit();

    loop {
        hlt();
    }
}

/// Terminate SVSM with generic SVSM reason
#[inline]
pub fn vc_terminate_svsm_general() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_GENERAL);
}

/// Terminate SVSM due to lack of memory
#[inline]
pub fn vc_terminate_svsm_enomem() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_ENOMEM);
}

/// Terminate SVSM due to firmware configuration error
#[inline]
pub fn vc_terminate_svsm_fwcfg() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_FW_CFG_ERROR);
}

/// Terminate SVSM due to invalid GHCB response
#[inline]
pub fn vc_terminate_svsm_resp_invalid() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_GHCB_RESP_INVALID);
}

/// Terminate SVSM due to a page-related error
#[inline]
pub fn vc_terminate_svsm_page_err() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_SET_PAGE_ERROR);
}

/// Terminate SVSM due to a PSC-related error
#[inline]
pub fn vc_terminate_svsm_psc() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_PSC_ERROR);
}

/// Terminate SVSM due to a BIOS-format related error
#[inline]
pub fn vc_terminate_svsm_bios() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_BIOS_FORMAT);
}

/// Terminate SVSM due to an unhandled #VC exception
#[inline]
pub fn vc_terminate_unhandled_vc() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_UNHANDLED_VC);
}

/// Terminate SVSM with generic GHCB reason
#[inline]
pub fn vc_terminate_ghcb_general() -> ! {
    vc_terminate(GHCB_REASON_CODE_SET, GHCB_TERM_GENERAL);
}

/// Terminate SVSM due to unsupported GHCB protocol
#[inline]
pub fn vc_terminate_ghcb_unsupported_protocol() -> ! {
    vc_terminate(GHCB_REASON_CODE_SET, GHCB_TERM_UNSUPPORTED_PROTOCOL);
}

/// Terminate SVSM due to error related with feature support
#[inline]
pub fn vc_terminate_ghcb_feature() -> ! {
    vc_terminate(GHCB_REASON_CODE_SET, GHCB_TERM_FEATURE_SUPPORT);
}

/// Terminate SVSM due to incorrect SEV features for VMPL1
#[inline]
pub fn vc_terminate_vmpl1_sev_features() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_VMPL1_SEV_FEATURES);
}

/// Terminate SVSM due to incorrect SEV features for VMPL0
#[inline]
pub fn vc_terminate_vmpl0_sev_features() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_VMPL0_SEV_FEATURES);
}

/// Terminate SVSM due to incorrect VMPL level on VMSA
#[inline]
pub fn vc_terminate_svsm_incorrect_vmpl() -> ! {
    vc_terminate(VMPL_REASON_CODE_SET, VMPL_TERM_INCORRECT_VMPL);
}

fn vc_msr_protocol(request: u64) -> u64 {
    unsafe { 
        let response: u64;

        // Create a new MSR object for the GHCB MSR
        let mut msr = Msr::new(MSR_GHCB);

        // Save the current GHCB MSR value
        let value = msr.read();

        // Perform the MSR protocol
        msr.write(request) ;

        // Perform the VMGEXIT
        vc_vmgexit();

        // Read the response
        response = msr.read();

        // Restore the GHCB MSR value
        msr.write(value);

        response
    }
}

fn vc_establish_protocol() {
    let mut response: u64;

    // Request SEV information
    response = vc_msr_protocol(GHCB_MSR_SEV_INFO_REQ);

    // Validate the GHCB protocol version
    if GHCB_MSR_INFO!(response) != GHCB_MSR_SEV_INFO_RES {
        vc_terminate_ghcb_general();
    }

    if GHCB_MSR_PROTOCOL_MIN!(response) > GHCB_PROTOCOL_MAX
        || GHCB_MSR_PROTOCOL_MAX!(response) < GHCB_PROTOCOL_MIN
    {
        vc_terminate_ghcb_unsupported_protocol();
    }

    // Request hypervisor feature support
    response = vc_msr_protocol(GHCB_MSR_HV_FEATURE_REQ);

    // Validate required SVSM feature(s)
    if GHCB_MSR_INFO!(response) != GHCB_MSR_HV_FEATURE_RES {
        vc_terminate_ghcb_general();
    }

    if (GHCB_MSR_HV_FEATURES!(response) & GHCB_VMPL_FEATURES) != GHCB_VMPL_FEATURES {
        vc_terminate_ghcb_feature();
    }

    unsafe {
        HV_FEATURES = GHCB_MSR_HV_FEATURES!(response);
    }
}

fn vc_get_ghcb() -> *mut Ghcb {
    unsafe {
        let va: VirtAddr = PERCPU.ghcb();
        let ghcb: *mut Ghcb = va.as_mut_ptr();

        ghcb
    }
}

unsafe fn vc_perform_vmgexit(ghcb: *mut Ghcb, code: u64, info1: u64, info2: u64) {
    (*ghcb).set_version(GHCB_VERSION_1);
    (*ghcb).set_usage(GHCB_USAGE);

    (*ghcb).set_sw_exit_code(code);
    (*ghcb).set_sw_exit_info_1(info1);
    (*ghcb).set_sw_exit_info_2(info2);

    vc_vmgexit();

    if !(*ghcb).is_sw_exit_info_1_valid() {
        vc_terminate_svsm_resp_invalid();
    }

    let info1: u64 = (*ghcb).sw_exit_info_1();
    if LOWER_32BITS!(info1) != 0 {
        vc_terminate_ghcb_general();
    }
}

/// Each vCPU has two VMSAs: One for VMPL0 (for SVSM) and one for VMPL1 (for
/// the guest).
///
/// The SVSM will use this function to invoke a GHCB NAE event to go back to
/// the guest after handling a request.
///
/// The guest will use the same GHCB NAE event to request something of the SVSM.
///
pub fn vc_run_vmpl(vmpl: VMPL) {
    let ghcb: *mut Ghcb = vc_get_ghcb();

    unsafe {
        vc_perform_vmgexit(ghcb, GHCB_NAE_RUN_VMPL, vmpl as u64, 0);

        (*ghcb).clear();
    }
}

fn vc_cpuid_vmgexit(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let ghcb: *mut Ghcb = vc_get_ghcb();
    let eax: u32;
    let ebx: u32;
    let ecx: u32;
    let edx: u32;

    unsafe {
        (*ghcb).set_rax(leaf as u64);
        (*ghcb).set_rcx(subleaf as u64);
        if leaf == CPUID_EXTENDED_STATE {
            if Cr4::read().contains(Cr4Flags::OSXSAVE) {
                (*ghcb).set_xcr0(XCr0::read_raw());
            } else {
                (*ghcb).set_xcr0(1);
            }
        }

        vc_perform_vmgexit(ghcb, GHCB_NAE_CPUID, 0, 0);

        if !(*ghcb).is_rax_valid()
            || !(*ghcb).is_rbx_valid()
            || !(*ghcb).is_rcx_valid()
            || !(*ghcb).is_rdx_valid()
        {
            vc_terminate_svsm_resp_invalid();
        }

        eax = (*ghcb).rax() as u32;
        ebx = (*ghcb).rbx() as u32;
        ecx = (*ghcb).rcx() as u32;
        edx = (*ghcb).rdx() as u32;

        (*ghcb).clear();
    }

    (eax, ebx, ecx, edx)
}

pub fn vc_outl(port: u16, value: u32) {
    let ghcb: *mut Ghcb = vc_get_ghcb();
    let mut ioio: u64 = (port as u64) << 16;

    ioio |= IOIO_ADDR_64;
    ioio |= IOIO_SIZE_32;

    unsafe {
        (*ghcb).set_rax(value as u64);

        vc_perform_vmgexit(ghcb, GHCB_NAE_IOIO, ioio, 0);

        (*ghcb).clear();
    }
}

pub fn vc_inl(port: u16) -> u32 {
    let ghcb: *mut Ghcb = vc_get_ghcb();
    let mut ioio: u64 = (port as u64) << 16;
    let value: u32;

    ioio |= IOIO_ADDR_64;
    ioio |= IOIO_SIZE_32;
    ioio |= IOIO_TYPE_IN;

    unsafe {
        (*ghcb).set_rax(0);

        vc_perform_vmgexit(ghcb, GHCB_NAE_IOIO, ioio, 0);

        if !(*ghcb).is_rax_valid() {
            vc_terminate_svsm_resp_invalid();
        }

        value = LOWER_32BITS!((*ghcb).rax()) as u32;

        (*ghcb).clear();
    }

    value
}

pub fn vc_outw(port: u16, value: u16) {
    let ghcb: *mut Ghcb = vc_get_ghcb();
    let mut ioio: u64 = (port as u64) << 16;

    ioio |= IOIO_ADDR_64;
    ioio |= IOIO_SIZE_16;

    unsafe {
        (*ghcb).set_rax(value as u64);

        vc_perform_vmgexit(ghcb, GHCB_NAE_IOIO, ioio, 0);

        (*ghcb).clear();
    }
}

pub fn vc_inw(port: u16) -> u16 {
    let ghcb: *mut Ghcb = vc_get_ghcb();
    let mut ioio: u64 = (port as u64) << 16;
    let value: u16;

    ioio |= IOIO_ADDR_64;
    ioio |= IOIO_SIZE_16;
    ioio |= IOIO_TYPE_IN;

    unsafe {
        (*ghcb).set_rax(0);

        vc_perform_vmgexit(ghcb, GHCB_NAE_IOIO, ioio, 0);

        if !(*ghcb).is_rax_valid() {
            vc_terminate_svsm_resp_invalid();
        }

        value = LOWER_16BITS!((*ghcb).rax()) as u16;

        (*ghcb).clear();
    }

    value
}

pub fn vc_outb(port: u16, value: u8) {
    let ghcb: *mut Ghcb = vc_get_ghcb();
    let mut ioio: u64 = (port as u64) << 16;

    ioio |= IOIO_ADDR_64;
    ioio |= IOIO_SIZE_8;

    unsafe {
        (*ghcb).set_rax(value as u64);

        vc_perform_vmgexit(ghcb, GHCB_NAE_IOIO, ioio, 0);

        (*ghcb).clear();
    }
}

pub fn vc_inb(port: u16) -> u8 {
    let ghcb: *mut Ghcb = vc_get_ghcb();
    let mut ioio: u64 = (port as u64) << 16;
    let value: u8;

    ioio |= IOIO_ADDR_64;
    ioio |= IOIO_SIZE_8;
    ioio |= IOIO_TYPE_IN;

    unsafe {
        (*ghcb).set_rax(0);

        vc_perform_vmgexit(ghcb, GHCB_NAE_IOIO, ioio, 0);

        if !(*ghcb).is_rax_valid() {
            vc_terminate_svsm_resp_invalid();
        }

        value = LOWER_8BITS!((*ghcb).rax()) as u8;

        (*ghcb).clear();
    }

    value
}

pub fn vc_register_ghcb(pa: PhysAddr) {
    // Perform GHCB registration
    let response: u64 = vc_msr_protocol(GHCB_MSR_REGISTER_GHCB!(pa.as_u64()));

    // Validate the response
    if GHCB_MSR_INFO!(response) != GHCB_MSR_REGISTER_GHCB_RES {
        vc_terminate_svsm_general();
    }

    if GHCB_MSR_DATA!(response) != pa.as_u64() {
        vc_terminate_svsm_general();
    }

    let mut msr = Msr::new(MSR_GHCB);
    unsafe { msr.write(pa.as_u64()) };
}

const PSC_SHARED: u64 = 2 << 52;
const PSC_PRIVATE: u64 = 1 << 52;
const PSC_ENTRIES: usize = (SHARED_BUFFER_SIZE - size_of::<PscOpHeader>()) / 8;

#[repr(C, packed)]
#[derive(Copy, Clone)]
struct PscOpHeader {
    pub cur_entry: u16,
    pub end_entry: u16,
    pub reserved: u32,
}

#[allow(dead_code)]
impl PscOpHeader {
    pub const fn new() -> Self {
        PscOpHeader {
            cur_entry: 0,
            end_entry: 0,
            reserved: 0,
        }
    }
    funcs!(cur_entry, u16);
    funcs!(end_entry, u16);
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct PscOpData {
    pub data: u64,
}

#[allow(dead_code)]
impl PscOpData {
    pub const fn new() -> Self {
        PscOpData { data: 0 }
    }
    funcs!(data, u64);
}

#[repr(C, packed)]
struct PscOp {
    pub header: PscOpHeader,
    pub entries: [PscOpData; PSC_ENTRIES],
}

#[allow(dead_code)]
impl PscOp {
    pub const fn new() -> Self {
        let h: PscOpHeader = PscOpHeader::new();
        let d: PscOpData = PscOpData::new();

        PscOp {
            header: h,
            entries: [d; PSC_ENTRIES],
        }
    }
    funcs!(header, PscOpHeader);
    funcs!(entries, [PscOpData; PSC_ENTRIES]);
}

pub fn vc_init(fd: VmplFile) -> VirtAddr {
    let ghcb_pa: PhysAddr = pgtable_va_to_pa(get_early_ghcb());

    vc_establish_protocol();
    vc_register_ghcb(ghcb_pa);

    get_early_ghcb()
}

#[cfg(feature = "ghcb")]
fn setup_ghcb(dune_fd: RawFd) -> Result<*mut Ghcb, std::io::Error> {
    log::info!("setup ghcb");

    let ghcb = unsafe {
        mmap(
            GHCB_MMAP_BASE as *mut libc::c_void,
            PAGE_SIZE,
            PROT_READ | PROT_WRITE,
            MAP_SHARED | MAP_FIXED,
            dune_fd,
            0,
        )
    };

    if ghcb == MAP_FAILED {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "dune: failed to map GHCB",
        ));
    }

    log::debug!("dune: GHCB at {:?}", ghcb);
    unsafe {
        memset(ghcb, 0, mem::size_of::<Ghcb>());
        ghcb_set_version(ghcb, GHCB_PROTOCOL_MIN);
        ghcb_set_usage(ghcb, GHCB_DEFAULT_USAGE);
        ghcb_set_sw_exit_code(ghcb, GHCB_NAE_RUN_VMPL);
        ghcb_set_sw_exit_info_1(ghcb, 0);
        ghcb_set_sw_exit_info_2(ghcb, 0);
    }

    Ok(ghcb)
}

#[cfg(feature = "ghcb")]
fn vc_init(dune_fd: RawFd) -> Result<*mut Ghcb, std::io::Error> {
    log::info!("setup GHCB");
    let ghcb_va = setup_ghcb(dune_fd)?;
    if ghcb_va.is_null() {
        log::error!("failed to setup GHCB");
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "failed to setup GHCB",
        ));
    }

    log::info!("setup VC");

    let ghcb_pa = unsafe { pgtable_va_to_pa(ghcb_va as VirtAddr) };
    log::debug!("ghcb_pa: {:x}", ghcb_pa);

    vc_establish_protocol();
    vc_register_ghcb(ghcb_pa);
    vc_set_ghcb(ghcb_va);

    Ok(ghcb_va)
}