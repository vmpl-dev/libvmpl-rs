/// GHCB (Guest-Hypervisor Communication Block) module
pub mod ghcb;
/// #VC (Virtualization Exception) module
pub mod vc;

pub use ghcb::Ghcb;
pub use vc::vc_init;