/// x86_64-specific system module
pub mod x86_64;
/// APIC (Advanced Programmable Interrupt Controller) module
pub mod apic;
/// IDT (Interrupt Descriptor Table) module
pub mod idt;
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

pub use crate::sys::x86_64::*;
#[cfg(feature = "apic")]
pub use crate::sys::apic::*;
pub use crate::sys::percpu::*;
#[cfg(feature = "seimi")]
pub use crate::sys::seimi::seimi_init;
pub use crate::sys::signal::signal_init;
pub use crate::sys::serial::serial_init;
pub use crate::sys::syscall::setup_syscall;
pub use crate::sys::syscall::setup_vsyscall;