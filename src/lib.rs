pub mod globals;
pub mod error;
pub mod util;
pub mod sys;
pub mod mm;
pub mod ghcb;
pub mod start;
pub mod dune;
pub mod vmpl;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
