# Error Codex — fortis Bare-Metal RISC-V

> Complete reference for every hardware fault trap and boot chain error code (FORT_ERR_01 through FORT_ERR_40). Each entry includes the trap vector, structural explanation, register dump format, and recovery path.

---

## Legend

| Field | Description |
|---|---|
| **Code** | Unique error identifier |
| **Phase** | Boot stage or runtime phase |
| **RISC-V Exception** | Associated RISC-V exception code (if applicable) |
| **Recovery** | Whether recovery is possible or system halts |

---

## Boot Chain Errors

### FORT_ERR_01 — ROM Hash Verification Failed

```
Phase: Stage 0 (Boot ROM)
RISC-V Exception: None
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_01] ROM hash verification FAILED
  Expected: {expected_hash}
  Actual:   {actual_hash}
  HALTING — system may be tampered
```

**Explanation:** The SHA-256 hash of the boot ROM does not match the hardware-fused reference hash. This indicates the boot ROM has been modified — a critical security failure.

**Register Dump:**
```
mepc  = 0x1000_0000  (ROM entry point)
mcause = 0 (no exception)
mtval  = 0x0000_0000
a0     = {expected_hash_low}
a1     = {expected_hash_high}
a2     = {actual_hash_low}
a3     = {actual_hash_high}
```

**Recovery:** None. System halts permanently. Requires physical reprogramming of the boot ROM.

---

### FORT_ERR_02 — SPL Signature Verification Failed

```
Phase: Stage 1 (SPL)
RISC-V Exception: None
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_02] SPL ML-KEM-768 signature verification FAILED
  Public key: {pubkey_hex}
  Ciphertext: {ct_hex}
  HALTING — SPL may be tampered
```

**Explanation:** The SPL (Secondary Program Loader) signature does not verify against the trusted public key. The ML-KEM-768 decapsulation failed — the SPL was not signed by the trusted authority.

**Recovery:** None. System halts permanently.

---

### FORT_ERR_03 — SPL Hash Mismatch

```
Phase: Stage 1 (SPL)
RISC-V Exception: None
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_03] SPL hash mismatch
  Expected: {expected}
  Actual:   {actual}
  HALTING
```

**Explanation:** Even though the signature verified, the SPL binary hash does not match the expected hash. This could indicate a replay attack (old signed SPL).

**Recovery:** None. System halts permanently.

---

### FORT_ERR_04 — Kernel Load Address Invalid

```
Phase: Stage 2 (Kernel Load)
RISC-V Exception: 0 (Instruction access fault)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_04] Kernel load address invalid: 0x{address}
  Valid range: 0x8000_0000 – 0x8FFF_FFFF
  HALTING
```

**Explanation:** The SPL tried to load the kernel at an address outside the valid DDR range.

**Recovery:** None. Requires SPL reprogramming.

---

### FORT_ERR_05 — Kernel Signature Verification Failed

```
Phase: Stage 2 (Kernel)
RISC-V Exception: None
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_05] Kernel ML-KEM-768 signature verification FAILED
  HALTING — kernel may be tampered
```

**Recovery:** None. System halts permanently.

---

## Memory Errors

### FORT_ERR_10 — DDR Initialization Failed

```
Phase: Stage 2 (Memory Init)
RISC-V Exception: 5 (Load access fault)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_10] DDR initialization failed
  Controller: {controller_id}
  Status register: 0x{status}
  HALTING
```

**Explanation:** The DDR memory controller failed to initialize. The memory is not usable.

**Recovery:** None. Hardware failure.

---

### FORT_ERR_11 — Memory Test Failed

```
Phase: Stage 2 (Memory Init)
RISC-V Exception: 5 (Load access fault) or 7 (Store access fault)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_11] Memory test failed at address 0x{address}
  Write: 0x{written}
  Read:  0x{read}
  XOR:   0x{xor_diff}
  HALTING
```

**Explanation:** The memory test (walking ones/zeros) found a faulty memory cell.

**Register Dump:**
```
mepc  = {faulting_pc}
mcause = {5 or 7}
mtval  = 0x{address}
```

**Recovery:** None. Hardware failure.

---

### FORT_ERR_12 — Stack Overflow

```
Phase: Runtime
RISC-V Exception: 1 (Instruction access fault) or 5 (Load access fault)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_12] Stack overflow detected
  Stack pointer: 0x{sp}
  Stack base:    0x{base}
  Stack limit:   0x{limit}
  HALTING
```

**Explanation:** The stack pointer went below the allocated stack area.

**Recovery:** None. Requires code modification to reduce stack usage.

---

### FORT_ERR_13 — Heap Exhaustion

```
Phase: Runtime
RISC-V Exception: None (software trap)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_13] Heap exhaustion
  Requested: {size} bytes
  Free: {free} bytes
  HALTING
```

**Explanation:** The heap allocator cannot satisfy a memory request. In bare-metal, there is no swap or OOM killer.

**Recovery:** None. Requires code modification to reduce memory usage.

---

### FORT_ERR_14 — Double-Free Detected

```
Phase: Runtime
RISC-V Exception: None (software trap)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_14] Double-free detected at 0x{address}
  First free at: {backtrace_1}
  Second free at: {backtrace_2}
  HALTING
```

**Explanation:** The same memory address was freed twice. This is a memory corruption bug.

**Recovery:** None. Requires code fix.

---

## Cryptographic Errors

### FORT_ERR_20 — SHA-256 Hash Mismatch

```
Phase: Crypto Verification
RISC-V Exception: None
Recovery: HALT (if boot chain) / Error return (if runtime)
```

**Trap Message:**
```
[FORT_ERR_20] SHA-256 hash mismatch
  Expected: {expected_hex}
  Actual:   {actual_hex}
  Context: {context_description}
```

**Explanation:** A SHA-256 hash verification failed. In boot chain context, this is fatal. In runtime context, returns an error.

**Recovery:** In boot chain: HALT. In runtime: returns `-FORT_ERR_20`.

---

### FORT_ERR_21 — ML-KEM-768 Decapsulation Failed

```
Phase: Crypto Verification
RISC-V Exception: None
Recovery: HALT (if boot chain) / Error return (if runtime)
```

**Trap Message:**
```
[FORT_ERR_21] ML-KEM-768 decapsulation failed
  Ciphertext: {ct_hex}
  Expected shared secret: {expected_ss_hex}
  Actual shared secret:   {actual_ss_hex}
```

**Explanation:** The ML-KEM-768 decapsulation produced an incorrect shared secret. The ciphertext was not generated with the corresponding public key.

**Recovery:** In boot chain: HALT. In runtime: returns `-FORT_ERR_21`.

---

### FORT_ERR_22 — AES-256-GCM Authentication Failed

```
Phase: Crypto Verification
RISC-V Exception: None
Recovery: Error return
```

**Trap Message:**
```
[FORT_ERR_22] AES-256-GCM authentication tag mismatch
  Expected tag: {expected_tag_hex}
  Actual tag:   {actual_tag_hex}
  Nonce:        {nonce_hex}
```

**Explanation:** The AES-256-GCM authentication tag does not match. The ciphertext has been tampered with.

**Recovery:** Returns `-FORT_ERR_22`. Do not use the decrypted data.

---

### FORT_ERR_23 — Key Zeroization Failed

```
Phase: Crypto Cleanup
RISC-V Exception: None
Recovery: Warning (non-fatal)
```

**Trap Message:**
```
[FORT_ERR_23] Key zeroization verification failed
  Key address: 0x{address}
  Expected: 0x0000000000000000
  Actual:   {remaining_hex}
```

**Explanation:** After a crypto operation, the key material was not properly zeroed. This could leave sensitive data in memory.

**Recovery:** Warning only. The key should be zeroed again explicitly.

---

### FORT_ERR_24 — Timing Side-Channel Detected

```
Phase: Crypto Runtime
RISC-V Exception: None
Recovery: Warning (non-fatal)
```

**Trap Message:**
```
[FORT_ERR_24] Constant-time violation detected
  Operation: {operation_name}
  Expected max: {max_cycles} cycles
  Actual:       {actual_cycles} cycles
  Deviation:    {deviation}%
```

**Explanation:** A crypto operation took a variable amount of time, indicating a potential timing side-channel.

**Recovery:** Warning only. Review the crypto implementation for data-dependent branches.

---

## Attestation Errors

### FORT_ERR_30 — Attestation Quote Invalid

```
Phase: Attestation
RISC-V Exception: None
Recovery: Error return
```

**Trap Message:**
```
[FORT_ERR_30] Attestation quote signature invalid
  Quote: {quote_hex}
  Expected signer: {expected_signer_hex}
  Actual signer:   {actual_signer_hex}
```

**Explanation:** The attestation quote was not signed by the expected key. The attestation may be forged.

**Recovery:** Returns `-FORT_ERR_30`. Reject the attestation.

---

### FORT_ERR_31 — PCR Mismatch

```
Phase: Attestation
RISC-V Exception: None
Recovery: Error return
```

**Trap Message:**
```
[FORT_ERR_31] PCR (Platform Configuration Register) mismatch
  Expected PCR[0]: {expected_pcr0}
  Actual PCR[0]:   {actual_pcr0}
  Expected PCR[1]: {expected_pcr1}
  Actual PCR[1]:   {actual_pcr1}
```

**Explanation:** The PCR values do not match the expected measurement. The system state has changed since the reference measurement.

**Recovery:** Returns `-FORT_ERR_31`. System state is untrusted.

---

### FORT_ERR_32 — Measurement Log Overflow

```
Phase: Attestation
RISC-V Exception: None
Recovery: Warning (non-fatal)
```

**Trap Message:**
```
[FORT_ERR_32] Measurement log overflow
  Current entries: {count}
  Maximum entries: {max}
  Oldest entry discarded
```

**Explanation:** The measurement log has more entries than the fixed-size buffer. Old entries are being discarded.

**Recovery:** Warning only. Older measurements are lost.

---

## Hardware Fault Traps

### FORT_ERR_35 — Instruction Access Fault

```
Phase: Runtime
RISC-V Exception: 1 (Instruction access fault)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_35] Instruction access fault
  mepc  = 0x{address}
  mcause = 1
  mtval  = 0x{faulting_address}
```

**Explanation:** The CPU tried to execute an instruction at an invalid address. Usually indicates a jump to an invalid address (wild jump, corrupted function pointer).

**Register Dump:**
```
mepc     = 0x{address}     (program counter at fault)
mcause   = 1               (instruction access fault)
mtval    = 0x{bad_address} (faulting address)
mstatus  = 0x{status}      (machine status)
ra       = 0x{return_addr} (return address)
sp       = 0x{stack_ptr}   (stack pointer)
```

**Recovery:** None. System halts.

---

### FORT_ERR_36 — Instruction Access Misaligned

```
Phase: Runtime
RISC-V Exception: 0 (Instruction address misaligned)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_36] Instruction address misaligned
  mepc  = 0x{address}
  mtval = 0x{bad_address}
```

**Explanation:** The CPU tried to execute an instruction at a misaligned address. This should never happen in normal operation.

**Recovery:** None. System halts.

---

### FORT_ERR_37 — Load Access Fault

```
Phase: Runtime
RISC-V Exception: 5 (Load access fault)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_37] Load access fault
  mepc  = 0x{pc}
  mtval = 0x{bad_address}
  Instruction: {disassembly}
```

**Explanation:** The CPU tried to load from an invalid memory address. Usually a null pointer dereference or buffer overflow.

**Recovery:** None. System halts.

---

### FORT_ERR_38 — Store/AMO Access Fault

```
Phase: Runtime
RISC-V Exception: 7 (Store/AMO access fault)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_38] Store/AMO access fault
  mepc  = 0x{pc}
  mtval = 0x{bad_address}
  Instruction: {disassembly}
```

**Explanation:** The CPU tried to store to an invalid memory address.

**Recovery:** None. System halts.

---

### FORT_ERR_39 — Environment Call from M-mode

```
Phase: Runtime
RISC-V Exception: 11 (Environment call from M-mode)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_39] Environment call from M-mode (ECALL_M)
  mepc = 0x{pc}
  a7   = {syscall_number}
```

**Explanation:** An ECALL instruction was executed from M-mode. In bare-metal, this is not expected — there is no S-mode to trap from.

**Recovery:** None. System halts.

---

### FORT_ERR_40 — Unknown Interrupt

```
Phase: Runtime
RISC-V Exception: Interrupt (mcause < 0)
Recovery: HALT
```

**Trap Message:**
```
[FORT_ERR_40] Unknown interrupt
  mcause = 0x{cause}
  mepc   = 0x{pc}
  mtval  = 0x{value}
```

**Explanation:** An interrupt was raised that has no handler registered.

**Recovery:** None. System halts.

---

## Quick Reference Table

| Code | Phase | RISC-V Exc | Severity | Short Description |
|---|---|---|---|---|
| FORT_ERR_01 | Boot ROM | None | HALT | ROM hash verification failed |
| FORT_ERR_02 | SPL | None | HALT | SPL signature failed |
| FORT_ERR_03 | SPL | None | HALT | SPL hash mismatch |
| FORT_ERR_04 | Kernel Load | 1 | HALT | Invalid load address |
| FORT_ERR_05 | Kernel | None | HALT | Kernel signature failed |
| FORT_ERR_10 | Memory Init | 5 | HALT | DDR init failed |
| FORT_ERR_11 | Memory Test | 5/7 | HALT | Memory test failed |
| FORT_ERR_12 | Runtime | 1/5 | HALT | Stack overflow |
| FORT_ERR_13 | Runtime | None | HALT | Heap exhaustion |
| FORT_ERR_14 | Runtime | None | HALT | Double-free |
| FORT_ERR_20 | Crypto | None | HALT/Error | SHA-256 mismatch |
| FORT_ERR_21 | Crypto | None | HALT/Error | ML-KEM-768 decap failed |
| FORT_ERR_22 | Crypto | None | Error | AES-GCM auth failed |
| FORT_ERR_23 | Crypto | None | Warning | Key zeroization failed |
| FORT_ERR_24 | Crypto | None | Warning | Timing side-channel |
| FORT_ERR_30 | Attestation | None | Error | Quote invalid |
| FORT_ERR_31 | Attestation | None | Error | PCR mismatch |
| FORT_ERR_32 | Attestation | None | Warning | Log overflow |
| FORT_ERR_35 | Runtime | 1 | HALT | Instruction access fault |
| FORT_ERR_36 | Runtime | 0 | HALT | Instruction misaligned |
| FORT_ERR_37 | Runtime | 5 | HALT | Load access fault |
| FORT_ERR_38 | Runtime | 7 | HALT | Store/AMO access fault |
| FORT_ERR_39 | Runtime | 11 | HALT | ECALL from M-mode |
| FORT_ERR_40 | Runtime | <0 | HALT | Unknown interrupt |

---

*Last updated: 2026-09-11. 24 error codes documented for RISC-V bare-metal.*
