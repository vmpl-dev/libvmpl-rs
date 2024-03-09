
// pgtable.c

use std::fmt::Display;

#[repr(C)]
#[derive(Debug, Default)]
pub struct VmplParam {
    gva: u64,
    page_size: u32,
    attrs: u32,
    nr_pages: u32,
}

impl VmplParam {
    pub fn new(gva: u64, page_size: u32, attrs: u32, nr_pages: u32) -> VmplParam {
        VmplParam {
            gva: gva,
            page_size: page_size,
            attrs: attrs,
            nr_pages: nr_pages,
        }
    }
}

// vmpl-dev.c

#[repr(C)]
#[derive(Debug, Default)]
pub struct VmplLayout {
    phys_limit: u64,
    base_map: u64,
    base_stack: u64,
}

impl VmplLayout {
    pub fn new(phys_limit: u64, base_map: u64, base_stack: u64) -> VmplLayout {
        VmplLayout {
            phys_limit: phys_limit,
            base_map: base_map,
            base_stack: base_stack,
        }
    }
}

// vmpl-core.c

#[repr(C)]
#[derive(Debug, Default)]
pub struct VmsaSeg {
    selector: u16,
    attrib: u16,
    limit: u32,
    base: u64,
}

impl VmsaSeg {
    pub fn new(selector: u16, attrib: u16, limit: u32, base: u64) -> VmsaSeg {
        VmsaSeg {
            selector: selector,
            attrib: attrib,
            limit: limit,
            base: base,
        }
    }

    pub fn fs(base: u64) -> VmsaSeg {
        VmsaSeg {
            selector: 0x33,
            attrib: 0x008b,
            limit: 0xffff,
            base: base,
        }
    }

    pub fn gs(base: u64) -> VmsaSeg {
        VmsaSeg {
            selector: 0x3b,
            attrib: 0x008b,
            limit: 0xffff,
            base: base,
        }
    }

    pub fn tr(selector: u16, base: u64, limit: u32, attrib: u16) -> VmsaSeg {
        VmsaSeg {
            selector: selector,
            attrib: attrib,
            limit: limit,
            base: base,
        }
    }
}

impl Display for VmsaSeg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "selector: 0x{:x}, attrib: 0x{:x}, limit: 0x{:x}, base: 0x{:x}", self.selector, self.attrib, self.limit, self.base)
    }
}

#[repr(C, packed)]
#[derive(Debug, Default)]
pub struct VmplSegs {
    fs: VmsaSeg,
    gs: VmsaSeg,
    gdtr: VmsaSeg,
    idtr: VmsaSeg,
    tr: VmsaSeg,
}

impl VmplSegs {
    pub fn new(fs: VmsaSeg, gs: VmsaSeg, gdtr: VmsaSeg, idtr: VmsaSeg, tr: VmsaSeg) -> VmplSegs {
        VmplSegs {
            fs: fs,
            gs: gs,
            gdtr: gdtr,
            idtr: idtr,
            tr: tr,
        }
    }
}

impl Display for VmplSegs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fs: {}, gs: {}, gdtr: {}, idtr: {}, tr: {}", self.fs, self.gs, self.gdtr, self.idtr, self.tr)
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct DuneConfig {
    ret: i64,
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rsp: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rip: u64,
    rflags: u64,
    cr3: u64,
    status: i64,
    vcpu: u64,
}

impl DuneConfig {

    pub fn new(rip: u64, rsp: u64, rflags: u64) -> DuneConfig {
        DuneConfig {
            ret: 0,
            rax: 0,
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rsp: rsp,
            rbp: 0,
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rip: rip,
            rflags: rflags,
            cr3: 0,
            status: 0,
            vcpu: 0,
        }
    }
}

pub struct VmplConfig {
    pub vcpu: u64,
    pub rip: u64,
    pub rsp: u64,
    pub rflags: u64,
    pub cr3: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct DuneTrapRegisters {
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rsp: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    rip: u64,
    rflags: u64,
}

#[repr(C, packed)]
pub struct GdtrEntry {
    limit_lo: u16,     // 段界限低16位
    base: u32,         // 
    base_hi: u8,
    type_: u8,
    s: u8,
    dpl: u8,
    p: u8,
    limit_hi: u8,
    avl: u8,
    l: u8,
    db: u8,
    g: u8,
    base_highest: u8,
}

#[repr(C, packed)]
pub struct DuneTrapFrame {
    /* manually saved, arguments */
    rdi: u64,
    rsi: u64,
    rdx: u64,
    rcx: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,

    /* saved by C calling conventions */
    rbx: u64,
    rbp: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,

    /* system call number, ret */
    rax: u64,

    /* exception frame */
    err: u32,
    pad1: u32,
    rip: u64,
    cs: u16,
    pad2: [u16; 3],
    rflags: u64,
    rsp: u64,
    ss: u16,
    pad3: [u16; 3],
}

#[repr(C, packed)]
#[derive(Debug, Default)]
pub struct GetPagesParams {
    num_pages: usize,
    phys: u64,
}

impl GetPagesParams {
    pub fn new(num_pages: usize, phys: u64) -> GetPagesParams {
        GetPagesParams {
            num_pages: num_pages,
            phys: phys,
        }
    }
}

impl Display for GetPagesParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "num_pages: {}, phys: 0x{:x}", self.num_pages, self.phys)
    }
}

#[repr(C, packed)]
pub struct SeimiParams {
    pgd_user: u64,
    pgd_super: u64,
}

impl SeimiParams {
    pub fn new(pgd_user: u64, pgd_super: u64) -> SeimiParams {
        SeimiParams {
            pgd_user: pgd_user,
            pgd_super: pgd_super,
        }
    }
}

impl Display for SeimiParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pgd_user: 0x{:x}, pgd_super: 0x{:x}", self.pgd_user, self.pgd_super)
    }
}