extern crate cc;

fn main() {
    println!("Hello, World!");
    // cc::Build::new()
    //     .flag("-masm=intel") // 添加适当的汇编标志
    //     .flag("-m64")
    //     .file("src/start/syscall_vmpl.h")
    //     .file("src/start/dune.S")
    //     .file("src/start/vsyscall.S")
    //     .compile("asm");
}