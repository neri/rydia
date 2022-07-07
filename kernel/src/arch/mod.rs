//! Architecture dependent module for RaspberryPi

use core::arch::asm;

pub mod fb;
pub mod gpio;
pub mod mbox;
pub mod timer;
pub mod uart;

#[no_mangle]
#[naked]
#[link_section = ".text.boot"]
unsafe extern "C" fn _start() {
    asm!(
        "
        mrs     x1, mpidr_el1
        and     x1, x1, #3
        cbz     x1, 2f
    1:  wfe
        b       1b
    2:

        adr     x1, _start
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
        options(noreturn)
    );
}

pub fn init() {
    raspi::init();
    uart::Uart::init().unwrap();
}

pub mod raspi {
    use core::{
        arch::asm,
        intrinsics::transmute,
        sync::atomic::{AtomicUsize, Ordering},
    };

    pub(super) fn init() {
        // detect board
        let midr_el1: usize;
        unsafe {
            asm!("mrs {}, midr_el1", out(reg) midr_el1);
        }
        CURRENT_MACHINE_TYPE.store(
            (match (midr_el1 >> 4) & 0xFFF {
                // 0xB76 => // rpi1
                // 0xC07 =>  // rpi2
                0xD03 => MachineType::RPi3,
                0xD08 => MachineType::RPi4,
                _ => MachineType::Unknown,
            }) as usize,
            Ordering::Relaxed,
        );

        MMIO_BASE.store(
            match current_machine_type() {
                MachineType::Unknown => 0x2000_0000,
                MachineType::RPi3 => 0x3F00_0000,
                MachineType::RPi4 => 0xFE00_0000,
            },
            Ordering::Relaxed,
        );
    }

    #[inline]
    pub fn current_machine_type() -> MachineType {
        unsafe { transmute(CURRENT_MACHINE_TYPE.load(Ordering::Relaxed)) }
    }

    static CURRENT_MACHINE_TYPE: AtomicUsize = AtomicUsize::new(0);

    #[repr(usize)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
    pub enum MachineType {
        #[default]
        Unknown,
        RPi3,
        RPi4,
    }

    static MMIO_BASE: AtomicUsize = AtomicUsize::new(0);

    #[inline]
    pub fn mmio_base() -> usize {
        MMIO_BASE.load(Ordering::Relaxed)
    }

    #[inline]
    pub fn no_op() {
        unsafe {
            asm!("nop");
        }
    }
}
