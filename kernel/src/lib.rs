#![no_std]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(const_trait_impl)]
#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(negative_impls)]

#[macro_use]
pub mod arch;
pub mod fw;
pub mod io;
pub mod mem;
pub mod sync;
pub mod system;
pub use meggl as drawing;
extern crate alloc;

use arch::uart::Uart0;
use core::fmt::Write;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let uart = Uart0::shared();
    let _ = writeln!(uart, "!!! PANIC: {}", info);
    loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
