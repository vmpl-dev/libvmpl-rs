#[cfg(feature = "dune")]
pub mod signal {
    use libc::sigaction;
    use log::info;
    use nix::errno::Errno;
    use nix::sys::signal::*;

    use crate::error::VmplError;

    pub fn signal_init() -> Result<(), VmplError> {
        info!("setup signal");

        // disable signals for now until we have better support
        info!("disable signals for now until we have better support");
        for i in 1..32 {
            let signum = match Signal::from_c_int(i) {
                Ok(s) => s,
                Err(Errno::EINVAL) => continue,
                Err(e) => panic!("unexpected error: {}", e),
            };

            match signum {
                SIGTSTP | SIGSTOP | SIGKILL | SIGCHLD | SIGINT | SIGTERM => continue,
                _ => (),
            }

            let act = SigAction::new(SigHandler::SigIgn, SaFlags::empty(), SigSet::empty());
            let oldact = SigAction::empty();

            unsafe {
                sigaction(signum, &act, &oldact);
            }
        }

        Ok(())
    }
}