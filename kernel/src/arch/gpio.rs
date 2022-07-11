use crate::mem::mmio::*;

use super::{cpu::Cpu, raspi};

#[derive(Debug, Clone, Copy)]
pub enum Gpio {
    Pin00 = 0,
    Pin01,
    Pin02,
    Pin03,
    Pin04,
    Pin05,
    Pin06,
    Pin07,
    Pin08,
    Pin09,
    Pin10,
    Pin11,
    Pin12,
    Pin13,
    Pin14,
    Pin15,
    Pin16,
    Pin17,
    Pin18,
    Pin19,
    Pin20,
    Pin21,
    Pin22,
    Pin23,
    Pin24,
    Pin25,
    Pin26,
    Pin27,
    Pin28,
    Pin29,
    Pin30,
    Pin31,
    Pin32,
    Pin33,
    Pin34,
    Pin35,
    Pin36,
    Pin37,
    Pin38,
    Pin39,
    Pin40,
    Pin41,
    Pin42,
    Pin43,
    Pin44,
    Pin45,
    Pin46,
    Pin47,
    Pin48,
    Pin49,
    Pin50,
    Pin51,
    Pin52,
    Pin53,
}

impl Gpio {
    pub const FUNCTION_OUT: u32 = 1;
    pub const FUNCTION_ALT5: u32 = 2;
    pub const FUNCTION_ALT3: u32 = 7;
    pub const FUNCTION_ALT0: u32 = 4;

    pub const PULL_NONE: u32 = 0;
    pub const PULL_DOWN: u32 = 1;
    pub const PULL_UP: u32 = 2;

    pub const SDA1: Self = Self::Pin02;
    pub const SCL1: Self = Self::Pin03;

    pub const SPI0_CE1_N: Self = Self::Pin07;
    pub const SPI0_CE0_N: Self = Self::Pin08;
    pub const SPI0_MISO: Self = Self::Pin09;
    pub const SPI0_MOSI: Self = Self::Pin10;
    pub const SPI0_SCLK: Self = Self::Pin11;

    pub const UART0_TXD: Self = Self::Pin14;
    pub const UART0_RXD: Self = Self::Pin15;

    #[inline]
    pub fn set(&self, value: u32) {
        unsafe { Regs::GPSET0._gpio_call(*self, value, 1) }
    }

    #[inline]
    pub fn clear(&self, value: u32) {
        unsafe { Regs::GPCLR0._gpio_call(*self, value, 1) }
    }

    #[inline]
    pub fn pull(&self, value: u32) {
        unsafe { Regs::GPPUPPDN0._gpio_call(*self, value, 2) }
    }

    #[inline]
    pub fn function(&self, value: u32) {
        unsafe { Regs::GPFSEL0._gpio_call(*self, value, 3) }
    }

    #[inline]
    pub fn use_as_alt5(&self) {
        self.pull(Gpio::PULL_NONE);
        self.function(Gpio::FUNCTION_ALT5);
    }

    #[inline]
    pub fn use_as_alt3(&self) {
        self.pull(Gpio::PULL_NONE);
        self.function(Gpio::FUNCTION_ALT3);
    }

    #[inline]
    pub fn use_as_alt0(&self) {
        self.pull(Gpio::PULL_NONE);
        self.function(Gpio::FUNCTION_ALT0);
    }

    #[inline]
    pub fn init_output_pin_with_pull_none(&self) {
        self.pull(Gpio::PULL_NONE);
        self.function(Gpio::FUNCTION_OUT);
    }

    #[inline]
    pub fn set_output(&self, val: bool) {
        if val {
            self.set(1);
        } else {
            self.clear(1);
        }
    }

    #[inline]
    pub fn enable(pins: &[Self]) {
        let acc = pins.iter().fold(0, |a, v| a | (1 << *v as usize));
        unsafe {
            Regs::GPPUD.as_reg().write(0);
            for _ in 0..150 {
                Cpu::no_op();
            }
            Regs::GPPUDCLK0.as_reg().write(acc);
            for _ in 0..150 {
                Cpu::no_op();
            }
            Regs::GPPUDCLK0.as_reg().write(0);
        }
    }
}

#[repr(usize)]
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum Regs {
    GPFSEL0 = 0x0000,
    GPFSEL1 = 0x0004,
    GPFSEL2 = 0x0008,
    GPFSEL3 = 0x000C,
    GPFSEL4 = 0x0010,
    GPFSEL5 = 0x0014,
    GPSET0 = 0x001C,
    GPSET1 = 0x0020,
    GPCLR0 = 0x0028,
    GPLEV0 = 0x0034,
    GPLEV1 = 0x0038,
    GPEDS0 = 0x0040,
    GPEDS1 = 0x0044,
    GPREN0 = 0x004C,
    GPREN1 = 0x0050,
    GPFEN0 = 0x0058,
    GPFEN1 = 0x005C,
    GPHEN0 = 0x0064,
    GPHEN1 = 0x0068,
    GPLEN0 = 0x0070,
    GPLEN1 = 0x0074,
    GPPUD = 0x0094,
    GPPUDCLK0 = 0x0098,
    GPPUDCLK1 = 0x009C,
    GPPUPPDN0 = 0x00E4,
}

impl Regs {
    #[inline]
    unsafe fn base_addr(&self) -> usize {
        raspi::mmio_base() + 0x0020_0000 + *self as usize
    }

    #[inline]
    unsafe fn as_reg(&self) -> Mmio32Reg {
        Mmio32Reg(self.base_addr())
    }

    #[must_use]
    unsafe fn _gpio_call(&self, pin: Gpio, value: u32, field_size: usize) {
        let pin_number = pin as usize;
        let field_mask = (1 << field_size) - 1;
        let num_fields = 32 / field_size;
        let reg = Mmio32Reg(self.base_addr() + ((pin_number / num_fields) * 4));
        let shift = (pin_number % num_fields) * field_size;

        let mut curval = reg.read();
        curval &= !(field_mask << shift);
        curval |= value << shift;
        reg.write(curval);
    }
}
