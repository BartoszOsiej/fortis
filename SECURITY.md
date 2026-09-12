# Security Policy — fortis

> **fortis implements a bare-metal chain-of-trust for RISC-V.** A vulnerability could compromise the entire boot chain, allowing unsigned code execution. We treat security reports with the highest priority.

## Supported Versions

| Version | Supported |
|---|---|
| `main` branch (latest) | ✅ Active |
| Older releases | ⚠️ Best-effort |

## Reporting Vulnerabilities

**DO NOT** open a public GitHub issue.

1. **GitHub Security Advisories** (preferred):
   https://github.com/BartoszOsiej/fortis/security/advisories/new

2. **Encrypted email**:
   ```
   thethreadcalls@outlook.com
   Subject: [SECURITY] fortis — <brief description>
   ```

### What to Include

- Vulnerability type: boot chain bypass, crypto weakness, memory corruption, attestation forgery
- Affected components: SPL, boot ROM verification, ML-KEM-768, SHA-256, chain-of-trust manager
- RISC-V ISA version affected (RV64GC, etc.)
- QEMU or hardware reproduction steps
- Impact assessment (boot chain bypass, unsigned code execution, key extraction, attestation bypass)

## Response SLAs

| Severity | Initial Response | Fix SLA |
|---|---|---|
| **Critical** (9.0–10.0) | 24 hours | 7 days |
| **High** (7.0–8.9) | 48 hours | 14 days |
| **Medium** (4.0–6.9) | 72 hours | 30 days |
| **Low** (0.1–3.9) | 7 days | 90 days |

## Scope

### In Scope

- Boot chain verification bypass (signature forgery, hash collision)
- Cryptographic weaknesses in ML-KEM-768 or SHA-256 implementation
- Memory corruption in bare-metal code (no MMU protection)
- Side-channel attacks on crypto operations (timing, power analysis)
- Attestation quote forgery
- Secure storage key extraction
- RISC-V privilege escalation (M-mode → S-mode bypass)

### Out of Scope

- RISC-V hardware vulnerabilities (report to SiFive/ARM)
- Vulnerabilities requiring physical JTAG access (expected for hardware debugging)
- Social engineering

## Security Design

1. **Immutable boot ROM**: First-stage hash is hardware-fused, cannot be modified.
2. **ML-KEM-768**: NIST FIPS 203 post-quantum signature scheme — quantum-resistant.
3. **Measurement log**: PCR-style register extends with every boot stage, prevents tampering.
4. **Constant-time crypto**: All ML-KEM and SHA-256 operations are constant-time.
5. **No dynamic allocation**: Bare-metal, no heap allocator — eliminates heap corruption classes.

## Contact

| Channel | Details |
|---|---|
| GitHub Advisory | https://github.com/BartoszOsiej/fortis/security/advisories |
| Email | thethreadcalls@outlook.com |
| maintainer | @BartoszOsiej |
