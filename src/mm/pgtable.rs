use x86_64::{PhysAddr, VirtAddr};

pub const PGTABLE_MMAP_BASE: PhysAddr = PhysAddr::zero(); // Replace with actual value
pub const PGTABLE_MMAP_SIZE: u64 = 0x0; // Replace with actual value
pub const PGSHIFT: usize = 12;
pub const PGSIZE: usize = 1 << PGSHIFT;
pub const PAGE_SIZE: usize = 1 << PGSHIFT;
pub const PAGE_2MB_SIZE: u64 = 1 << 21;

#[cfg(not(feature = "vm"))]
pub fn pgtable_init(fd: i32) -> Result<(), i32> {
    let _ = fd;
    println!("pgtable init");
    Ok(())
}

#[cfg(not(feature = "vm"))]
pub fn pgtable_cleanup() {
    println!("pgtable cleanup");
}

#[cfg(feature = "vm")]
pub fn pgtable_init(fd: i32) -> Result<(), i32> {
    let _ = fd;
    println!("pgtable init");
    Ok(())
}

#[cfg(feature = "vm")]
pub fn pgtable_cleanup() {
    println!("pgtable cleanup");
}

pub fn pgtable_va_to_pa(va: VirtAddr) -> PhysAddr {
    let _ = va;
    println!("pgtable va to pa");
    PhysAddr::zero()
}

pub fn pgtable_pa_to_va(pa: PhysAddr) -> VirtAddr {
    let _ = pa;
    println!("pgtable pa to va");
    VirtAddr::zero()
}

pub fn pgtable_make_pages_shared(va: VirtAddr, len: usize) -> Result<(), i32> {
    let _ = va;
    let _ = len;
    println!("pgtable make pages shared");
    Ok(())
}

pub fn pgtable_make_pages_private(va: VirtAddr, len: usize) -> Result<(), i32> {
    let _ = va;
    let _ = len;
    println!("pgtable make pages private");
    Ok(())
}

pub fn mem_allocate_frames(len: u64) -> Result<(), i32> {
    let _ = len;
    println!("mem allocate frames");
    Ok(())
}

pub fn mem_free_frames(len: u64) -> Result<(), i32> {
    let _ = len;
    println!("mem free frames");
    Ok(())
}