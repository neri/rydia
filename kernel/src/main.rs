#![no_std]
#![no_main]

use core::{arch::asm, fmt::Write};
use rydia::arch::uart::Uart;
use rydia::drawing::*;
use rydia::system::System;

#[no_mangle]
fn main() {
    unsafe {
        System::init();
    }

    let uart = Uart::shared();
    writeln!(uart, "hello, world!").unwrap();

    let emcon = System::em_console();
    writeln!(emcon, "Hello, Raspberry Pi!").unwrap();

    let mut bitmap = System::main_screen();
    bitmap.fill_circle(Point::new(200, 200), 100, Color::LIGHT_RED);
    bitmap.fill_circle(Point::new(300, 300), 100, Color::LIGHT_GREEN);
    bitmap.fill_circle(Point::new(400, 200), 100, Color::LIGHT_BLUE);

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
