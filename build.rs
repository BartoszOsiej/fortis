#![allow(clippy::expect_used)] // build scripts must panic on failure

fn main() {
    let ld_script = std::fs::canonicalize("link.ld").expect("link.ld not found");
    println!("cargo:rustc-link-arg=-T{}", ld_script.display());
    println!("cargo:rerun-if-changed=link.ld");
}
