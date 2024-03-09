use std::arch::asm;

pub const EFER_SVME: u64 = 1 << 12;

pub fn cmpxchg(cmpval: u64, newval: u64, va: u64) -> u64 {
    let ret: u64;

    unsafe {
        asm!("lock cmpxchg [{0}], {1}",
             in(reg) va, in(reg) newval, in("rax") cmpval,
             lateout("rax") ret,
             options(nostack));
    }

    ret
}