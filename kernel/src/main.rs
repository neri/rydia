#![no_std]
#![no_main]

use core::{arch::asm, fmt::Write};
use rydia::arch::{self, uart::Uart};
use rydia::drawing::*;

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

#[no_mangle]
fn main() {
    arch::init();

    let uart = Uart::shared();
    // writeln!(uart, "hello, world!").unwrap();
    let _ = uart.write_str("hello, world!\n");

    let mut bitmap = arch::fb::Fb::init(800, 600).unwrap();

    bitmap.fill_circle(Point::new(200, 200), 100, Color::LIGHT_RED.into());
    bitmap.fill_circle(Point::new(300, 300), 100, Color::LIGHT_GREEN.into());
    bitmap.fill_circle(Point::new(400, 200), 100, Color::LIGHT_BLUE.into());

    loop {
        unsafe {
            asm!("wfe");
        }
        if uart.is_input_ready() {
            let data = uart.read_byte();
            if data == '\r' as u8 {
                uart.write_byte('\n' as u8);
            }
            uart.write_byte(data);
        }
    }
}
