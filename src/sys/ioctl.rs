extern crate libc;

pub mod vmpl_ioctl {
    
    use core::error::Error;
    
    use std::ptr;
    use std::io::{Error, ErrorKind};
    use std::mem::size_of;
    use std::fs::File;
    use std::os::unix::prelude::AsRawFd;
    use std::os::unix::io::RawFd;
    
    use libc::{c_int, c_ulong, ioctl};
    use libc::ioctl;
    
    use iocuddle::*;
    use iocuddle::IoctlCmd::*;
    use iocuddle::IoctlFlags::*;

    use crate::sys::core::VmsaConfig;

    // Create a group for the ioctl commands of VMPL
    const VMPL_IOCTL: Group = Group::new(b'k');

    // Define the ioctl commands
    const VMPL_IOCTL_GET_DATA: Ioctl<Write, &VmplParam> = unsafe { VMPL_IOCTL.write(0x11) };
    const VMPL_IOCTL_SET_DATA: Ioctl<Write, &VmplParam> = unsafe { VMPL_IOCTL.write(0x12) };
    const VMPL_IOCTL_VMPL_INIT: Ioctl<Write, &VmplConfig> = unsafe { VMPL_IOCTL.write(0x13) };
    const VMPL_IOCTL_VMPL_RUN: Ioctl<WriteRead, &VmsaConfig> =
        unsafe { VMPL_IOCTL.write_read(0x14) };
    const VMPL_IOCTL_GET_GHCB: Ioctl<Read, &u64> = unsafe { VMPL_IOCTL.read(0x15) };
    const VMPL_IOCTL_GET_CR3: Ioctl<Read, &u64> = unsafe { VMPL_IOCTL.read(0x16) };
    const VMPL_IOCTL_GET_PAGES: Ioctl<WriteRead, &GetPagesParams> =
        unsafe { VMPL_IOCTL.write_read(0x17) };
    const VMPL_IOCTL_SET_SYSCALL: Ioctl<Write, &u64> = unsafe { VMPL_IOCTL.write(0x19) };
    const VMPL_IOCTL_SET_SEIMI: Ioctl<Write, &u64> = unsafe { VMPL_IOCTL.write(0x18) };
    const VMPL_IOCTL_SET_SEGS: Ioctl<Write, &VmplSegs> = unsafe { VMPL_IOCTL.write(0x21) };
    const VMPL_IOCTL_GET_SEGS: Ioctl<Read, &VmplSegs> = unsafe { VMPL_IOCTL.read(0x22) };

    pub fn vmpl_ioctl_set_pgtable_vmpl(
        vmpl_fd: &File,
        gva: u64,
        page_size: u64,
        attrs: u32,
    ) -> Result<(), Error> {
        let mut data = VmplParam {
            gva,
            page_size,
            attrs,
        };

        let rc = unsafe { ioctl(vmpl_fd.as_raw_fd(), VMPL_IOCTL_GET_DATA, &mut data) };
        if rc < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn vmpl_ioctl_set_user_vmpl(
        vmpl_fd: &File,
        gva: u64,
        page_size: u64,
        attrs: u32,
    ) -> Result<(), Error> {
        let mut data = VmplParam {
            gva,
            page_size,
            attrs,
        };

        let rc = unsafe { ioctl(vmpl_fd.as_raw_fd(), VMPL_IOCTL_SET_DATA, &mut data) };
        if rc < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn vmpl_ioctl_get_ghcb(vmpl_fd: c_int) -> Result<u64, Error> {
        let mut ghcb: u64 = 0;

        let rc = unsafe { ioctl(vmpl_fd, VMPL_IOCTL_GET_GHCB as _, &mut ghcb as *mut _) };

        if rc != 0 {
            return Err(Error::last_os_error());
        }

        println!("dune: GHCB at 0x{:x}", ghcb);

        Ok(ghcb)
    }

    pub fn vmpl_ioctl_get_cr3(vmpl_fd: c_int) -> Result<u64, Error> {
        let mut cr3: u64 = 0;

        let rc = unsafe { ioctl(vmpl_fd, VMPL_IOCTL_GET_CR3 as _, &mut cr3 as *mut _) };

        if rc != 0 {
            return Err(Error::last_os_error());
        }

        println!("dune: CR3 at 0x{:x}", cr3);

        Ok(cr3)
    }

    pub fn vmpl_ioctl_get_pages(vmpl_fd: c_int, param: &mut GetPagesParams) -> Result<(), Error> {
        let rc = unsafe { ioctl(vmpl_fd, VMPL_IOCTL_GET_PAGES as _, param) };

        if rc != 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn vmpl_ioctl_set_syscall(vmpl_fd: c_int, syscall: &mut u64) -> Result<(), Error> {
        let rc = unsafe { ioctl(vmpl_fd, VMPL_IOCTL_SET_SYSCALL as _, syscall) };

        if rc < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn vmpl_ioctl_set_seimi(vmpl_fd: c_int, seimi: &mut SeimiParams) -> Result<(), Error> {
        let rc = unsafe { ioctl(vmpl_fd, VMPL_IOCTL_SET_SEIMI as _, seimi) };

        if rc < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn vmpl_ioctl_set_segs(vmpl_fd: c_int, segs: &mut VmplSegs) -> Result<(), Error> {
        let rc = unsafe { ioctl(vmpl_fd, VMPL_IOCTL_SET_SEGS as _, segs) };

        if rc < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn vmpl_ioctl_get_segs(vmpl_fd: c_int, segs: &mut VmplSegs) -> Result<(), Error> {
        let rc = unsafe { ioctl(vmpl_fd, VMPL_IOCTL_GET_SEGS as _, segs) };

        if rc < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }

    pub fn vmpl_ioctl_vmpl_run(vmpl_fd: &File, vmsa_config: &mut VmsaConfig) -> Result<i32, Error> {
        let rc = unsafe { ioctl(vmpl_fd.as_raw_fd(), VMPL_IOCTL_VMPL_RUN, vmsa_config) };
        if rc < 0 {
            return Err(Error::last_os_error());
        }

        Ok(rc)
    }
}

#[cfg(feature = "dune")]
mod dune_ioctl {

    use std::io::{Error, ErrorKind};
    use std::fs::File;
    use std::os::unix::io::AsRawFd;
    use std::os::unix::io::RawFd;
    use libc::{ioctl, c_ulong};

    // Create a group for the ioctl commands of Dune
    const DUNE: Group = Group::new(233);

    const DUNE_ENTER: Ioctl<WriteRead, &DuneConfig> = unsafe { DUNE.write_read(0x01) };
    const DUNE_GET_SYSCALL: Ioctl<Read, &u64> = unsafe { DUNE.read(0x02) };
    const DUNE_GET_LAYOUT: Ioctl<Read, &DuneLayout> = unsafe { DUNE.read(0x03) };
    const DUNE_TRAP_ENABLE: Ioctl<Write, &DuneTrapConfig> = unsafe { DUNE.write(0x04) };
    const DUNE_TRAP_DISABLE: Ioctl<Io, &()> = unsafe { DUNE.io(0x05) };

    pub fn trap_enable(dune_fd: &File, trap_config: &mut DuneTrapConfig) -> Result<(), Error> {
        let rc = unsafe { ioctl(dune_fd.as_raw_fd(), DUNE_TRAP_ENABLE, trap_config) };
        if rc < 0 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    pub fn trap_disable(dune_fd: &File) -> Result<(), Error> {
        let rc = unsafe { ioctl(dune_fd.as_raw_fd(), DUNE_TRAP_DISABLE) };
        if rc < 0 {
            return Err(Error::last_os_error());
        }
        Ok(())
    }

    pub fn get_syscall(dune_fd: &File, syscall: &mut u64) -> Result<(), Error> {
        let rc = unsafe { ioctl(dune_fd.as_raw_fd(), DUNE_GET_SYSCALL, syscall) };
        if rc != 0 {
            return Err(Error::last_os_error());
        }
        println!("dune: syscall at 0x{:x}", syscall);
        Ok(())
    }

    pub fn get_layout(dune_fd: &File, layout: &mut DuneLayout) -> Result<(), Error> {
        let rc = unsafe { ioctl(dune_fd.as_raw_fd(), DUNE_GET_LAYOUT, layout) };
        if rc != 0 {
            return Err(Error::last_os_error());
        }
        println!("dune: phys_limit at 0x{:x}", layout.phys_limit);
        println!("dune: base_map at 0x{:x}", layout.base_map);
        println!("dune: base_stack at 0x{:x}", layout.base_stack);
        Ok(())
    }
}
