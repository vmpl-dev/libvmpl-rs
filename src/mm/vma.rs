use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ffi::CString;

pub const PERM_NONE: u32 = 0; // no access
pub const PERM_R: u32 = 0x0001; // read permission
pub const PERM_W: u32 = 0x0002; // write permission
pub const PERM_X: u32 = 0x0004; // execute permission
pub const PERM_U: u32 = 0x0008; // user-level permission
pub const PERM_UC: u32 = 0x0010; // make uncachable
pub const PERM_COW: u32 = 0x0020; // COW flag
pub const PERM_USR1: u32 = 0x1000; // User flag 1
pub const PERM_USR2: u32 = 0x2000; // User flag 2
pub const PERM_USR3: u32 = 0x3000; // User flag 3
pub const PERM_BIG: u32 = 0x0100; // Use large pages
pub const PERM_BIG_1GB: u32 = 0x0200; // Use large pages (1GB)

// Helper Macros
pub const PERM_SCODE: u32 = PERM_R | PERM_X;
pub const PERM_STEXT: u32 = PERM_R | PERM_W;
pub const PERM_SSTACK: u32 = PERM_STEXT;
pub const PERM_UCODE: u32 = PERM_R | PERM_U | PERM_X;
pub const PERM_UTEXT: u32 = PERM_R | PERM_U | PERM_W;
pub const PERM_USTACK: u32 = PERM_UTEXT;

pub const VMPL_VMA_TYPE_FILE: i32 = 1;
pub const VMPL_VMA_TYPE_ANONYMOUS: i32 = 2;
pub const VMPL_VMA_TYPE_HEAP: i32 = 3;
pub const VMPL_VMA_TYPE_STACK: i32 = 4;
pub const VMPL_VMA_TYPE_VSYSCALL: i32 = 5;
pub const VMPL_VMA_TYPE_VDSO: i32 = 6;
pub const VMPL_VMA_TYPE_VVAR: i32 = 7;
pub const VMPL_VMA_TYPE_UNKNOWN: i32 = 8;

enum VmplVmaType {
    File,
    Anonymous,
    Heap,
    Stack,
    Vsyscall,
    Vdso,
    Vvar,
    Unknown,
}

pub struct ProcmapEntry {
    begin: u64,
    end: u64,
    offset: u32,
    r: bool, // Readable
    w: bool, // Writable
    x: bool, // Executable
    p: bool, // Private (or shared)
    minor: u32,  // New field for device
    major: u32,  // New field for device
    inode: u32,  // New field for inode
    path: Option<String>,
    entry_type: VmplVmaType,
}

impl ProcmapEntry {
    pub fn new(
        begin: u64,
        end: u64,
        offset: u32,
        r: char,
        w: char,
        x: char,
        p: char,
        minor: u32,
        major: u32,
        inode: u32,
        path: Option<String>,
        entry_type: VmplVmaType,
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
            entry_type,
        }
    }
}

impl From <(u64, u64, u32, bool, bool, bool, bool, u32, u32, u32, Option<String>, VmplVmaType)> for ProcmapEntry {
    fn from(entry: (u64, u64, u32, bool, bool, bool, bool, u32, u32, u32, String)) -> Self {
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
            entry_type: entry.11,
        }
    }
}

fn get_vmpl_vma_type(path: &str) -> VmplVmaType {
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
        if path == "[stack]" || (path.chars().nth(6).unwrap() == ':' && path.chars().last().unwrap() == ']') {
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

type ProcmapsCallback = fn(&ProcmapEntry, &mut ());

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
            entry_type: VmplVmaType::Unknown,
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

        if let Ok(value) = scan_fmt!(&line, "{x}-{x} {}{}{}{} {x} {:x}:{:x} {} {}",
            u64, u64, char, char, char, char, u32, u32, u32, u32, String) {
            callback(&ProcmapEntry::from(value), arg);
        }
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct VmplVma {
    start: u64,
    end: u64,
    offset: u32,
    prot: u64,
    flags: u64,
    minor: u32,  // New field for device
    major: u32,  // New field for device
    inode: u32,  // New field for inode
    vm_file: Option<String>,
}

use std::fmt;

impl VmplVma {
    pub fn new(start: u64, end: u64, flags: u64, prot: u64, offset: u32) -> Self {
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
            "vmpl-vma: {:x}-{:x} {}{}{} {:08x} {:02x}:{:02x} {:8} {}",
            self.start,
            self.end,
            if self.prot & PROT_READ != 0 { 'r' } else { '-' },
            if self.prot & PROT_WRITE != 0 { 'w' } else { '-' },
            if self.prot & PROT_EXEC != 0 { 'x' } else { '-' },
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