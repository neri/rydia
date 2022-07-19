//! Architecture dependent module for RaspberryPi

#[macro_use]
pub mod cpu;
pub mod page;
mod raspi;
pub mod spin;

use self::page::PhysicalAddress;
use crate::io::uart::Uart;

#[inline]
pub unsafe fn init_early(dtb: usize) {
    raspi::init_early(dtb);
}

#[inline]
pub fn std_uart<'a>() -> &'a mut dyn Uart {
    raspi::uart::Uart0::shared() as &mut dyn Uart
}

#[inline]
pub fn max_pa() -> PhysicalAddress {
    raspi::max_pa()
}

#[inline]
pub fn device_memlist() -> impl Iterator<Item = (PhysicalAddress, usize)> {
    raspi::device_memlist()
}

#[inline]
pub fn fix_memlist(base: PhysicalAddress, size: usize) -> (PhysicalAddress, usize) {
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
