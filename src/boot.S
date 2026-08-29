/* boot.S — RISC-V 64 entry point for QEMU virt machine.
 *
 * Sets up a minimal stack and jumps to the Rust _start() function.
 * QEMU virt loads the ELF at 0x80200000 by default.
 */

    .section .text.start, "ax", @progbits
    .globl _start
    .type _start, @function
_start:
    /* Mask all interrupts (supervisor mode, no interrupts needed yet). */
    csrw sie, zero
    csrw sip, zero

    /* Set up a 16 KiB stack below the load address. */
    la   sp, _stack_top

    /* Clear the BSS section. */
    la   t0, _bss_start
    la   t1, _bss_end
.Lclear_bss:
    bgeu  t0, t1, .Lbss_done
    sd    zero, 0(t0)
    addi  t0, t0, 8
    j     .Lclear_bss
.Lbss_done:

    /* Jump to Rust entry. */
    jal   ra, rust_main

    /* If rust_main returns, spin. */
.Lhalt:
    wfi
    j     .Lhalt

    .section .bss.stack, "aw", @progbits
    .align 4
    .globl _stack_bottom
_stack_bottom:
    .space 16384  /* 16 KiB stack */
    .globl _stack_top
_stack_top:
