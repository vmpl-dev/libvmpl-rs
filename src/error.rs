pub enum VmplError {
    Io(std::io::Error),
    Sys(i32),
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