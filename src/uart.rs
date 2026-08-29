//! Minimal MMIO UART driver for the ns16550a UART in QEMU virt.
//!
//! The QEMU riscv64 virt machine exposes the UART at 0x10000000.
//! Register layout (16550-compatible):
//!   RBR/THR (offset 0) — receive/transmit buffer
//!   IER     (offset 1) — interrupt enable
//!   LCR     (offset 3) — line control (DLAB bit)
//!   LSR     (offset 5) — line status

use core::fmt;

/// QEMU virt UART base address (ns16550a).
const UART_BASE: usize = 0x10000000;

/// Write a single byte to a UART register.
///
/// # Safety
/// `addr` must be a valid MMIO address for the UART.
#[inline(always)]
unsafe fn mmio_write(addr: usize, val: u8) {
    unsafe {
        core::ptr::write_volatile(addr as *mut u8, val);
    }
}

/// Read a single byte from a UART register.
///
/// # Safety
/// `addr` must be a valid MMIO address for the UART.
#[inline(always)]
unsafe fn mmio_read(addr: usize) -> u8 {
    unsafe { core::ptr::read_volatile(addr as *const u8) }
}

/// Initialise the UART: disable interrupts, enable FIFO, set baud rate.
pub fn uart_init() {
    let base = UART_BASE;
    unsafe {
        mmio_write(base + 1, 0x00); // Disable interrupts
        mmio_write(base + 3, 0x80); // Enable DLAB (set baud rate divisor)
        mmio_write(base, 0x03); // Set divisor lo byte (38400 baud)
        mmio_write(base + 1, 0x00); // Set divisor hi byte
        mmio_write(base + 3, 0x03); // 8 bits, no parity, one stop bit
        mmio_write(base + 2, 0xC7); // Enable FIFO, clear them, 14-byte threshold
        mmio_write(base + 4, 0x0B); // IRQs enabled, RTS/DSR set
    }
}

/// Write a single byte to the UART, busy-waiting for the transmit buffer.
pub fn uart_putc(c: u8) {
    let base = UART_BASE;
    unsafe {
        // Wait until the transmit holding register is empty.
        while (mmio_read(base + 5) & 0x20) == 0 {}
        mmio_write(base, c);
    }
}

/// Write a byte string to the UART.
pub fn uart_puts(s: &str) {
    for byte in s.bytes() {
        // Convert \n to \r\n for proper terminal output.
        if byte == b'\n' {
            uart_putc(b'\r');
        }
        uart_putc(byte);
    }
}

/// Implement `fmt::Write` so we can use `write!()` / `writeln!()` macros.
struct UartWriter;

impl fmt::Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        uart_puts(s);
        Ok(())
    }
}

/// Print formatted output to the UART (macro-compatible).
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    UartWriter.write_fmt(args).unwrap();
}

/// Print macro: `print!("hello {}", 42);`
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::uart::_print(format_args!($($arg)*))
    };
}

/// Print macro with newline: `println!("hello {}", 42);` or `println!();`
#[macro_export]
macro_rules! println {
    () => {
        $crate::uart::_print(format_args!("\n"));
    };
    ($($arg:tt)*) => {
        $crate::uart::_print(format_args!($($arg)*));
        $crate::uart::_print(format_args!("\n"));
    };
}
