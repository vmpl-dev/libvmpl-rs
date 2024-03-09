pub enum VmplRet {
    None = 0,
    Exit = 1,
    Syscall = 2,
    Interrupt = 3,
    Signal = 4,
    NoEnter = 6,
}

pub enum DuneRet {
    None = 0,
    Exit = 1,
    Syscall = 2,
    Interrupt = 3,
    Signal = 4,
    NoEnter = 6,
}