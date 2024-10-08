#[cfg(feature = "mm")]
pub mod vm {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use x86_64::VirtAddr;

    use super::{FitAlgorithm, VmplVma};

    pub struct VmplVm {
        vma_dict: HashMap<VirtAddr, VmplVma>,
        fit_algorithm: FitAlgorithm, // Assuming FitAlgorithm is an enum you've defined
        va_start: u64,
        va_end: u64,
        phys_limit: usize,
        mmap_base: usize,
        start_stack: usize,
        lock: Mutex<()>,
    }

    pub fn vm_init(fd: i32) -> Result<(), i32> {
        todo!("vm_init");
    }
}