#![no_std]
#![no_main]

use alloc::vec::Vec;
use core::fmt::Write;
use rydia::mem::MemoryManager;
use rydia::system::System;
use rydia::{drawing::*, system};

extern crate alloc;

#[no_mangle]
fn main(dtb: usize) -> ! {
    unsafe {
        System::init(dtb);
    }

    let stdout = System::stdout();
    writeln!(
        stdout,
        "{} v{} [codename {}]",
        System::name(),
        System::version(),
        System::codename()
    )
    .unwrap();
    writeln!(stdout, "model: {}", System::model_name().unwrap(),).unwrap();

    let emcon = System::em_console();
    writeln!(
        emcon,
        "{} v{} [codename {}]",
        System::name(),
        System::version(),
        System::codename()
    )
    .unwrap();
    writeln!(emcon, "model: {}", System::model_name().unwrap(),).unwrap();
    writeln!(
        emcon,
        "memory: {} KB",
        MemoryManager::free_memory_size() >> 10
    )
    .unwrap();

    let mut bitmap = System::main_screen().unwrap();
    bitmap.fill_circle(Point::new(200, 200), 100, Color::LIGHT_RED);
    bitmap.fill_circle(Point::new(300, 300), 100, Color::LIGHT_GREEN);
    bitmap.fill_circle(Point::new(400, 200), 100, Color::LIGHT_BLUE);

    writeln!(
        stdout,
        "free memory: {} KB",
        MemoryManager::free_memory_size() >> 10
    )
    .unwrap();
    let mut foo = Vec::<u32>::new();
    writeln!(stdout, "empty: {:08x}", foo.as_ptr() as usize).unwrap();
    foo.resize(1, 0);
    writeln!(stdout, "base1: {:08x}", foo.as_ptr() as usize).unwrap();
    foo.resize(0x100_0000, 0);
    writeln!(stdout, "base2: {:08x}", foo.as_ptr() as usize).unwrap();
    core::mem::forget(foo);
    writeln!(
        stdout,
        "free memory: {} KB",
        MemoryManager::free_memory_size() >> 10
    )
    .unwrap();

    loop {
        if stdout.is_input_ready() {
            let data = stdout.read_byte();
            if data == '\r' as u8 {
                stdout.write_byte('\n' as u8);
            }
            stdout.write_byte(data);
        }
    }
}
