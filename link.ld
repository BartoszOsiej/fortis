/* link.ld — Minimal linker script for QEMU riscv64 virt machine.
 *
 * QEMU virt loads the firmware at 0x80200000 (SBI jump address).
 * We define .text.start first so _start is at the entry point.
 */

OUTPUT_FORMAT("elf64-littleriscv")
OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    RAM (rwx) : ORIGIN = 0x80200000, LENGTH = 128M
}

SECTIONS
{
    . = ORIGIN(RAM);

    /* Code — .text.start must be first for the entry point. */
    .text : ALIGN(4K)
    {
        _text_start = .;
        KEEP(*(.text.start))
        *(.text .text.*)
        _text_end = .;
    }

    /* Read-only data. */
    .rodata : ALIGN(4K)
    {
        _rodata_start = .;
        *(.rodata .rodata.*)
        _rodata_end = .;
    }

    /* Data. */
    .data : ALIGN(4K)
    {
        _data_start = .;
        *(.data .data.*)
        _data_end = .;
    }

    /* BSS — zero-initialized. */
    .bss : ALIGN(4K)
    {
        _bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        _bss_end = .;
    }

    /* Stack (from boot.S). */
    .bss.stack : ALIGN(4K)
    {
        *(.bss.stack)
    }

    /DISCARD/ :
    {
        *(.eh_frame)
        *(.comment)
        *(.note*)
    }
}
