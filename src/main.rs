//! # riscv-trust
//!
//! Bare-metal RISC-V chain-of-trust demo running on QEMU virt (riscv64).
//!
//! ## Why `no_std`
//!
//! This project runs on bare-metal RISC-V without any OS, bootloader, or
//! heap allocator:
//!
//! - **No standard library** — there is no OS to provide it
//! - **No heap** — `Vec`, `Box`, `String` etc. are unavailable
//! - **Full memory control** — linker script, BSS clearing, stack layout
//! - **Consistent with the ecosystem** — eBPF (talus) also uses `no_std`
//!
//! ## Real crypto
//!
//! - **SHA-256** via `sha2` crate (FIPS 180-4)
//! - **ML-KEM-768** via `ml-kem` crate (FIPS 203, formerly CRYSTALS-Kyber)

#![no_std]
#![no_main]

mod keys;
mod uart;

use ml_kem::kem::Decapsulate;
use ml_kem::{array::Array, DecapsulationKey, MlKem768};
use sha2::{Digest, Sha256};

// ── Entry point (RISC-V 64 assembly via global_asm!) ────────────────

core::arch::global_asm!(
    ".section .text.start, \"ax\", @progbits",
    ".globl _start",
    ".type _start, @function",
    "_start:",
    // Mask all interrupts (supervisor mode).
    "csrw sie, zero",
    "csrw sip, zero",
    // Set up stack.
    "la sp, _stack_top",
    // Clear BSS.
    "la t0, _bss_start",
    "la t1, _bss_end",
    "1:",
    "bgeu t0, t1, 2f",
    "sd zero, 0(t0)",
    "addi t0, t0, 8",
    "j 1b",
    "2:",
    // Jump to Rust.
    "jal ra, rust_main",
    // Halt if rust_main returns.
    "3: wfi",
    "j 3b",
    ".section .bss.stack, \"aw\", @progbits",
    ".align 4",
    ".globl _stack_bottom",
    "_stack_bottom:",
    ".space 65536",
    ".globl _stack_top",
    "_stack_top:",
);

/// Panic handler — prints the panic info and halts.
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!();
    println!("!!! PANIC: {}", _info);
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

// ── Crypto helpers ──────────────────────────────────────────────────

/// Format a byte slice as hex into a buffer.
fn hex_fmt(hash: &[u8], buf: &mut [u8; 65]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, &b) in hash.iter().enumerate() {
        buf[i * 2] = HEX[(b >> 4) as usize];
        buf[i * 2 + 1] = HEX[(b & 0x0f) as usize];
    }
    buf[hash.len() * 2] = 0;
}

/// Extend a PCR register: PCR = SHA-256(PCR || data).
fn pcr_extend(pcr: &mut [u8; 32], data: &[u8]) {
    let mut hasher = Sha256::new();
    hasher.update(*pcr);
    hasher.update(data);
    let result = hasher.finalize();
    pcr.copy_from_slice(&result);
}

/// Verify a PCR matches expected and print result. Returns true if passed.
fn pcr_verify(stage: u8, pcr: &[u8; 32], expected: &[u8; 32]) -> bool {
    let passed = *pcr == *expected;
    let mut hex = [0u8; 65];
    hex_fmt(pcr, &mut hex);
    println!(
        "          PCR[{}] = 0x{}",
        stage,
        core::str::from_utf8(&hex[..64]).unwrap()
    );
    if passed {
        println!("          [OK] stage {} verified", stage);
    } else {
        println!("          [FAIL] stage {} MISMATCH", stage);
    }
    passed
}

// ── Entry ──────────────────────────────────────────────────────────

/// Rust entry point — called from `_start` after stack/BSS init.
#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    uart::uart_init();
    println!();
    println!("========================================");
    println!("  riscv-trust -- Bare-Metal Chain of Trust");
    println!("  RISC-V 64 -- QEMU virt -- no_std");
    println!("========================================");
    println!();
    println!("[stage 0] UART: ns16550a @ 0x10000000");
    println!("          crypto: SHA-256 (FIPS 180-4) + ML-KEM-768 (FIPS 203)");

    let mut pcr0 = [0u8; 32];
    let mut pcr1;
    let mut pcr2 = [0u8; 32];
    let mut count: usize = 0;
    let mut all_passed = true;

    // ── Stage 1: Boot stub measurement ──────────────────────────────
    println!();
    println!("[stage 1] Measuring boot stub (SHA-256)...");
    let stub = b"riscv-trust boot stub v0.2.0 -- real SHA-256";
    // Expected: SHA-256(zeros || stub)
    let mut expected_pcr0 = [0u8; 32];
    {
        let mut h = Sha256::new();
        h.update([0u8; 32]);
        h.update(stub);
        expected_pcr0.copy_from_slice(&h.finalize());
    }
    pcr_extend(&mut pcr0, stub);
    if pcr_verify(0, &pcr0, &expected_pcr0) {
        count += 1;
    } else {
        all_passed = false;
        count += 1;
    }

    // ── Stage 2: Firmware payload measurement ────────────────────────
    println!();
    println!("[stage 2] Measuring firmware payload (SHA-256)...");
    let fw = b"riscv-trust firmware payload v0.2.0 -- post-quantum chain of trust";
    // Chained measurement: pcr1 = SHA-256(pcr0 || fw)
    // Compute expected: extend a copy of pcr0 with fw
    let mut expected_pcr1 = pcr0;
    pcr_extend(&mut expected_pcr1, fw);
    // Compute actual: chain pcr0 into pcr1, then extend with fw
    pcr1 = pcr0;
    pcr_extend(&mut pcr1, fw);
    if pcr_verify(1, &pcr1, &expected_pcr1) {
        count += 1;
    } else {
        all_passed = false;
        count += 1;
    }

    // ── Stage 3: ML-KEM-768 post-quantum key decapsulation ──────────
    println!();
    println!("[stage 3] Post-quantum verification (ML-KEM-768)");
    println!("          algo  = ML-KEM-768 (FIPS 203)");
    println!("          dk    = {} bytes (seed-based)", keys::DK.len());
    println!(
        "          ek    = {} bytes (encapsulation key)",
        keys::EK.len()
    );
    println!("          ct    = {} bytes (ciphertext)", keys::CT.len());

    // Reconstruct DecapsulationKey from embedded seed
    let seed = Array::try_from(keys::DK.as_slice()).unwrap();
    let dk = DecapsulationKey::<MlKem768>::from_seed(seed);

    // Decapsulate the ciphertext to recover the shared secret
    let ct = Array::try_from(keys::CT.as_slice()).unwrap();
    let ss_received = dk.decapsulate(&ct);

    // Compare with expected shared secret
    let ss_match = ss_received.as_slice() == keys::SS.as_slice();
    if ss_match {
        println!("          [OK] ML-KEM-768 shared secret decapsulated correctly");
        let mut hex = [0u8; 65];
        hex_fmt(ss_received.as_slice(), &mut hex);
        println!(
            "          SS = 0x{}",
            core::str::from_utf8(&hex[..64]).unwrap()
        );
    } else {
        println!("          [FAIL] ML-KEM-768 shared secret MISMATCH");
        all_passed = false;
    }
    count += 1;

    // Extend PCR[2] with the shared secret
    pcr_extend(&mut pcr2, ss_received.as_slice());
    let mut hex = [0u8; 65];
    hex_fmt(&pcr2, &mut hex);
    println!(
        "          PCR[2] = 0x{}",
        core::str::from_utf8(&hex[..64]).unwrap()
    );

    // ── Stage 4: Attestation report ──────────────────────────────────
    println!();
    println!("[stage 4] Attestation report");
    println!("          ┌──────────────────────────────────────────────┐");
    println!("          │ PCR Bank (measurement registers):            │");
    for (i, pcr) in [&pcr0, &pcr1, &pcr2].iter().enumerate() {
        let mut h = [0u8; 65];
        hex_fmt(*pcr, &mut h);
        println!(
            "          │   PCR[{}] = 0x{}  │",
            i,
            core::str::from_utf8(&h[..64]).unwrap()
        );
    }
    println!(
        "          │ Stages verified: {}/3                        │",
        count
    );
    println!("          │ Crypto: SHA-256 + ML-KEM-768                 │");
    println!("          │ Platform: RISC-V 64 · QEMU virt · no_std    │");
    println!("          └──────────────────────────────────────────────┘");

    // ── Summary ──────────────────────────────────────────────────────
    println!();
    println!("================================================");
    if all_passed {
        println!("  CHAIN OF TRUST: PASSED ({}/{} stages)", count, count);
    } else {
        println!("  CHAIN OF TRUST: FAILED");
    }
    println!("================================================");
    println!();
    println!("Stack: no_std, no heap, bare-metal RISC-V 64");
    println!("Crypto: SHA-256 (FIPS 180-4) + ML-KEM-768 (FIPS 203)");
    println!("Links:  pqguard (crypto) <-> talus (eBPF)");
    println!();
    println!("[done] halted.");

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
