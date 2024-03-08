use core::arch::global_asm;

global_asm!(include_str!("dune.S"));
global_asm!(include_str!("vsyscall.S"));

extern "C" {
    // your extern functions here
    type DuneIntrCb = fn(tf: &mut DuneTrapFrame);
    type DunePgfltCb = fn(addr: usize, fec: u64, tf: &mut DuneTrapFrame);
    type DuneSyscallCb = fn(tf: &mut DuneTrapFrame);
    type SighandlerT = fn(signum: i32);

    // dune fd for the current process
    pub static DUNE_FD: i32;

    // assembly routines from dune.S
    pub fn __dune_enter(fd: i32, config: *mut DuneConfig) -> i32;
    pub fn __dune_ret() -> i32;
    pub fn __dune_syscall();
    pub fn __dune_syscall_end();
    pub fn __dune_intr();
    pub fn __dune_go_linux(config: *mut DuneConfig);
    pub fn __dune_go_dune(fd: i32, config: *mut DuneConfig);

    // assembly routine for handling vsyscalls
    pub static __dune_vsyscall_page: c_char;

    // dune routines for registering handlers
    pub fn dune_register_intr_handler(vec: i32, cb: DuneIntrCb) -> i32;
    pub fn dune_register_signal_handler(signum: i32, cb: DuneIntrCb) -> i32;
    pub fn dune_register_pgflt_handler(cb: DunePgfltCb);
    pub fn dune_register_syscall_handler(cb: DuneSyscallCb);

    pub fn dune_pop_trap_frame(tf: *mut DuneTrapFrame);
    pub fn dune_jump_to_user(tf: *mut DuneTrapFrame) -> i32;
    pub fn dune_ret_from_user(ret: i32) -> !;
    pub fn dune_dump_trap_frame(tf: *mut DuneTrapFrame);
    pub fn dune_passthrough_syscall(tf: *mut DuneTrapFrame);

    // fault handling
    pub fn dune_signal(sig: i32, cb: SighandlerT) -> SighandlerT;
    pub fn dune_get_user_fs() -> u64;
    pub fn dune_set_user_fs(fs_base: u64);
}

#[no_mangle]
pub extern "C" fn dune_init(_map_full: bool) -> i32 {
    // Your implementation here
    -1
}

#[no_mangle]
pub extern "C" fn dune_enter() -> i32 {
    // Your implementation here
    -1
}

#[no_mangle]
pub extern "C" fn dune_init_and_enter() -> i32 {
    let mut ret;

    unsafe {
        ret = dune_init(true);
        if ret != 0 {
            return ret;
        }

        ret = dune_enter();
    }

    ret
}