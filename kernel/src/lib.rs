#![no_std]
#![feature(const_trait_impl)]

pub mod raspi;

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
