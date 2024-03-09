/// Memory management module
/// @mbs0221 - 2021-08-10
#[cfg(feature = "mm")]
pub mod mm {
    use libc::{getrlimit, rlimit, setrlimit, RLIMIT_DATA, RLIMIT_STACK};
    use log::{error, info};

    use crate::mm::page::common::page_init;
    use crate::mm::pgtable::pgtable_init;
    use crate::mm::vm::vm_init;

    pub fn setup_stack(stack_size: usize) -> Result<(), i32> {
        info!("setup stack");

        let mut rl: rlimit = unsafe { std::mem::zeroed() };
        let rc = unsafe { getrlimit(RLIMIT_STACK, &mut rl) };
        if rc != 0 {
            error!("dune: failed to get stack size");
            return Err(rc);
        }

        if rl.rlim_cur < stack_size as u64 {
            rl.rlim_cur = stack_size as u64;
            let rc = unsafe { setrlimit(RLIMIT_STACK, &rl) };
            if rc != 0 {
                error!("dune: failed to set stack size");
                return Err(rc);
            }
        }

        Ok(())
    }

    pub fn setup_heap(increase_size: usize) -> Result<(), i32> {
        info!("setup heap");

        let mut rl: rlimit = unsafe { std::mem::zeroed() };
        let rc = unsafe { getrlimit(RLIMIT_DATA, &mut rl) };
        if rc != 0 {
            error!("dune: failed to get heap size");
            return Err(rc);
        }

        rl.rlim_cur += increase_size as u64;
        let rc = unsafe { setrlimit(RLIMIT_DATA, &rl) };
        if rc != 0 {
            error!("dune: failed to set heap size");
            return Err(rc);
        }

        Ok(())
    }

    pub fn mm_init(fd: i32) -> Result<(), i32> {
        info!("mm init");

        page_init(fd)?;
        mm_init(fd)?;
        pgtable_init(fd)?;

        Ok(())
    }
}
