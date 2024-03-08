use x86_64::VirtAddr;
use x86_64::PhysAddr;
use x86_64::addr::Offset;
use x86_64::instructions::{hlt, wrmsr, rdmsr};
use x86_64::instructions::interrupts::{int3, int, int3_with_ss};
use x86_64::instructions::port::Port;
use x86_64::instructions::tables::{lgdt, lidt, sgdt, sidt};
use x86_64::instructions::tables::{lldt, sldt, ltr, str};
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
use x86_64::registers::control::{Cr3, Cr3Flags, Cr3PageTablePointer};
use x86_64::registers::control::{Cr8};
use x86_64::registers::control::{Efer, EferFlags};
use x86_64::registers::debug::{Dr7, Dr7Flags};
use x86_64::registers::model_specific::{Efer, EferFlags};
use x86_64::registers::msr::{IA32_EFER, IA32_FS_BASE, IA32_GS_BASE, IA32_KERNEL_GS_BASE};
use x86_64::registers::msr::{IA32_SYSENTER_CS, IA32_SYSENTER_EIP, IA32_SYSENTER_ESP};
use x86_64::registers::msr::{IA32_STAR, IA32_LSTAR, IA32_FMASK};
use x86_64::registers::msr::{IA32_TSC, IA32_TSC_AUX};
use x86_64::registers::msr::{IA32_APIC_BASE, IA32_FEATURE_CONTROL, IA32_MISC_ENABLE};
use x86_64::registers::mxcsr::Mxcsr;
use x86_64::registers::rflags::RFlags;
use x86_64::registers::segmentation::{Cs, Ds, Es, Fs, Gs, Ss};
use x86_64::structures::gdt::DescriptorFlags;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentDescriptor, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::structures::DescriptorTablePointer;

const MAX_LINE_LENGTH: usize = 256;

/*
 * We use the same general GDT layout as Linux so that can we use
 * the same syscall MSR values. In practice only code segments
 * matter, since ia-32e mode ignores most of segment values anyway,
 * but just to be extra careful we match data as well.
 */
const GD_KT: u32 = 0x10;
const GD_KD: u32 = 0x18;
const GD_UD: u32 = 0x28;
const GD_UT: u32 = 0x30;
const GD_TSS: u32 = 0x38;
const GD_TSS2: u32 = 0x40;
const NR_GDT_ENTRIES: u32 = 9;

const KERNEL_CODE32: u64 = 0x00cf9b000000ffff; // [G], [D], L, AVL, [P], DPL=0, [1], [1], C, [R], [A]
const KERNEL_CODE64: u64 = 0x00af9b000000ffff; // [G], D, [L], AVL, [P], DPL=0, [1], [1], C, [R], [A]
const KERNEL_DATA: u64 = 0x00cf93000000ffff; // [G], [B], L, AVL, [P], DPL=0, [1], [0], E, [W], [A]
const USER_CODE32: u64 = 0x00cffb000000ffff; // [G], [D], L, AVL, [P], DPL=3, [1], [1], C, [R], [A]
const USER_DATA: u64 = 0x00cff3000000ffff; // [G], [D], L, AVL, [P], DPL=3, [1], [0], E, [W], [A]
const USER_CODE64: u64 = 0x00affb000000ffff; // [G], D, [L], AVL, [P], DPL=3, [1], [1], C, [R], [A]
const TSS: u64 = 0x0080890000000000; // [G], B, L, AVL, [P], DPL=0, [0], [0], [0], [0], [0]
const TSS2: u64 = 0x0000000000000000; // [G], B, L, AVL, [P], DPL=0, [0], [0], [0], [0], [0]

const VSYSCALL_ADDR: u64 = 0xffffffffff600000;

const RUN_VMPL_DEV_NAME: &str = "/dev/vmpl";

macro_rules! BIT {
    ($x:expr) => (1 << $x);
}


//
// External symbol support:
//   To better control the expected type of value in the external symbol,
//   create getter and, optionally, setter functions for accessing the
//   sysmbols.
//
macro_rules! extern_symbol_u64_ro {
    ($name: ident, $T: ty) => {
        paste::paste! {
            extern "C" {
                static $name: $T;
            }
            pub fn [<get_ $name>]() -> u64 {
                unsafe {
                    $name as u64
                }
            }
        }
    };
}

macro_rules! extern_symbol_virtaddr_ro {
    ($name: ident, $T: ty) => {
        paste::paste! {
            extern "C" {
                static $name: $T;
            }
            pub fn [<get_ $name>]() -> VirtAddr {
                unsafe {
                    VirtAddr::new($name as u64)
                }
            }
        }
    };
}

macro_rules! extern_symbol_u64_rw {
    ($name: ident, $T1: ty) => {
        paste::paste! {
            extern "C" {
                static mut $name: $T1;
            }
            pub fn [<get_ $name>]() -> u64 {
                unsafe {
                    $name as u64
                }
            }
            pub fn [<set_ $name>](value: u64) {
                unsafe {
                    $name = value;
                }
            }
        }
    };
}

// extern_symbol_u64_ro!(sev_encryption_mask, u64);
// extern_symbol_virtaddr_ro!(svsm_begin, u64);
// extern_symbol_virtaddr_ro!(svsm_end, u64);
// extern_symbol_virtaddr_ro!(svsm_sbss, u64);
// extern_symbol_virtaddr_ro!(svsm_ebss, u64);
// extern_symbol_virtaddr_ro!(svsm_sdata, u64);
// extern_symbol_virtaddr_ro!(svsm_edata, u64);
// extern_symbol_virtaddr_ro!(svsm_secrets_page, u64);
// extern_symbol_virtaddr_ro!(svsm_cpuid_page, u64);
// extern_symbol_u64_ro!(svsm_cpuid_page_size, u64);
// extern_symbol_virtaddr_ro!(bios_vmsa_page, u64);
// extern_symbol_virtaddr_ro!(guard_page, u64);
// extern_symbol_virtaddr_ro!(early_ghcb, u64);
// extern_symbol_virtaddr_ro!(early_tss, u64);
// extern_symbol_u64_ro!(gdt64_tss, u64);
// extern_symbol_u64_ro!(gdt64_kernel_cs, u64);
// extern_symbol_virtaddr_ro!(dyn_mem_begin, u64);
// extern_symbol_virtaddr_ro!(dyn_mem_end, u64);
// extern_symbol_u64_rw!(hl_main, u64);
// extern_symbol_u64_rw!(cpu_mode, u64);
// extern_symbol_u64_rw!(cpu_stack, u64);
// extern_symbol_u64_ro!(cpu_start, u64);