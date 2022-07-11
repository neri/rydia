//! Architecture dependent module for RaspberryPi

use self::{page::PhysicalAddress, spin::Spinlock};
use core::{
    arch::asm,
    fmt::Write,
    sync::atomic::{AtomicUsize, Ordering},
};

#[macro_use]
pub mod cpu;
pub mod fb;
pub mod gpio;
pub mod mbox;
pub mod page;
pub mod spin;
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

        lsl     x2, x1, #16
        mov     sp, x2

        bl      _smp_main

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

#[inline]
fn _end() -> PhysicalAddress {
    let result: u64;
    unsafe {
        asm!("ldr {}, =_end", out(reg)result);
    }
    PhysicalAddress::new(result)
}

static SMP_TOKEN: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
unsafe fn _smp_main(_dtb: usize, cpuid: usize) {
    let current_token = cpuid - 1;
    while SMP_TOKEN.load(Ordering::Acquire) != current_token {
        asm!("wfe");
    }

    let stdout = uart::Uart0::shared();
    writeln!(stdout, "SMP: started core #{}", cpuid).unwrap();

    SMP_TOKEN.store(cpuid, Ordering::Release);
    asm!("sev");

    loop {
        asm!("wfi");
    }
}

pub(crate) fn fix_memlist(base: PhysicalAddress, size: usize) -> (PhysicalAddress, usize) {
    let page_size = 0x1000 as u64;
    let page_mask = page_size - 1;
    let frame_mask = !page_mask;
    let end = (_end() + page_mask) & frame_mask;
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

fn _test_load(val: &mut u64) -> (u32, u64, usize) {
    let status: u32;
    let result: u64;
    let ptr = unsafe {
        asm!("
        ldaxr {1}, [{2}]
        add {1}, {1}, #1 
        stlxr {0:w}, {1}, [{2}]
        ldar {3}, [{2}]
        ", out(reg) status, out(reg)_, in(reg)val, out(reg)result);
        val as *const _ as usize
    };
    (status, result, ptr)
}

pub(crate) fn init_minimal() {
    raspi::init();
    uart::Uart0::init().unwrap();

    let stdout = uart::Uart0::shared();
    writeln!(stdout, "\nStarting RasPi...").unwrap();

    let mut cpus = 0;
    for p in [0xE0, 0xE8, 0xF0] {
        unsafe {
            cpus += 1;
            let p = &*(p as *const AtomicUsize);
            let f = _start as usize;
            p.store(f, Ordering::SeqCst);
            asm!("sev");
        }
    }

    while SMP_TOKEN.load(Ordering::Acquire) != cpus {
        unsafe {
            asm!("wfe");
        }
    }

    cpus += 1;
    writeln!(stdout, "Total {cpus} cores initialize OK").unwrap();

    let mut test = 0x12345678;
    let (status, val, ptr) = _test_load(&mut test);
    writeln!(
        stdout,
        "TEST VALUE: {} {:x} {:x} @{:012x}\n",
        status, val, test, ptr
    )
    .unwrap();
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
}
