pub const PGTABLE_MMAP_BASE: u64 = 0x0; // Replace with actual value
pub const PGTABLE_MMAP_SIZE: u64 = 0x0; // Replace with actual value
pub const PGSHIFT: usize = 12;
pub const PGSIZE: usize = 1 << PGSHIFT;

pub fn pgtable_init(fd: i32) -> Result<(), i32> {
    println!("pgtable init");



    Ok(())
}