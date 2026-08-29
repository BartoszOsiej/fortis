//! Fortis host-side verifier
//!
//! Reads the firmware ELF, independently computes expected PCR values,
//! runs the firmware in QEMU, and compares the results.
//!
//! Usage: cargo run --release -- path/to/fortis.elf

use ml_kem::kem::Decapsulate;
use ml_kem::{array::Array, DecapsulationKey, MlKem768};
use object::{Object, ObjectSection};
use sha2::{Digest, Sha256};
use std::fs;
use std::process::Command;

fn main() {
    let elf_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "target/riscv64gc-unknown-none-elf/release/fortis".to_string());

    // Compute project root from ELF path (ELF is at target/<triple>/release/fortis)
    let elf_path_std = std::path::Path::new(&elf_path);
    let project_root = elf_path_std
        .parent()
        .unwrap() // release/
        .parent()
        .unwrap() // <triple>/
        .parent()
        .unwrap() // target/
        .parent()
        .unwrap(); // project root
    let keys_path = project_root.join("src/keys.rs");

    println!("=== Fortis Verifier ===");
    println!("ELF: {}", elf_path);
    println!();

    // ── Step 1: Read ELF and extract sections ───────────────────────
    let elf_bytes = fs::read(&elf_path).expect("Failed to read ELF file");
    let elf = object::File::parse(&*elf_bytes).expect("Failed to parse ELF");

    // Extract .text section — by NAME, not by file order!
    let text_section = elf
        .section_by_name(".text")
        .expect("No .text section found in ELF");
    let text_bytes = text_section.data().expect("Failed to read .text data");

    // Extract .rodata section — by NAME, not by file order!
    let rodata_section = elf
        .section_by_name(".rodata")
        .expect("No .rodata section found in ELF");
    let rodata_bytes = rodata_section.data().expect("Failed to read .rodata data");

    println!("ELF sections:");
    println!(
        "  .text:   addr=0x{:08x}  size={}",
        text_section.address(),
        text_bytes.len()
    );
    println!(
        "  .rodata: addr=0x{:08x}  size={}",
        rodata_section.address(),
        rodata_bytes.len()
    );
    println!();

    // ── Step 2: Compute expected PCR values ─────────────────────────
    //
    // EXACT same algorithm as firmware:
    //   pcr0 = SHA-256(zeros_32 || .text)
    //   pcr1 = SHA-256(pcr0 || .rodata)
    //   pcr2 = SHA-256(pcr1 || ss)

    // PCR[0] = SHA-256(zeros || .text)
    let mut pcr0 = [0u8; 32];
    {
        let mut hasher = Sha256::new();
        hasher.update([0u8; 32]);
        hasher.update(text_bytes);
        pcr0.copy_from_slice(&hasher.finalize());
    }

    // PCR[1] = SHA-256(pcr0 || .rodata) — chained
    let mut pcr1 = pcr0;
    {
        let mut hasher = Sha256::new();
        hasher.update(pcr1);
        hasher.update(rodata_bytes);
        pcr1.copy_from_slice(&hasher.finalize());
    }

    // ML-KEM-768: compute shared secret from embedded keys
    let keys_src = fs::read_to_string(&keys_path).unwrap_or_else(|e| {
        panic!("Failed to read {}: {}", keys_path.display(), e);
    });
    let dk_bytes = parse_const_array(&keys_src, "DK", 64);
    let ct_bytes = parse_const_array(&keys_src, "CT", 1088);
    let ss_expected = parse_const_array(&keys_src, "SS", 32);

    let seed = Array::try_from(dk_bytes.as_slice()).unwrap();
    let dk = DecapsulationKey::<MlKem768>::from_seed(seed);
    let ct = Array::try_from(ct_bytes.as_slice()).unwrap();
    let ss_computed = dk.decapsulate(&ct);

    assert_eq!(
        ss_computed.as_slice(),
        ss_expected.as_slice(),
        "ML-KEM-768 roundtrip failed: computed SS != embedded SS"
    );

    // PCR[2] = SHA-256(zeros || ss) — starts from zero, same as firmware
    let mut pcr2 = [0u8; 32];
    {
        let mut hasher = Sha256::new();
        hasher.update([0u8; 32]);
        hasher.update(ss_computed.as_slice());
        pcr2.copy_from_slice(&hasher.finalize());
    }

    println!("Expected PCR values (host-computed):");
    println!("  PCR[0] = 0x{}", hex_str(&pcr0));
    println!("  PCR[1] = 0x{}", hex_str(&pcr1));
    println!("  PCR[2] = 0x{}", hex_str(&pcr2));
    println!();

    // ── Step 3: Run firmware in QEMU ────────────────────────────────
    println!("Running firmware in QEMU...");
    let output = Command::new("timeout")
        .args(["10"])
        .arg("qemu-system-riscv64")
        .args(["-machine", "virt"])
        .args(["-bios", "default"])
        .args(["-nographic"])
        .args(["-kernel", &elf_path])
        .output()
        .expect("Failed to run QEMU — is qemu-system-riscv64 installed?");

    let qemu_output = String::from_utf8_lossy(&output.stdout);

    // Parse PCR values from firmware output
    let fw_pcr0 = parse_pcr(&qemu_output, 0).expect("Failed to parse PCR[0] from QEMU output");
    let fw_pcr1 = parse_pcr(&qemu_output, 1).expect("Failed to parse PCR[1] from QEMU output");
    let fw_pcr2 = parse_pcr(&qemu_output, 2).expect("Failed to parse PCR[2] from QEMU output");

    println!("Firmware PCR values (from QEMU):");
    println!("  PCR[0] = 0x{}", hex_str(&fw_pcr0));
    println!("  PCR[1] = 0x{}", hex_str(&fw_pcr1));
    println!("  PCR[2] = 0x{}", hex_str(&fw_pcr2));
    println!();

    // ── Step 4: Compare ─────────────────────────────────────────────
    let mut all_pass = true;

    println!("Verification:");
    if pcr0 == fw_pcr0 {
        println!("  PCR[0] ✓ match");
    } else {
        println!("  PCR[0] ✗ MISMATCH");
        all_pass = false;
    }

    if pcr1 == fw_pcr1 {
        println!("  PCR[1] ✓ match");
    } else {
        println!("  PCR[1] ✗ MISMATCH");
        all_pass = false;
    }

    if pcr2 == fw_pcr2 {
        println!("  PCR[2] ✓ match");
    } else {
        println!("  PCR[2] ✗ MISMATCH");
        all_pass = false;
    }

    println!();
    if all_pass {
        println!("=== VERIFICATION: PASSED ===");
    } else {
        println!("=== VERIFICATION: FAILED ===");
        std::process::exit(1);
    }
}

/// Parse a `const NAME: [u8; N] = [0xNN, ...];` from Rust source.
fn parse_const_array(src: &str, name: &str, expected_len: usize) -> Vec<u8> {
    // Find the const declaration
    let pattern = format!("pub const {}: [u8;", name);
    let start = src.find(&pattern).unwrap_or_else(|| {
        panic!("Could not find '{}' in source", name);
    });

    // Find the opening bracket of the array value (after the `= `)
    let eq_pos = src[start..].find("= [").unwrap() + start;
    let bracket_start = eq_pos + 2; // skip '= '
                                    // Find the closing bracket
    let bracket_end = src[bracket_start..].find(']').unwrap() + bracket_start;

    let array_str = &src[bracket_start + 1..bracket_end];

    // Parse hex values
    let bytes: Vec<u8> = array_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let s = s.trim_start_matches("0x");
            u8::from_str_radix(s, 16).unwrap_or_else(|_| {
                panic!("Invalid hex value: '{}'", s);
            })
        })
        .collect();

    assert_eq!(
        bytes.len(),
        expected_len,
        "Expected {} bytes for {}, got {}",
        expected_len,
        name,
        bytes.len()
    );

    bytes
}

/// Parse `PCR[i] = 0x...` from QEMU output.
fn parse_pcr(output: &str, index: usize) -> Option<[u8; 32]> {
    let prefix = format!("PCR[{}] = 0x", index);
    let line = output.lines().find(|l| l.contains(&prefix))?;
    let hex_start = line.find(&prefix)? + prefix.len();
    let hex_str: String = line[hex_start..]
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();

    if hex_str.len() != 64 {
        return None;
    }

    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex_str[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

/// Format bytes as hex string.
fn hex_str(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}
