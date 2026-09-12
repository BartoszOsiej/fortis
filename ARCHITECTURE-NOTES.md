## Architecture

fortis is a **bare-metal RISC-V chain-of-trust** implementing SHA-256 and ML-KEM-768 post-quantum cryptography without an OS. It runs directly on RISC-V hardware (QEMU or real SiFive boards) with a custom boot chain.

### Boot Chain & Trust Model

```mermaid
graph TB
    subgraph "Immutable Boot ROM"
        RESET["Reset Vector<br/>0x1000_0000"]
        ROM_HASH["SHA-256 ROM Hash<br/>hardware fuse"]
    end

    subgraph "Stage 1 — Bootloader"
        SPL["SPL (Secondary Program Loader)<br/>Verifies Stage 2 signature"]
        SPL_VERIFY["ML-KEM-768 Decapsulation<br/>Post-quantum signature check"]
    end

    subgraph "Stage 2 — Kernel"
        KERNEL["fortis Kernel<br/>Bare-metal RISC-V M-mode"]
        MEM_INIT["Memory Init<br/>DDR + paging"]
        CRYPTO_INIT["Crypto Engine Init<br/>SHA-256 + AES-NI"]
    end

    subgraph "Stage 3 — Runtime"
        CHAIN["Chain-of-Trust Manager<br/>Measurement + attestation"]
        APP["User Application<br/>Ring 0 (M-mode)"]
        SEAL["Secure Storage<br/>ML-KEM sealed keys"]
    end

    RESET -->|"hash compare"| ROM_HASH
    ROM_HASH -->|"match ✓"| SPL
    SPL -->|"ML-KEM decap"| SPL_VERIFY
    SPL_VERIFY -->|"valid signature ✓"| KERNEL
    KERNEL --> MEM_INIT
    KERNEL --> CRYPTO_INIT
    KERNEL --> CHAIN
    CHAIN --> APP
    CHAIN --> SEAL

    style RESET fill:#DA2C38,color:#fff,stroke:none
    style SPL_VERIFY fill:#F15A24,color:#000,stroke:none
    style CHAIN fill:#F15A24,color:#000,stroke:none
```

### Cryptographic Pipeline

```mermaid
flowchart LR
    A["ML-KEM-768<br/>Key Generation"] --> B["Ciphertext<br/>1088 bytes"]
    B --> C["Decapsulation<br/>→ Shared Secret"]
    C --> D["HKDF-SHA256<br/>Key Derivation"]
    D --> E["AES-256-GCM<br/>Authenticated Encryption"]
    D --> F["HMAC-SHA256<br/>Integrity Tag"]
    E --> G["Sealed Storage<br/>Disk/Flash"]
    F --> G

    H["Measurement Log"] --> I["PCR Register<br/>Extends with hash"]
    I --> J["Attestation Quote<br/>ML-KEM signed"]
    J --> K["Verifier<br/>Remote party"]

    style A fill:#F15A24,color:#000,stroke:none
    style C fill:#DA2C38,color:#fff,stroke:none
    style E fill:#1a1a2e,color:#F15A24,stroke:#F15A24
```

### Memory Layout

```mermaid
graph LR
    subgraph "Physical Memory Map"
        ROM["0x1000_0000<br/>Boot ROM (32 KB)"]
        SPL_MEM["0x1000_8000<br/>SPL (64 KB)"]
        KERNEL_MEM["0x8000_0000<br/>Kernel (2 MB)"]
        HEAP["0x8020_0000<br/>Heap (64 MB)"]
        CRYPTO_MEM["0x8400_0000<br/>Crypto Buffers (1 MB)"]
        SECURE["0x8500_0000<br/>Secure Storage (8 MB)"]
    end

    ROM --> SPL_MEM --> KERNEL_MEM
    KERNEL_MEM --> HEAP
    KERNEL_MEM --> CRYPTO_MEM
    KERNEL_MEM --> SECURE
```

## Quickstart

### One-liner — QEMU boot (no hardware needed)

```bash
git clone https://github.com/BartoszOsiej/fortis && cd fortis && make qemu
```

### One-liner — Full build (requires riscv64 cross-compiler)

```bash
git clone https://github.com/BartoszOsiej/fortis && cd fortis && \
  export CROSS_COMPILE=riscv64-linux-gnu- && make clean && make -j$(nproc) && \
  qemu-system-riscv64 -machine virt -bios none -kernel build/fortis.bin -nographic
```

### One-liner — Docker (cross-compile environment)

```bash
docker run --rm -v "$(pwd)":/build -w /build \
  ghcr.io/bartoszosiej/riscv-toolchain:latest \
  make -j$(nproc) && \
  qemu-system-riscv64 -machine virt -bios none -kernel build/fortis.bin -nographic
```

### One-liner — Run crypto benchmarks

```bash
cd fortis && make bench && ./build/bench --crypto
```

### Verify

```bash
# After boot, in the fortis shell:
help                    # list commands
crypto-test             # run SHA-256 + ML-KEM-768 self-test
chain verify            # verify boot chain integrity
mem dump 0x80000000 64  # dump first 64 bytes of kernel
```
