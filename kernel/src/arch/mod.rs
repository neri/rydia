//! Architecture dependent module for RaspberryPi

use core::arch::asm;

pub mod fb;
pub mod gpio;
pub mod mbox;
pub mod uart;

#[no_mangle]
#[link_section = ".text.boot"]
unsafe fn _start() {
    asm!(
        "
        mrs     x1, mpidr_el1
        and     x1, x1, #3
        cbz     x1, 2f
    1:  wfe
        b       1b
    2:

        ldr     x1, =_start
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
        options(nomem, nostack)
    );
}

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
