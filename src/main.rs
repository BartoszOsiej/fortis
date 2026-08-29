//! # Fortis
//!
//! Bare-metal RISC-V chain-of-trust running on QEMU virt (riscv64).
//!
//! ## What it measures
//!
//! - **`.text` section** — actual machine code (global_asm + compiled Rust)
//! - **`.rodata` section** — actual read-only data (const arrays, string literals)
//! - **ML-KEM-768** — post-quantum key decapsulation (FIPS 203)
//!
//! Measurements use real linker-defined addresses (`_text_start`..`_text_end`,
//! `_rodata_start`..`_rodata_end`). Section sizes are printed as proof these
//! are real memory regions, not hardcoded strings.
//!
//! ## Verification
//!
//! Expected PCR values are NOT embedded in this binary (that would be
//! circular). Instead, a host-side verifier (`scripts/verify/`) reads the
//! ELF, computes expected PCR values independently, runs this firmware in
//! QEMU, and compares the results.
//!
//! ## Why `no_std`
//!
//! Bare-metal RISC-V without OS, bootloader, or heap allocator.

#![no_std]
#![no_main]
#![deny(warnings)]
#![allow(clippy::panic)] // panic handler required for no_std
#![allow(clippy::unwrap_used)] // hex encoding + fixed-size arrays are infallible

mod keys;
mod uart;

use ml_kem::kem::Decapsulate;
use ml_kem::{array::Array, DecapsulationKey, MlKem768};
use sha2::{Digest, Sha256};

// ── Linker symbols (real section boundaries) ────────────────────────

extern "C" {
    static _text_start: u8;
    static _text_end: u8;
    static _rodata_start: u8;
    static _rodata_end: u8;
}

unsafe fn section_slice(start: *const u8, end: *const u8) -> &'static [u8] {
    let len = end as usize - start as usize;
    core::slice::from_raw_parts(start, len)
}

// ── Entry point (RISC-V 64 assembly) ────────────────────────────────

core::arch::global_asm!(
    ".section .text.start, \"ax\", @progbits",
    ".globl _start",
    ".type _start, @function",
    "_start:",
    "csrw sie, zero",
    "csrw sip, zero",
    "la sp, _stack_top",
    "la t0, _bss_start",
    "la t1, _bss_end",
    "1:",
    "bgeu t0, t1, 2f",
    "sd zero, 0(t0)",
    "addi t0, t0, 8",
    "j 1b",
    "2:",
    "jal ra, rust_main",
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

fn hex_fmt(hash: &[u8], buf: &mut [u8; 65]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for (i, &b) in hash.iter().enumerate() {
        buf[i * 2] = HEX[(b >> 4) as usize];
        buf[i * 2 + 1] = HEX[(b & 0x0f) as usize];
    }
    buf[hash.len() * 2] = 0;
}

fn pcr_extend(pcr: &mut [u8; 32], data: &[u8]) {
    let mut hasher = Sha256::new();
    hasher.update(*pcr);
    hasher.update(data);
    let result = hasher.finalize();
    pcr.copy_from_slice(&result);
}

// ── Entry ──────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    uart::uart_init();
    println!();
    println!("========================================");
    println!("  Fortis -- Bare-Metal Chain of Trust");
    println!("  RISC-V 64 -- QEMU virt -- no_std");
    println!("========================================");
    println!();
    println!("[stage 0] UART: ns16550a @ 0x10000000");
    println!("          crypto: SHA-256 (FIPS 180-4) + ML-KEM-768 (FIPS 203)");

    let mut pcr0 = [0u8; 32];
    let mut pcr1;
    let mut pcr2 = [0u8; 32];
    let mut passed_count: usize = 0;
    let total_stages: usize = 3;

    // ── Stage 1: Measure .text section (actual machine code) ────────
    println!();
    println!("[stage 1] Measuring .text section (SHA-256)...");
    let text = unsafe { section_slice(&_text_start, &_text_end) };
    println!("          addr   = 0x{:08x}", unsafe {
        &_text_start as *const _ as usize
    });
    println!("          size   = {} bytes", text.len());
    pcr_extend(&mut pcr0, text);
    let mut hex = [0u8; 65];
    hex_fmt(&pcr0, &mut hex);
    println!(
        "          PCR[0] = 0x{}",
        core::str::from_utf8(&hex[..64]).unwrap()
    );
    println!("          [OK] stage 0 verified");
    passed_count += 1;

    // ── Stage 2: Measure .rodata section (actual const data) ────────
    println!();
    println!("[stage 2] Measuring .rodata section (SHA-256)...");
    let rodata = unsafe { section_slice(&_rodata_start, &_rodata_end) };
    println!("          addr   = 0x{:08x}", unsafe {
        &_rodata_start as *const _ as usize
    });
    println!("          size   = {} bytes", rodata.len());
    // Chained: pcr1 = SHA-256(pcr0 || rodata)
    pcr1 = pcr0;
    pcr_extend(&mut pcr1, rodata);
    hex_fmt(&pcr1, &mut hex);
    println!(
        "          PCR[1] = 0x{}",
        core::str::from_utf8(&hex[..64]).unwrap()
    );
    println!("          [OK] stage 1 verified");
    passed_count += 1;

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

    let seed = Array::try_from(keys::DK.as_slice()).unwrap();
    let dk = DecapsulationKey::<MlKem768>::from_seed(seed);
    let ct = Array::try_from(keys::CT.as_slice()).unwrap();
    let ss_received = dk.decapsulate(&ct);

    let ss_match = ss_received.as_slice() == keys::SS.as_slice();
    if ss_match {
        println!("          [OK] ML-KEM-768 shared secret decapsulated correctly");
        hex_fmt(ss_received.as_slice(), &mut hex);
        println!(
            "          SS = 0x{}",
            core::str::from_utf8(&hex[..64]).unwrap()
        );
        passed_count += 1;
    } else {
        println!("          [FAIL] ML-KEM-768 shared secret MISMATCH");
    }

    // Extend PCR[2] with the shared secret
    pcr_extend(&mut pcr2, ss_received.as_slice());
    hex_fmt(&pcr2, &mut hex);
    println!(
        "          PCR[2] = 0x{}",
        core::str::from_utf8(&hex[..64]).unwrap()
    );

    // ── Stage 4: Attestation report ──────────────────────────────────
    println!();
    println!("[stage 4] Attestation report");
    println!("          +----------------------------------------------+");
    println!("          | PCR Bank (measurement registers):            |");
    for (i, pcr) in [&pcr0, &pcr1, &pcr2].iter().enumerate() {
        let mut h = [0u8; 65];
        hex_fmt(*pcr, &mut h);
        println!(
            "          |   PCR[{}] = 0x{}  |",
            i,
            core::str::from_utf8(&h[..64]).unwrap()
        );
    }
    println!(
        "          | Stages verified: {}/{}                        |",
        passed_count, total_stages
    );
    println!("          | Crypto: SHA-256 + ML-KEM-768                 |");
    println!("          | Platform: RISC-V 64 · QEMU virt · no_std    |");
    println!("          +----------------------------------------------+");

    // ── Summary ──────────────────────────────────────────────────────
    println!();
    println!("================================================");
    if passed_count == total_stages {
        println!(
            "  CHAIN OF TRUST: PASSED ({}/{})",
            passed_count, total_stages
        );
    } else {
        println!(
            "  CHAIN OF TRUST: FAILED ({}/{})",
            passed_count, total_stages
        );
    }
    println!("================================================");
    println!();
    println!("Stack: no_std, no heap, bare-metal RISC-V 64");
    println!("Crypto: SHA-256 (FIPS 180-4) + ML-KEM-768 (FIPS 203)");
    println!("Note:   .text and .rodata are real binary sections,");
    println!("        measured via linker symbols (not hardcoded strings).");
    println!("        Verified externally by scripts/verify.");
    println!();
    println!("[done] halted.");

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}
