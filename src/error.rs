use std::fmt;

// dune: unable to setup syscall handler
// dune: unable to setup vsyscall handler
// dune: failed to setup APIC
// dune: unable to setup memory management
// dune: failed to setup safe stack
pub enum VmplError {
    Io(std::io::Error),
    Sys(i32),
    ApicSetupFailed(i32),
    #[cfg(feature = "seimi")]
    SeimiSetupFailed(i32),
    SyscallSetupFailed(i32),
    VsyscallSetupFailed(i32),
    MemorySetupFailed(i32),
    SafeStackSetupFailed(i32),
}

impl From<std::io::Error> for VmplError {
    fn from(e: std::io::Error) -> VmplError {
        VmplError::Io(e)
    }
}

impl From<i32> for VmplError {
    fn from(e: i32) -> VmplError {
        VmplError::Sys(e)
    }
}



impl fmt::Display for VmplError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "VMPL Error: ");
        match self {
            VmplError::Io(e) => write!(f, "{}", e),
            VmplError::Sys(e) => write!(f, "{}", e),
            VmplError::ApicSetupFailed(e) => write!(f, "failed to setup APIC"),
            #[cfg(feature = "seimi")]
            VmplError::SeimiSetupFailed(e) => write!(f, "failed to setup SEIMI"),
        }
    }
}