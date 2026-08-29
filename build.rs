fn main() {
    let ld_script = std::fs::canonicalize("link.ld").unwrap();
    println!("cargo:rustc-link-arg=-T{}", ld_script.display());
    println!("cargo:rerun-if-changed=link.ld");
}
