#![no_std]
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
#![feature(const_swap)]
#![feature(lang_items)]
#![feature(alloc_error_handler)]

pub mod arch;
pub mod drawing;
pub mod mem;

extern crate alloc;

use arch::uart::Uart;
use core::fmt::Write;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let uart = Uart::shared();
    let _ = writeln!(uart, "!!! PANIC: {}", info);
    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
