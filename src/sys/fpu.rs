use x86_64::VirtAddr;
use x86_64::structures::paging::PageTable;
use x86_64::structures::paging::PhysFrame;
use x86_64::structures::paging::PageTableFlags;
use x86_64::structures::paging::Mapper;

use crate::sys::percpu::DunePerCpu;

#[cfg(feature = "xsave")]
const XSAVE_SIZE: usize = 4096;
#[cfg(feature = "xsave")]
const XCR_XFEATURE_ENABLED_MASK: u32 = 0x00000000;

#[cfg(feature = "xsave")]
fn xsave_begin(percpu: &mut DunePerCpu) -> Result<(), i32> {
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
        return Err(libc::ENOMEM);
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

    percpu.xsave_mask = mask;
    percpu.xsave_area = Some(xsave_area);

    Ok(())
}

#[cfg(feature = "xsave")]
fn xsave_end(percpu: &mut DunePerCpu) -> Result<(), i32> {
    let mask = percpu.xsave_mask;
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

    percpu.xsave_area = None;

    println!("xsave end");
    Ok(())
}

#[cfg(not(feature = "xsave"))]
fn xsave_begin(_percpu: &mut DunePerCpu) -> Result<(), i32> {
    Ok(())
}

#[cfg(not(feature = "xsave"))]
fn xsave_end(_percpu: &mut DunePerCpu) -> Result<(), i32> {
    Ok(())
}
