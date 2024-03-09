#[cfg(feature = "mm")]
pub mod vma {
    use crate::BIT;
    use libc::{PROT_EXEC, PROT_NONE, PROT_READ, PROT_WRITE};
    use std::{
        default,
        fmt::{self, Display},
    };

    pub const PERM_NONE: u32 = 0; // no access
    pub const PERM_R: u32 = BIT!(0); // read permission
    pub const PERM_W: u32 = BIT!(1); // write permission
    pub const PERM_X: u32 = BIT!(2); // execute permission
    pub const PERM_U: u32 = BIT!(3); // user-level permission
    pub const PERM_UC: u32 = BIT!(4); // make uncachable
    pub const PERM_COW: u32 = BIT!(5); // COW flag
    pub const PERM_USR1: u32 = BIT!(12); // User flag 1
    pub const PERM_USR2: u32 = BIT!(13); // User flag 2
    pub const PERM_USR3: u32 = BIT!(14); // User flag 3
    pub const PERM_BIG: u32 = BIT!(8); // Use large pages
    pub const PERM_BIG_1GB: u32 = BIT!(9); // Use large pages (1GB)

    // Helper Macros
    pub const PERM_SCODE: u32 = PERM_R | PERM_X;
    pub const PERM_STEXT: u32 = PERM_R | PERM_W;
    pub const PERM_SSTACK: u32 = PERM_STEXT;
    pub const PERM_UCODE: u32 = PERM_R | PERM_U | PERM_X;
    pub const PERM_UTEXT: u32 = PERM_R | PERM_U | PERM_W;
    pub const PERM_USTACK: u32 = PERM_UTEXT;

    pub enum VmplVmaType {
        File,
        Anonymous,
        Heap,
        Stack,
        Vsyscall,
        Vdso,
        Vvar,
        Unknown,
    }

    impl From<&str> for VmplVmaType {
        fn from(path: &str) -> Self {
            if !path.is_empty() && path.chars().next().unwrap() != '[' {
                return VmplVmaType::File;
            }
            if path.is_empty() {
                return VmplVmaType::Anonymous;
            }
            if path == "[heap]" {
                return VmplVmaType::Heap;
            }
            if path.starts_with("[stack") {
                if path == "[stack]"
                    || (path.chars().nth(6).unwrap() == ':' && path.chars().last().unwrap() == ']')
                {
                    return VmplVmaType::Stack;
                }
            }
            if path == "[vsyscall]" {
                return VmplVmaType::Vsyscall;
            }
            if path == "[vdso]" {
                return VmplVmaType::Vdso;
            }
            if path == "[vvar]" {
                return VmplVmaType::Vvar;
            }
            VmplVmaType::Unknown
        }
    }

    pub struct ProcmapEntry {
        begin: u64,
        end: u64,
        offset: u32,
        r: bool,    // Readable
        w: bool,    // Writable
        x: bool,    // Executable
        p: bool,    // Private (or shared)
        minor: u32, // New field for device
        major: u32, // New field for device
        inode: u32, // New field for inode
        path: Option<String>,
    }

    impl ProcmapEntry {
        pub fn new(
            begin: u64,
            end: u64,
            offset: u32,
            r: bool,
            w: bool,
            x: bool,
            p: bool,
            minor: u32,
            major: u32,
            inode: u32,
            path: Option<String>,
        ) -> Self {
            Self {
                begin,
                end,
                offset,
                r,
                w,
                x,
                p,
                minor,
                major,
                inode,
                path,
            }
        }
    }

    impl
        From<(
            u64,
            u64,
            u32,
            bool,
            bool,
            bool,
            bool,
            u32,
            u32,
            u32,
            Option<String>,
        )> for ProcmapEntry
    {
        fn from(
            entry: (
                u64,
                u64,
                u32,
                bool,
                bool,
                bool,
                bool,
                u32,
                u32,
                u32,
                Option<String>,
            ),
        ) -> Self {
            Self {
                begin: entry.0,
                end: entry.1,
                offset: entry.2,
                r: entry.3,
                w: entry.4,
                x: entry.5,
                p: entry.6,
                minor: entry.7,
                major: entry.8,
                inode: entry.9,
                path: entry.10,
            }
        }
    }

    type ProcmapsCallback = fn(&ProcmapEntry, &mut ());

    #[cfg(feature = "vm")]
    fn parse_procmaps(callback: ProcmapsCallback, arg: &mut ()) -> Result<(), std::io::Error> {
        let file = File::open("/proc/self/maps")?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let mut path = [0; 256];
            let mut entry = ProcmapEntry {
                begin: 0,
                end: 0,
                offset: 0,
                r: false,
                w: false,
                x: false,
                p: false,
                minor: 0,
                major: 0,
                inode: 0,
                path: None,
            };
            use scan_fmt::scan_fmt;

            let mut begin: u64;
            let mut begin: u64;
            let mut end: u64;
            let mut read: char;
            let mut write: char;
            let mut execute: char;
            let mut private: char;
            let mut offset: u32;
            let mut minor: u32;

            let value = scan_fmt!(
                &line,
                "{x}-{x} {}{}{}{} {x} {:x}:{:x} {} {}",
                begin,
                end,
                read,
                write,
                execute,
                private,
                offset,
                minor,
                major,
                inode,
                path
            );
            if let Some(value) = value {
                callback(&ProcmapEntry::from(value), arg);
            }
        }

        Ok(())
    }

    #[derive(Debug, Default)]
    enum Prot {
        #[default]
        None,
        Read,
        Write,
        Exec,
    }

    impl From<i32> for Prot {
        fn from(prot: i32) -> Self {
            match prot {
                PROT_NONE => Prot::None,
                PROT_READ => Prot::Read,
                PROT_WRITE => Prot::Write,
                PROT_EXEC => Prot::Exec,
                _ => Prot::None,
            }
        }
    }

    impl Display for Prot {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Prot::None => write!(f, "----"),
                Prot::Read => write!(f, "r---"),
                Prot::Write => write!(f, "-w--"),
                Prot::Exec => write!(f, "--x-"),
            }
        }
    }

    #[derive(Debug, Default)]
    pub struct VmplVma {
        start: u64,
        end: u64,
        offset: u32,
        prot: Prot,
        flags: u64,
        minor: u32,
        major: u32,
        inode: u32,
        vm_file: Option<String>,
    }

    impl VmplVma {
        pub fn new(start: u64, end: u64, flags: u64, prot: Prot, offset: u32) -> Self {
            Self {
                start,
                end,
                flags,
                prot,
                offset,
                minor: 0,
                major: 0,
                inode: 0,
                vm_file: None,
            }
        }

        pub fn len(&self) -> u64 {
            self.end - self.start
        }

        pub fn print(&self) {
            println!("{}", self);
        }

        pub fn dump(&self) {
            println!("{}", self);
        }
    }

    impl fmt::Display for VmplVma {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "vmpl-vma: {:x}-{:x} {} {:08x} {:02x}:{:02x} {:8} {}",
                self.start,
                self.end,
                self.prot,
                self.offset,
                self.minor,
                self.major,
                self.inode,
                self.vm_file.as_deref().unwrap_or(""),
            )
        }
    }

    pub enum FitAlgorithm {
        FirstFit,
        NextFit,
        BestFit,
        WorstFit,
        RandomFit,
    }
}