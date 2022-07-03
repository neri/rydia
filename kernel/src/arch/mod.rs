//! Architecture dependent module for RaspberryPi

pub mod fb;
pub mod gpio;
pub mod mbox;
pub mod uart;

pub fn init() {
    uart::Uart::init();
}

pub mod raspi {
    use core::arch::asm;

    pub fn mmio_base() -> usize {
        // raspi3
        0x3F00_0000
        // raspi4
        // 0xFE00_0000
    }

    #[inline]
    pub fn no_op() {
        unsafe {
            asm!("nop");
        }
    }
}
