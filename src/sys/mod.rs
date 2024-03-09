/// APIC (Advanced Programmable Interrupt Controller) module
pub mod apic;
/// VMPL Core module
pub mod core;
/// IOCTL module
pub mod ioctl;
/// Per-CPU module
pub mod percpu;
/// SEIMI (Secure Execution Instruction Memory Isolation) module
pub mod seimi;
/// Serial module
pub mod serial;
/// Signal module
pub mod signal;
/// Syscall module
pub mod syscall;

pub use crate::sys::apic::*;
pub use crate::sys::percpu::*;
pub use crate::sys::seimi::setup_seimi;
pub use crate::sys::signal::setup_signal;
pub use crate::sys::serial::serial_init;
pub use crate::sys::syscall::setup_syscall;
pub use crate::sys::syscall::setup_vsyscall;