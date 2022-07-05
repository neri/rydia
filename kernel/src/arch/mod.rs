//! Architecture dependent module for RaspberryPi

use core::arch::asm;

pub mod fb;
pub mod gpio;
pub mod mbox;
pub mod uart;

#[no_mangle]
#[naked]
#[link_section = ".text.boot"]
unsafe extern "C" fn _start() {
    asm!(
        "
        mrs     x1, mpidr_el1
        and     x1, x1, #3
        cbz     x1, 2f
    1:  wfe
        b       1b
    2:

        adr     x1, _start
        mov     sp, x1

        ldr     x1, =__bss_start
        ldr     w2, =__bss_size
    3:  cbz     w2, 4f
        str     xzr, [x1], #8
        sub     w2, w2, #1
        cbnz    w2, 3b

    4:  bl      main
        b       1b
    ",
        options(noreturn)
    );
}

pub fn init() {
    raspi::init();
    uart::Uart::init().unwrap();
}

pub mod raspi {
    use core::{
        arch::asm,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use crate::system::System;

    pub(super) fn init() {
        let model_name = System::model_name().unwrap();
        if model_name.starts_with("Raspberry Pi 3") {
            MMIO_BASE.store(0x3F00_0000, Ordering::Relaxed);
        } else if model_name.starts_with("Raspberry Pi 4") {
            MMIO_BASE.store(0xFE00_0000, Ordering::Relaxed);
        }
    }

    static MMIO_BASE: AtomicUsize = AtomicUsize::new(0);

    #[inline]
    pub fn mmio_base() -> usize {
        MMIO_BASE.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn no_op() {
        unsafe {
            asm!("nop");
        }
    }
}
