use crate::{
    arch::{arm64::raspi::fb::Fb, cpu::Cpu},
    mem::{mmio, PhysicalAddress},
    system::System,
};
use core::{
    arch::asm,
    fmt::Write,
    intrinsics::transmute,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::{page::PageManager, spin::Spinlock};

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
        cbz     x1, 102f

        mov     x2, #0xd8
    100:
        ldr     x3, [x2, x1, lsl #3]
        cbnz    x3, 101f
        wfe
        b       100b
    101:
        lsl     x4, x1, #16
        add     x4, x4, #0x10000
        b       103f

    102:
        adr     x4, _start
    103:
        mov     sp, x4
        msr     sp_el1, x4

        mrs     x2, midr_el1
        mrs     x3, mpidr_el1
        msr     vpidr_el2, x2
        msr     vmpidr_el2, x3

        mov     x2, #3 << 20
        msr     cpacr_el1, x2

        mov     x2, #0x0002
        movk    x2, #0x8000, lsl #16
        msr     hcr_el2, x2
        adr     x3, 104f
        msr     elr_el2, x3
        mov     x4, #0x03C5
        msr     spsr_el2, x4
        eret
    104:

        mrs     x1, mpidr_el1
        and     x1, x1, #3
        cbz     x1, 2f

        bl      _smp_main
        b       5f

    2:
        ldr     x1, =__bss_start
        ldr     w2, =__bss_size
    3:  cbz     w2, 4f
        str     xzr, [x1], #8
        sub     w2, w2, #1
        cbnz    w2, 3b

    4:  bl      main
    5:
    ",
        options(noreturn)
    );
}

#[inline]
pub(super) fn _end() -> PhysicalAddress {
    let result: u64;
    unsafe {
        asm!("ldr {}, =_end", out(reg)result);
    }
    PhysicalAddress::new(result)
}

static SMP_TOKEN: AtomicUsize = AtomicUsize::new(1);
static SMP_BLOCK1: AtomicUsize = AtomicUsize::new(0);
static SMP_LOCK: Spinlock = Spinlock::new();
static SMP_TEST: AtomicUsize = AtomicUsize::new(0);

#[no_mangle]
unsafe fn _smp_main(_: usize, cpuid: usize) -> ! {
    while SMP_TOKEN.load(Ordering::Acquire) != cpuid {
        asm!("wfe");
    }

    let stdout = System::stdout();
    writeln!(stdout, "SMP: started core #{}", cpuid).unwrap();

    SMP_TOKEN.store(cpuid + 1, Ordering::Release);
    asm!("sev");

    PageManager::init_mp();

    while SMP_BLOCK1.load(Ordering::SeqCst) == 0 {
        asm!("nop");
    }

    SMP_LOCK.synchronized(|| {
        writeln!(stdout, "SPIN TEST: #{} OK", cpuid).unwrap();
    });

    SMP_TEST.fetch_or(1 << cpuid, Ordering::SeqCst);
    asm!("sev");

    loop {
        Cpu::wait_for_interrupt();
    }
}

unsafe fn _wake_smp() -> usize {
    let mut cpus = 1;
    for p in [0xE0, 0xE8, 0xF0] {
        let p = &*(p as *const AtomicUsize);
        p.store(_start as usize, Ordering::Release);
        asm!("sev");
        cpus += 1;
    }

    while SMP_TOKEN.load(Ordering::Acquire) != cpus {
        asm!("wfe");
    }

    cpus
}

fn _test_spin(val: &mut u64) -> (u32, u64) {
    let status: u32;
    let result: u64;
    unsafe {
        asm!("
        ldaxr {1}, [{2}]
        add {1}, {1}, #1 
        stlxr {0:w}, {1}, [{2}]
        ldar {3}, [{2}]
        ", out(reg) status, out(reg)_, in(reg)val, out(reg)result);
    };
    (status, result)
}

pub(super) unsafe fn init_early(dtb: usize) {
    // detect board
    let midr_el1: usize;
    asm!("mrs {}, midr_el1", out(reg) midr_el1);
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

    uart::Uart0::init().unwrap();

    crate::mem::MemoryManager::init_early(_end().rounding_up(0x1000), 0x40_0000);
    PageManager::init_early(dtb);

    let stdout = super::std_uart();
    writeln!(stdout, "\nStarting RasPi...").unwrap();

    let cpus = _wake_smp();
    writeln!(stdout, "Total {cpus} cores").unwrap();
    PageManager::init_mp();

    let mut test = 0x12345678;
    let (status, val) = _test_spin(&mut test);
    writeln!(stdout, "SPIN TEST: {} {:x} {:x}", status, val, test).unwrap();

    SMP_BLOCK1.store(1, Ordering::SeqCst);
    asm!("sev");

    while SMP_TEST.load(Ordering::SeqCst) != 0xE {
        asm!("wfe");
    }

    writeln!(stdout, "SPIN TEST: ALL OK",).unwrap();

    let (ptr, w, h, stride) = Fb::init(800, 600).unwrap();
    writeln!(stdout, "FB: {:012x} {} {} {}", ptr as usize, w, h, stride).unwrap();
}

#[inline]
pub(super) fn max_pa() -> PhysicalAddress {
    PhysicalAddress::new(0x1_0000_0000)
}

#[inline]
pub fn device_memlist<'a>() -> impl Iterator<Item = (PhysicalAddress, usize)> {
    let list = [(PhysicalAddress::from_usize(mmio_base()), 0x1_000_000)];
    list.into_iter()
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
fn mmio_base() -> usize {
    MMIO_BASE.load(Ordering::Relaxed)
}
