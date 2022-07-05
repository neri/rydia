use super::{gpio::*, mbox::*, raspi};
use crate::mem::mmio::*;
use core::fmt::Write;

pub struct Uart;

static mut UART: Uart = Uart {};

impl Uart {
    pub const CLOCK: u32 = 500_000_000;

    #[inline]
    pub fn shared<'a>() -> &'a mut Self {
        unsafe { &mut UART }
    }

    #[inline]
    pub const fn mu_baud(baud: u32) -> u32 {
        match Self::CLOCK.checked_div(baud * 8) {
            Some(v) => v - 1,
            None => 0,
        }
    }

    pub fn init() -> Result<(), ()> {
        // unsafe {
        //     Regs::ENABLE.write(1); //enable UART1, AUX mini uart
        //     Regs::MU_CNTL.write(0);
        //     Regs::MU_LCR.write(3); //8 bits
        //     Regs::MU_MCR.write(0);
        //     Regs::MU_IER.write(0);
        //     Regs::MU_IIR.write(0xC6); //disable interrupts

        //     let mut mbox = Mbox::PROP.mbox::<36>().ok_or(())?;
        //     mbox.append(Tag::SET_CLKRATE(2, Self::CLOCK, 0))?;
        //     mbox.call()?;

        //     Regs::MU_BAUD.write(Self::mu_baud(115200));
        //     // Regs::MU_BAUD.write(270);

        //     Gpio::UART0_TXD.use_as_alt5();
        //     Gpio::UART0_RXD.use_as_alt5();
        //     Gpio::enable(&[Gpio::UART0_TXD, Gpio::UART0_RXD]);

        //     Regs::MU_CNTL.write(3); //enable RX/TX

        // }

        Ok(())
    }

    #[inline]
    pub fn is_output_ready(&self) -> bool {
        (unsafe { Regs::MU_LSR.read() } & 0x20) != 0
    }

    #[inline]
    pub fn is_input_ready(&self) -> bool {
        (unsafe { Regs::MU_LSR.read() } & 0x01) != 0
    }

    pub fn write_byte(&self, ch: u8) {
        while !self.is_output_ready() {
            raspi::no_op();
        }
        unsafe {
            Regs::MU_IO.write(ch as u32);
        }
    }

    pub fn read_byte(&self) -> u8 {
        while !self.is_input_ready() {
            raspi::no_op();
        }
        unsafe { Regs::MU_IO.read() as u8 }
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
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
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
    pub fn base_addr() -> usize {
        raspi::mmio_base() + 0x21_5000
    }
}

unsafe impl Mmio32 for Regs {
    #[inline]
    fn addr(&self) -> usize {
        Self::base_addr() + *self as usize
    }
}
