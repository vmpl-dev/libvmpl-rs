
use nix::sys::signal::{Signal, SigAction, SaFlags, SigSet, SigHandler, sigaction};
use nix::Error::Sys;
use nix::errno::Errno;
use nix::sys::signal::Signal::{SIGTSTP, SIGSTOP, SIGKILL, SIGCHLD, SIGINT, SIGTERM};
use nix::sys::signal::Signal::SIGWINCH;
use log::info;

pub fn setup_signal() {
    info!("setup signal");

    // disable signals for now until we have better support
    info!("disable signals for now until we have better support");
    for i in 1..32 {
        let signal = match Signal::from_c_int(i) {
            Ok(s) => s,
            Err(Errno::EINVAL) => continue,
            Err(e) => panic!("unexpected error: {}", e),
        };

        match signal {
            SIGTSTP | SIGSTOP | SIGKILL | SIGCHLD | SIGINT | SIGTERM => continue,
            _ => (),
        }

        let sa = SigAction::new(
            SigHandler::SigIgn,
            SaFlags::empty(),
            SigSet::empty(),
        );

        unsafe {
            if let Err(e) = sigaction(signal, &sa) {
                panic!("sigaction() {}: {}", i, e);
            }
        }
    }
}