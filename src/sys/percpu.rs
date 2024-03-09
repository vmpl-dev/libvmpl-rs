use libc::mmap;
use libc::FS_BASE;
use libc::MAP_ANONYMOUS;
use libc::MAP_FAILED;
use libc::MAP_PRIVATE;
use libc::PROT_READ;
use libc::PROT_WRITE;
use log::{error, info};
use std::arch::asm;
use std::fmt::Pointer;
use std::mem;
use std::mem::{offset_of, size_of};
use std::os::raw::c_char;
use std::os::raw::c_int;
use std::ptr::null_mut;
use std::ptr;
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

use crate::error::VmplError;
use crate::ghcb::vc_init;
use crate::ghcb::Ghcb;
use crate::globals::NR_GDT_ENTRIES;
use crate::mm::PGSIZE;
use crate::sys::serial_init;

use super::core::VmplSegs;
use super::core::VmsaSeg;
use super::ioctl::vmpl_ioctl::VmplFile;

const NR_GDT_ENTRIES: usize = 0; // 需要根据实际的值来修改

#[cfg(feature = "xsave")]
const XSAVE_SIZE: usize = 4096;
#[cfg(feature = "xsave")]
const XCR_XFEATURE_ENABLED_MASK: u32 = 0x00000000;
/// Global variable to indicate whether VMPL has been booted
const VMPL_BOOTED: bool = false;

#[repr(C)]
pub struct DunePerCpu {
    percpu_ptr: Box<DunePerCpu>,
    tmp: u64,
    kfs_base: u64,
    ufs_base: u64,
    in_usermode: u64,
    tss: TaskStateSegment,
    gdt: [u64; NR_GDT_ENTRIES],
    ghcb: *mut Ghcb,
    xsave_area: *mut c_char,
    xsave_mask: u64,
    pkey: c_int,
}

pub fn dune_get_user_fs() -> u64 {
    let ptr: *mut u8;
    unsafe {
        asm!(
            "movq gs:{}, {}",
            in(reg) offset_of!(DunePerCpu, ufs_base),
            out(reg) ptr,
            options(nostack, preserves_flags),
        );
    }
    ptr as u64
}

pub fn dune_set_user_fs(fs_base: u64) {
    unsafe {
        asm!(
            "movq {}, gs:{}",
            in(reg) fs_base,
            in(reg) offset_of!(DunePerCpu, ufs_base),
            options(nostack, preserves_flags),
        );
    }
}

use crate::globals::GD_TSS;
use std::fmt::Display;

impl DunePerCpu {

    fn setup_gdt(&mut self) {
        GDT.init();
        let mut gdt = [0; NR_GDT_ENTRIES];
        let tss = VmsaSeg::new(GD_TSS, 0x0089, mem::size_of_val(&self.tss), &mut self.tss);
        gdt[GD_TSS as usize] = tss.as_u64();
        self.gdt = gdt;
    }

    fn setup_vmsa(&mut self, fd: VmplFile) -> Result<(), VmplError> {
        let fs = VmsaSeg::fs(self.kfs_base);
        let gs = VmsaSeg::gs(self as *mut _ as u64);
        let tr = VmsaSeg::new(GD_TSS, 0x0089,  mem::size_of_val(&self.tss), &mut self.tss);
        let gdtr = VmsaSeg::new(0, 0, mem::size_of_val(&self.gdt) as usize - 1, &mut self.gdt as *mut _ as u64);
        let idtr = VmsaSeg::new(0, 0, mem::size_of_val(&IDT) - 1, &IDT as *const _ as u64);
        let segs = VmplSegs::new(fs, gs, gdtr, idtr, tr);
        let mut segs = Box::new(segs);

        fd.set_segs(segs.as_mut())?;

        Ok(())
    }

    pub fn get_ghcb(&self) -> *mut Ghcb {
        self.ghcb
    }

    fn setup_safe_stack(&mut self) -> Result<(), VmplError> {
        println!("setup safe stack");
        let safe_stack = unsafe {
            mmap(
                std::ptr::null_mut(),
                PGSIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if safe_stack == MAP_FAILED {
            return Err(VmplError::Sys(libc::ENOMEM));
        }

        let safe_stack = unsafe { safe_stack.offset(PGSIZE as isize) };
        self.tss.iomap_base = size_of::<TaskStateSegment>() as u16;

        for i in 0..7 {
            self.tss.interrupt_stack_table[i] = VirtAddr::new(safe_stack as u64);
        }

        self.tss.privilege_stack_table[0] = VirtAddr::new(safe_stack as u64);

        Ok(())
    }

    pub fn alloc(&mut self) -> Result<Box<DunePerCpu>, VmplError> {
        info!("vmpl_alloc_percpu");

        let fs_base: u64;
        let gs_base: u64;

        unsafe {
            asm!(
                "rdfsbase {}",
                out(reg) fs_base,
                options(nostack, preserves_flags),
            );
            info!("dune: FS base at 0x{:x} with rdfsbase", fs_base);

            asm!(
                "rdgsbase {}",
                out(reg) gs_base,
                options(nostack, preserves_flags),
            );
            info!("dune: GS base at 0x{:x} with rdgsbase", gs_base);
        }

        let percpu = unsafe {
            mmap(
                std::ptr::null_mut(),
                PGSIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            ) as *mut DunePerCpu
        };

        if percpu == MAP_FAILED as *mut DunePerCpu {
            return Err(VmplError::Sys(libc::ENOMEM));
        }

        unsafe {
            (*percpu).kfs_base = fs_base;
            (*percpu).ufs_base = fs_base;
            (*percpu).in_usermode = 1;
            (*percpu).ghcb = std::ptr::null_mut();
        }

        match self.setup_safe_stack() {
            Ok(_) => {}
            Err(rc) => {
                error!("dune: failed to setup safe stack");
                unsafe { libc::munmap(percpu as *mut _, PGSIZE) };
                return Err(rc);
            }
        }

        Ok(unsafe { Box::from_raw(percpu) })
    }

    #[cfg(feature = "xsave")]
    fn xsave_begin(&mut self) -> Result<(), VmplError> {
        println!("xsave begin");
        let mut mask: u64 = 0x07;
        unsafe {
            asm!(
                "xgetbv",
                in("ecx") XCR_XFEATURE_ENABLED_MASK,
                out("eax") mask,
            );
        }

        println!("xsave mask: {:x}", mask);
        let xsave_area = vec![0u8; XSAVE_SIZE];
        if xsave_area.is_empty() {
            eprintln!("dune: failed to allocate xsave area");
            return Err(VmplError::Sys(libc::ENOMEM));
        }

        println!("xsave area at {:?}", xsave_area.as_ptr());
        unsafe {
            asm!(
                ".byte 0x48, 0x0f, 0xae, 0x27",
                in("rdi") xsave_area.as_ptr(),
                in("eax") mask,
                in("edx") 0x00,
            );
        }

        self.xsave_mask = mask;
        self.xsave_area = Some(xsave_area);

        Ok(())
    }

    #[cfg(feature = "xsave")]
    fn xsave_end(&mut self) -> Result<(), VmplError> {
        let mask = self.xsave_mask;
        unsafe {
            asm!(
                "xsetbv",
                in("ecx") XCR_XFEATURE_ENABLED_MASK,
                in("eax") mask,
                in("edx") (mask >> 32),
            );

            asm!(
                ".byte 0x48, 0x0f, 0xae, 0x2f",
                in("rdi") percpu.xsave_area.as_ref().unwrap().as_ptr(),
                in("eax") mask,
                in("edx") 0x00,
            );
        }

        self.xsave_area = None;

        println!("xsave end");
        Ok(())
    }

    #[cfg(not(feature = "xsave"))]
    fn xsave_begin(&mut self) -> Result<(), VmplError> {
        Ok(())
    }

    #[cfg(not(feature = "xsave"))]
    fn xsave_end(&mut self) -> Result<(), VmplError> {
        Ok(())
    }

    #[cfg(feature = "dune")]
    fn dune_boot(&mut self) -> Result<(), VmplError> {
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
    fn dune_boot(&mut self) -> Result<(), VmplError> {
        Ok(())
    }

    pub fn pre_init(&mut self, fd: VmplFile) -> Result<(), VmplError> {
        info!("vmpl_init_pre");

        self.setup_gdt();

        if let Err(rc) = self.setup_vmsa(fd) {
            error!("dune: failed to setup vmsa");
            return Err(rc);
        }

        if let Err(rc) = self.xsave_begin() {
            error!("dune: failed to setup xsave");
            return Err(rc);
        }

        Ok(())
    }

    pub fn post_init(&mut self, dune_fd: VmplFile) -> Result<(), VmplError> {
        info!("vmpl_init_post");

        self.in_usermode = 0;
        self.xsave_end()?;
        self.ghcb = vc_init(dune_fd);

        serial_init();

        // write fsbase and gsbase use x86_64
        x86_64::instructions::segmentation::FS;


        VMPL_BOOTED = true;

        self.dune_boot()?;
        // self.init_test();
        // self.init_banner();
        // self.init_stats();
        Ok(())
    }
}

impl Display for DunePerCpu {
    #[cfg(not(feature = "debug"))]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DunePerCpu\n")?;
        write!(f, "PerCpu Entry:\n")?;
        write!(f, "percpu_ptr: {:p}\n", self.percpu_ptr)?;
        write!(f, "kfs_base: {:#x} ufs_base: {:#x}\n", self.kfs_base, self.ufs_base)?;
        write!(f, "in_usermode: {}\n", self.in_usermode)?;
        write!(f, "tss: {:p} gdt: {:p}", &self.tss, self.gdt)?;
        write!(f, "ghcb: {:p}", self.ghcb)?;
        write!(f, "lstar: {:p} vsyscall: {:p}", self.lstar, self.vsyscall)?;
        f.write_str("\n")
    }

    #[cfg(feature = "debug")]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DunePerCpu\n");
        write!(f, "PerCpu Entry:\n");
        write!(f, "percpu_ptr: %lx\n", self.percpu_ptr);
        write!(f, "kfs_base: %lx ufs_base: %lx\n", self.kfs_base, self.ufs_base);
        write!(f, "in_usermode: %lx", self.in_usermode);
        write!(f, "tss: %p gdt: %p", &self.tss, self.gdt);
        write!(f, "ghcb: %p", self.ghcb);
        write!(f, "lstar: %p vsyscall: %p", self.lstar, self.vsyscall);
        write!(f, "VMPL Configs:\n");
        write!(f, "{}", self.idt);
        write!(f, "{}", self.gdt);
        write!(f, "{}", self.tss);
        write!(f, "{}", self.ghcb);
        f.write_str("\n")
    }
}

impl Drop for DunePerCpu {
    fn drop(&mut self) {
        log::debug!("vmpl_free_percpu");

        if !self.ghcb.is_null() {
            unsafe {
                libc::free(self.ghcb as *mut libc::c_void);
                self.ghcb = ptr::null_mut();
            }
        }

        if !self.xsave_area.is_null() {
            unsafe {
                libc::free(self.xsave_area as *mut libc::c_void);
                self.xsave_area = ptr::null_mut();
            }
        }

        unsafe {
            libc::munmap(self as *mut _ as *mut libc::c_void, PGSIZE);
        }
    }
}