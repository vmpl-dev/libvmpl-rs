extern crate libc;

pub mod vmpl_ioctl {

    use libc::{c_int, ioctl};
    use std::fs::File;
    use std::io::{Error, ErrorKind};
    use std::os::unix::prelude::AsRawFd;

    use iocuddle::*;
    use log::debug;

    use crate::sys::core::{DuneConfig, GetPagesParams, SeimiParams, VmplConfig, VmplParam, VmplSegs};

    pub struct VmplFile {
        fd: File,
    }

    // Create a group for the ioctl commands of VMPL
    const VMPL_IOCTL: Group = Group::new(b'k');

    // Define the ioctl commands
    const VMPL_IOCTL_GET_DATA: Ioctl<Write, &VmplParam> = unsafe { VMPL_IOCTL.write(0x11) };
    const VMPL_IOCTL_SET_DATA: Ioctl<Write, &VmplParam> = unsafe { VMPL_IOCTL.write(0x12) };
    const VMPL_IOCTL_VMPL_INIT: Ioctl<Write, &VmplConfig> = unsafe { VMPL_IOCTL.write(0x13) };
    const VMPL_IOCTL_VMPL_RUN: Ioctl<WriteRead, &DuneConfig> =
        unsafe { VMPL_IOCTL.write_read(0x14) };
    const VMPL_IOCTL_GET_GHCB: Ioctl<Read, &u64> = unsafe { VMPL_IOCTL.read(0x15) };
    const VMPL_IOCTL_GET_CR3: Ioctl<Read, &u64> = unsafe { VMPL_IOCTL.read(0x16) };
    const VMPL_IOCTL_GET_PAGES: Ioctl<WriteRead, &GetPagesParams> =
        unsafe { VMPL_IOCTL.write_read(0x17) };
    const VMPL_IOCTL_SET_SYSCALL: Ioctl<Write, &u64> = unsafe { VMPL_IOCTL.write(0x19) };
    const VMPL_IOCTL_SET_SEIMI: Ioctl<Write, &SeimiParams> = unsafe { VMPL_IOCTL.write(0x18) };
    const VMPL_IOCTL_SET_SEGS: Ioctl<Write, &VmplSegs> = unsafe { VMPL_IOCTL.write(0x21) };
    const VMPL_IOCTL_GET_SEGS: Ioctl<Read, &VmplSegs> = unsafe { VMPL_IOCTL.read(0x22) };

    impl VmplFile {
        pub fn new(fd: File) -> VmplFile {
            VmplFile { fd }
        }

        pub fn set_pgtable_vmpl(
            &mut self,
            gva: u64,
            page_size: u32,
            attrs: u32,
        ) -> Result<(), Error> {
            let nr_pages = 1;
            let mut data = VmplParam::new(gva, page_size, attrs, nr_pages);
            // 用rust风格消除if语句
            VMPL_IOCTL_GET_DATA.ioctl(&mut self.fd, &mut data)?;
            Ok(())
        }

        pub fn set_user_vmpl(&mut self, gva: u64, page_size: u32, attrs: u32) -> Result<(), Error> {
            let nr_pages = 1;
            let data = VmplParam::new(gva, page_size, attrs, nr_pages);
            VMPL_IOCTL_SET_DATA.ioctl(&mut self.fd, &data)?;
            Ok(())
        }

        pub fn get_ghcb(&mut self) -> Result<u64, Error> {
            let (rc, ghcb) = VMPL_IOCTL_GET_GHCB.ioctl(&self.fd)?;
            debug!("dune: returned {}", rc);
            debug!("dune: GHCB at 0x{:x}", ghcb);

            Ok(ghcb)
        }

        pub fn get_cr3(&mut self) -> Result<u64, Error> {
            let (rc, cr3) = VMPL_IOCTL_GET_CR3.ioctl(&self.fd)?;
            debug!("dune: returned {}", rc);
            debug!("dune: CR3 at 0x{:x}", cr3);

            Ok(cr3)
        }

        pub fn get_pages(&mut self, param: &mut GetPagesParams) -> Result<(), Error> {
            VMPL_IOCTL_GET_PAGES.ioctl(&mut self.fd, param)?;
            debug!("dune: pages at 0x{}", param);
            Ok(())
        }

        pub fn set_syscall(&mut self, syscall: &mut u64) -> Result<(), Error> {
            VMPL_IOCTL_SET_SYSCALL.ioctl(&mut self.fd, syscall)?;
            debug!("dune: syscall at 0x{:x}", syscall);

            Ok(())
        }

        pub fn set_seimi(&mut self, seimi: &mut SeimiParams) -> Result<(), Error> {
            VMPL_IOCTL_SET_SEIMI.ioctl(&mut self.fd, seimi)?;
            debug!("dune: seimi at {}", seimi);

            Ok(())
        }

        pub fn set_segs(&mut self, segs: &VmplSegs) -> Result<(), Error> {
            VMPL_IOCTL_SET_SEGS.ioctl(&mut self.fd, segs)?;
            debug!("dune: segs at 0x{}", segs);

            Ok(())
        }

        pub fn get_segs(&mut self) -> Result<VmplSegs, Error> {
            let (rc, segs) = VMPL_IOCTL_GET_SEGS.ioctl(&mut self.fd)?;
            debug!("dune: returned {}", rc);
            debug!("dune: segs at 0x{}", segs);

            Ok(segs)
        }

        pub fn vmpl_run(&mut self, vmsa_config: &mut DuneConfig) -> Result<u32, Error> {
            let rc = VMPL_IOCTL_VMPL_RUN.ioctl(&mut self.fd, vmsa_config)?;

            Ok(rc)
        }
    }
}

#[cfg(feature = "dune")]
mod dune_ioctl {

    use libc::{c_ulong, ioctl};
    use std::fs::File;
    use std::io::{Error, ErrorKind};
    use std::os::unix::io::AsRawFd;
    use std::os::unix::io::RawFd;

    // Create a group for the ioctl commands of Dune
    const DUNE: Group = Group::new(233);

    const DUNE_ENTER: Ioctl<WriteRead, &DuneConfig> = unsafe { DUNE.write_read(0x01) };
    const DUNE_GET_SYSCALL: Ioctl<Read, &u64> = unsafe { DUNE.read(0x02) };
    const DUNE_GET_LAYOUT: Ioctl<Read, &DuneLayout> = unsafe { DUNE.read(0x03) };
    const DUNE_TRAP_ENABLE: Ioctl<Write, &DuneTrapConfig> = unsafe { DUNE.write(0x04) };
    const DUNE_TRAP_DISABLE: Ioctl<Io, &()> = unsafe { DUNE.io(0x05) };

    pub struct DuneFile {
        fd: File,
    }

    impl DuneFile {
        pub fn new(fd: File) -> DuneFile {
            DuneFile { fd }
        }

        pub fn enter(&mut self, config: &mut DuneConfig) -> Result<(), Error> {
            let rc = DUNE_ENTER.ioctl(&self.fd, config)?;
            Ok(())
        }

        pub fn trap_enable(&mut self, trap_config: &mut DuneTrapConfig) -> Result<(), Error> {
            let rc = DUNE_TRAP_ENABLE.ioctl(self.fd, trap_config)?;
            Ok(())
        }

        pub fn trap_disable(&mut self) -> Result<(), Error> {
            let rc = DUNE_TRAP_DISABLE.ioctl(self.fd, &())?;
            Ok(())
        }

        pub fn get_syscall(&mut self, syscall: &mut u64) -> Result<(), Error> {
            let rc = DUNE_GET_SYSCALL.ioctl(self.fd, syscall)?;
            println!("dune: syscall at 0x{:x}", syscall);
            Ok(())
        }

        pub fn get_layout(&mut self, layout: &mut DuneLayout) -> Result<(), Error> {
            let rc = DUNE_GET_LAYOUT.ioctl(self.fd, layout)?;
            println!("dune: phys_limit at 0x{:x}", layout.phys_limit);
            println!("dune: base_map at 0x{:x}", layout.base_map);
            println!("dune: base_stack at 0x{:x}", layout.base_stack);
            Ok(())
        }
    }
}
