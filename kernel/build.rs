fn main() {
    // Собираем ассемблерный bootstrap с помощью nasm
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest = std::path::Path::new(&out_dir).join("boot.o");

    let status = std::process::Command::new("nasm")
        .args(["-f", "elf64", "src/arch/x86_64/boot.s", "-o"])
        .arg(&dest)
        .status()
        .expect("failed to run nasm");

    if !status.success() {
        panic!("nasm failed");
    }

    println!("cargo:rustc-link-arg={}", dest.display());
    println!("cargo:rerun-if-changed=src/arch/x86_64/boot.s");
    println!("cargo:rerun-if-changed=linker/linker.ld");
}
