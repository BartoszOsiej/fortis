# riscv-trust

**Bare-metal RISC-V chain-of-trust demo** — real SHA-256, real ML-KEM-768
post-quantum key decapsulation, MMIO UART, PCR measurement registers,
on QEMU virt.

## What it demonstrates

| Feature | Implementation |
|---------|---------------|
| SHA-256 | `sha2` crate (FIPS 180-4) — real SHA-256, not a placeholder |
| ML-KEM-768 | `ml-kem` crate (FIPS 203) — post-quantum key encapsulation |
| MMIO UART | ns16550a driver for QEMU virt (`0x10000000`) |
| Measured boot | SHA-256 PCR registers (extend + verify each stage) |
| Chain of trust | 3-stage measurement with attestation report |
| Binary size | Under 64 KiB code (stripped, LTO, `opt-level=z`) |

## Why `no_std`

This project runs on **bare-metal RISC-V** without any OS, bootloader, or
heap allocator:

- **No standard library** — there is no OS to provide it
- **No heap** — `Vec`, `Box`, `String` etc. are unavailable
- **Full memory control** — linker script, BSS clearing, stack layout
- **Consistent with the ecosystem** — eBPF (talus) also uses `no_std`

The `no_std` attribute is not a limitation here — it's the correct choice
for firmware that must be small, fast, and verifiable.

## Architecture

```
┌─────────────────────────────────────────────────────┐
│ Stage 0: Reset vector → global_asm! _start          │
│   • Mask interrupts (supervisor mode)               │
│   • Set up 16 KiB stack                             │
│   • Clear BSS section                               │
│   • Jump to rust_main()                              │
├─────────────────────────────────────────────────────┤
│ Stage 1: Boot stub measurement                      │
│   • SHA-256(zeros || stub) → PCR[0]                 │
│   • Verify integrity before proceeding               │
├─────────────────────────────────────────────────────┤
│ Stage 2: Firmware payload measurement                │
│   • SHA-256(PCR[0] || payload) → PCR[1]            │
│   • Chained measurement (PCR[0] feeds into PCR[1])  │
├─────────────────────────────────────────────────────┤
│ Stage 3: ML-KEM-768 post-quantum verification       │
│   • Decapsulate ciphertext → shared secret           │
│   • Verify shared secret matches expected            │
│   • Extend PCR[2] with shared secret                 │
├─────────────────────────────────────────────────────┤
│ Stage 4: Attestation report                         │
│   • Print PCR bank (PCR[0], PCR[1], PCR[2])         │
│   • Chain of trust verdict: PASSED / FAILED          │
└─────────────────────────────────────────────────────┘
```

## Crypto details

### SHA-256 (FIPS 180-4)

```rust
// PCR extend: PCR = SHA-256(PCR_old || new_data)
let mut hasher = Sha256::new();
hasher.update(pcr);
hasher.update(data);
pcr.copy_from_slice(&hasher.finalize());
```

### ML-KEM-768 (FIPS 203)

```rust
// Pre-generated keypair (from keygen tool)
let dk = DecapsulationKey::<MlKem768>::from_seed(seed);
let ct = Array::try_from(CT.as_slice()).unwrap();

// Decapsulate ciphertext → shared secret
let ss = dk.decapsulate(&ct);
// ss is the 32-byte shared secret
```

The keypair and ciphertext are pre-generated using a deterministic keygen
tool (runs on host with OS randomness), then embedded as constants in the
bare-metal firmware.

## Connections to the portfolio

```
pqguard (post-quantum crypto)  ←→  riscv-trust (firmware verification)
        ↓                                    ↓
   ML-KEM-768                         Chain of trust
        ↓                                    ↓
talus (eBPF monitoring)        ←→  Runtime attestation
```

## Running

### Prerequisites

```bash
# Install Rust target
rustup target add riscv64gc-unknown-none-elf
rustup component add rust-src

# Install QEMU (Ubuntu/Debian)
sudo apt install qemu-system-misc
```

### Build

```bash
cargo build --target riscv64gc-unknown-none-elf --release
```

### Run in QEMU

```bash
qemu-system-riscv64 \
    -machine virt \
    -bios default \
    -nographic \
    -kernel target/riscv64gc-unknown-none-elf/release/riscv-trust
```

### Expected output

```
========================================
  riscv-trust -- Bare-Metal Chain of Trust
  RISC-V 64 -- QEMU virt -- no_std
========================================

[stage 0] UART: ns16550a @ 0x10000000
          crypto: SHA-256 (FIPS 180-4) + ML-KEM-768 (FIPS 203)

[stage 1] Measuring boot stub (SHA-256)...
          PCR[0] = 0x<a3b4c5d6...>
          [OK] stage 0 verified

[stage 2] Measuring firmware payload (SHA-256)...
          PCR[1] = 0x<f7e8d9c0...>
          [OK] stage 1 verified

[stage 3] Post-quantum verification (ML-KEM-768)
          algo  = ML-KEM-768 (FIPS 203)
          dk    = 64 bytes (seed-based)
          ek    = 1184 bytes (encapsulation key)
          ct    = 1088 bytes (ciphertext)
          [OK] ML-KEM-768 shared secret decapsulated correctly
          SS = 0x<1234567890abcdef...>
          PCR[2] = 0x<abcdef1234567890...>

[stage 4] Attestation report
          ┌──────────────────────────────────────────────┐
          │ PCR Bank (measurement registers):            │
          │   PCR[0] = 0x...  │
          │   PCR[1] = 0x...  │
          │   PCR[2] = 0x...  │
          │ Stages verified: 3/3                        │
          │ Crypto: SHA-256 + ML-KEM-768                 │
          │ Platform: RISC-V 64 · QEMU virt · no_std    │
          └──────────────────────────────────────────────┘

================================================
  CHAIN OF TRUST: PASSED (3/3 stages)
================================================

Stack: no_std, no heap, bare-metal RISC-V 64
Crypto: SHA-256 (FIPS 180-4) + ML-KEM-768 (FIPS 203)
Links:  pqguard (crypto) <-> talus (eBPF)

[done] halted.
```

## Project structure

```
riscv-trust/
├── src/
│   ├── main.rs          # Entry point + chain of trust demo
│   ├── uart.rs          # MMIO UART driver (ns16550a)
│   └── keys.rs          # Pre-generated ML-KEM-768 keypair + ciphertext
├── link.ld              # Linker script (QEMU virt, 0x80200000)
├── build.rs             # Build script (linker flags)
├── Cargo.toml
├── .github/workflows/
│   └── ci.yml           # Build + QEMU test + size check + clippy
└── README.md
```

## Key generation

The ML-KEM-768 keypair and ciphertext are generated on the host using
a deterministic seed (no OS randomness needed at runtime):

```bash
# This is a one-shot process — keys are embedded in keys.rs
# The keygen tool uses ml-kem with hazmat feature for deterministic ops
```

## License

MIT
