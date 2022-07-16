//! Architecture dependent module for RaspberryPi

#[macro_use]
pub mod cpu;
pub mod fb;
pub mod gpio;
pub mod mbox;
pub mod page;
pub(super) mod raspi;
pub mod spin;
pub mod timer;
pub mod uart;

use self::page::PhysicalAddress;

pub(crate) fn fix_memlist(base: PhysicalAddress, size: usize) -> (PhysicalAddress, usize) {
    let page_size = 0x1000 as u64;
    let page_mask = page_size - 1;
    let frame_mask = !page_mask;
    let end = (raspi::_end() + page_mask) & frame_mask;
    let area_end = base + size;
    if base >= end {
        (base, size)
    } else {
        if area_end < end {
            (PhysicalAddress::NULL, 0)
        } else {
            let diff = end - base;
            (base + diff, size - diff)
        }
    }
}

pub(crate) fn init_minimal() {
    raspi::init();
}
