#![no_std]
#![no_main]

use alloc::vec::Vec;
use core::fmt::Write;
use core::slice;
use rydia::arch::uart;
use rydia::drawing::*;
use rydia::fw::dt::{PropName, Token};
use rydia::mem::MemoryManager;
use rydia::system::System;

extern crate alloc;

#[no_mangle]
fn main(dtb: usize) -> ! {
    unsafe {
        System::init(dtb);
    }

    let stdout = uart::Uart0::shared();
    writeln!(stdout, "hello, world!").unwrap();
    writeln!(stdout, "model: {}", System::model_name().unwrap()).unwrap();

    let emcon = System::em_console();
    writeln!(emcon, "Hello, world!").unwrap();
    writeln!(emcon, "model: {}", System::model_name().unwrap()).unwrap();
    writeln!(
        emcon,
        "memory: {} KB",
        MemoryManager::free_memory_size() >> 10
    )
    .unwrap();

    let dt = System::device_tree().unwrap();
    if false {
        let mut indent = 0;
        for token in dt.header().tokens().into_iter().take(200) {
            for _ in 0..indent {
                write!(stdout, "  ").unwrap();
            }
            match token {
                Token::BeginNode(name) => {
                    writeln!(stdout, "{} {{", name.0).unwrap();
                    indent += 1;
                }
                Token::EndNode => {
                    writeln!(stdout, "}}").unwrap();
                    indent -= 1;
                }
                Token::Prop(name, ptr, len) => match name {
                    PropName::ADDRESS_CELLS | PropName::SIZE_CELLS | PropName::PHANDLE => {
                        let data = unsafe { (ptr as *const u32).read_volatile() }.to_be();
                        writeln!(stdout, "{} = <0x{:08x}> ({})", name.0, data, data).unwrap();
                    }
                    PropName::REG | PropName::RANGES => {
                        let slice = unsafe { slice::from_raw_parts(ptr as *const u32, len / 4) };
                        write!(stdout, "{} = <", name.0).unwrap();
                        for data in slice {
                            write!(stdout, "0x{:08x},", data.to_be()).unwrap();
                        }
                        writeln!(stdout, ">").unwrap();
                    }
                    _ => {
                        let slice = unsafe { slice::from_raw_parts(ptr as *const u8, len) };
                        write!(stdout, "{} = \"", name.0).unwrap();
                        for ch in slice {
                            let ch = *ch;
                            if ch >= 0x20 && ch < 0x80 {
                                write!(stdout, "{}", ch as char).unwrap();
                            } else {
                                write!(stdout, "\\{:02x}", ch).unwrap();
                            }
                        }
                        writeln!(stdout, "\"").unwrap();
                    }
                },
            }
        }
    }

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
    foo.resize(1_000_000, 0);
    writeln!(stdout, "base2: {:08x}", foo.as_ptr() as usize).unwrap();
    core::mem::forget(foo);
    writeln!(
        stdout,
        "free memory: {} KB",
        MemoryManager::free_memory_size() >> 10
    )
    .unwrap();

    let mut bitmap = System::main_screen();
    bitmap.fill_circle(Point::new(200, 200), 100, Color::LIGHT_RED);
    bitmap.fill_circle(Point::new(300, 300), 100, Color::LIGHT_GREEN);
    bitmap.fill_circle(Point::new(400, 200), 100, Color::LIGHT_BLUE);

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
