use super::{gpio::*, *};
use core::{arch::asm, fmt::Write};

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
enum Regs {
    ENABLE = 0x0004,
    MU_IO = 0x0040,
    MU_IER = 0x0044,
    MU_IIR = 0x0048,
    MU_LCR = 0x004C,
    MU_MCR = 0x0050,
    MU_LSR = 0x0054,
    MU_MSR = 0x0058,
    MU_SCRATCH = 0x005C,
    MU_CNTL = 0x0060,
    MU_STAT = 0x0064,
    MU_BAUD = 0x0068,
}

impl Regs {
    #[inline]
    pub fn base() -> usize {
        Gpio::base() + 0x21_5000
    }
}

unsafe impl Mmio for Regs {
    #[inline]
    fn addr(&self) -> usize {
        Self::base() + *self as usize
    }
}

pub struct Uart;

impl Uart {
    pub const CLOCK: usize = 500_000_000;
    pub const MAX_QUEUE: usize = 16 * 1024;

    #[inline]
    pub const fn shared<'a>() -> Self {
        Self {}
    }

    #[inline]
    pub const fn mu_baud(baud: usize) -> u32 {
        match Self::CLOCK.checked_div(baud * 8) {
            Some(v) => (v - 1) as u32,
            None => 0,
        }
    }

    pub fn init() {
        unsafe {
            Regs::ENABLE.write(Regs::ENABLE.read() | 1); //enable UART1, AUX mini uart
            Regs::MU_CNTL.write(0);
            Regs::MU_LCR.write(3); //8 bits
            Regs::MU_MCR.write(0);
            Regs::MU_IER.write(0);
            Regs::MU_IIR.write(0xC6); //disable interrupts

            // Regs::MU_BAUD.write(Self::mu_baud(115200));
            Regs::MU_BAUD.write(270);

            Gpio::Pin14.use_as_alt5();
            Gpio::Pin15.use_as_alt5();
            Gpio::enable(&[Gpio::Pin14, Gpio::Pin15]);

            Regs::MU_CNTL.write(3); //enable RX/TX
        }
    }

    #[inline]
    pub fn is_send_ready() -> bool {
        (unsafe { Regs::MU_LSR.read() } & 0x20) != 0
    }

    pub fn write_byte(&self, ch: u8) {
        while !Self::is_send_ready() {
            unsafe {
                asm!("nop");
            }
        }
        unsafe {
            Regs::MU_IO.write(ch as u32);
        }
    }
}

impl Write for Uart {
    #[inline]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.chars() {
            if ch == '\n' {
                self.write_byte('\r' as u8);
            }
            self.write_byte(ch as u8);
        }
        Ok(())
    }

    #[inline]
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.write_byte(c as u8);
        Ok(())
    }
}
