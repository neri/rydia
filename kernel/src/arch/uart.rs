use super::{gpio::*, mbox::*, raspi};
use crate::mem::mmio::*;
use core::fmt::Write;

pub struct Uart;

static mut UART: Uart = Uart {};
// static mut UART0: Uart0 = Uart0::CR;

impl Uart {
    pub const CLOCK: u32 = 500_000_000;

    #[inline]
    pub fn shared<'a>() -> &'a mut Uart {
        unsafe { &mut UART }
    }

    #[inline]
    pub const fn baud(baud: u32) -> u32 {
        match Self::CLOCK.checked_div(baud * 8) {
            Some(v) => v - 1,
            None => 0,
        }
    }

    pub fn init() -> Result<(), ()> {
        unsafe {
            Gpio::UART0_TXD.use_as_alt5();
            Gpio::UART0_RXD.use_as_alt5();
            Gpio::enable(&[Gpio::UART0_TXD, Gpio::UART0_RXD]);

            Uart1::ENABLE.write(1); //enable UART1, AUX mini uart
            Uart1::CNTL.write(0);
            Uart1::LCR.write(3); //8 bits
            Uart1::MCR.write(0);
            Uart1::IER.write(0);
            Uart1::IIR.write(0xC6); //disable interrupts

            match raspi::current_machine_type() {
                raspi::MachineType::Unknown => {
                    // TODO:
                }
                raspi::MachineType::RPi3 => {
                    Uart1::BAUD.write(270);
                }
                raspi::MachineType::RPi4 => {
                    Uart1::BAUD.write(Self::baud(115200));
                }
            }

            Uart1::CNTL.write(3); //enable RX/TX
        }

        Ok(())
    }

    #[inline]
    pub fn is_output_ready(&self) -> bool {
        (unsafe { Uart1::LSR.read() } & 0x20) != 0
    }

    #[inline]
    pub fn is_input_ready(&self) -> bool {
        (unsafe { Uart1::LSR.read() } & 0x01) != 0
    }

    pub fn write_byte(&self, ch: u8) {
        while !self.is_output_ready() {
            raspi::no_op();
        }
        unsafe {
            Uart1::IO.write(ch as u32);
        }
    }

    pub fn read_byte(&self) -> u8 {
        while !self.is_input_ready() {
            raspi::no_op();
        }
        unsafe { Uart1::IO.read() as u8 }
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

/// Uart 0 (PL011)
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Uart0 {
    DR = 0x00,
    RSRECR = 0x04,
    FR = 0x18,
    ILPR = 0x20,
    IBRD = 0x24,
    FBRD = 0x28,
    LCRH = 0x2C,
    CR = 0x30,
    IFLS = 0x34,
    IMSC = 0x38,
    RIS = 0x3C,
    MIS = 0x40,
    ICR = 0x44,
    DMACR = 0x48,
    ITCR = 0x80,
    ITIP = 0x84,
    ITOP = 0x88,
    TDR = 0x8C,
}

unsafe impl Mmio32 for Uart0 {
    #[inline]
    fn addr(&self) -> usize {
        raspi::mmio_base() + 0x20_1000 + *self as usize
    }
}

#[allow(dead_code)]
impl Uart0 {
    pub fn init() -> Result<(), ()> {
        unsafe {
            // Disable UART0.
            Uart0::CR.write(0);

            Gpio::UART0_TXD.use_as_alt0();
            Gpio::UART0_RXD.use_as_alt0();
            Gpio::enable(&[Gpio::UART0_TXD, Gpio::UART0_RXD]);

            // Clear pending interrupts.
            Uart0::ICR.write(0x7FF);

            let mut mbox = Mbox::PROP.mbox::<10>().ok_or(())?;
            mbox.append(Tag::SET_CLKRATE(2, 3000000, 0))?;
            mbox.call()?;

            // Divider = 3000000 / (16 * 115200) = 1.627 = ~1.
            Uart0::IBRD.write(1);
            // Fractional part register = (.627 * 64) + 0.5 = 40.6 = ~40.
            Uart0::FBRD.write(40);

            // Enable FIFO & 8 bit data transmission (1 stop bit, no parity).
            Uart0::LCRH.write(0x0070);

            // Mask all interrupts.
            Uart0::IMSC.write(0x7F2);

            // Enable UART0, receive & transfer part of UART.
            Uart0::CR.write(0x301);
        }
        Ok(())
    }

    #[inline]
    pub fn is_output_ready(&self) -> bool {
        unsafe { (Uart0::FR.read() & 0x20) == 0 }
    }

    #[inline]
    pub fn is_input_ready(&self) -> bool {
        unsafe { (Uart0::FR.read() & 0x10) == 0 }
    }

    pub fn write_byte(&self, ch: u8) {
        while !self.is_output_ready() {
            raspi::no_op();
        }
        unsafe {
            Uart0::DR.write(ch as u32);
        }
    }

    pub fn read_byte(&self) -> u8 {
        while !self.is_input_ready() {
            raspi::no_op();
        }
        unsafe { Uart0::DR.read() as u8 }
    }
}

impl Write for Uart0 {
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

/// Mini UART
#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
enum Uart1 {
    ENABLE = 0x0004,
    IO = 0x0040,
    IER = 0x0044,
    IIR = 0x0048,
    LCR = 0x004C,
    MCR = 0x0050,
    LSR = 0x0054,
    MSR = 0x0058,
    SCRATCH = 0x005C,
    CNTL = 0x0060,
    STAT = 0x0064,
    BAUD = 0x0068,
}

unsafe impl Mmio32 for Uart1 {
    #[inline]
    fn addr(&self) -> usize {
        raspi::mmio_base() + 0x21_5000 + *self as usize
    }
}
