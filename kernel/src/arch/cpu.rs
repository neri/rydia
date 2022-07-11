use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

pub struct Cpu {}

impl Cpu {
    #[inline]
    pub fn no_op() {
        unsafe {
            asm!("nop", options(nomem, nostack));
        }
    }

    #[inline]
    pub fn spin_loop_hint() {
        unsafe {
            asm!("nop");
        }
    }

    #[inline]
    pub fn wait_for_interrupt() {
        unsafe {
            asm!("wfi");
        }
    }

    #[inline]
    pub unsafe fn enable_interrupt() {
        // TODO:
    }

    #[inline]
    pub unsafe fn disable_interrupt() {
        // TODO:
    }

    #[inline]
    pub unsafe fn interrupt_guard() -> InterruptGuard {
        InterruptGuard(0)
    }

    #[inline]
    pub fn interlocked_test_and_set(p: &AtomicUsize, position: usize) -> bool {
        let test = 1 << position;
        let mut result = false;
        let _ = p.fetch_update(Ordering::SeqCst, Ordering::Relaxed, |data| {
            result = (data & test) != 0;
            Some(data & test)
        });
        result
    }

    #[inline]
    pub fn interlocked_test_and_clear(p: &AtomicUsize, position: usize) -> bool {
        let test = 1 << position;
        let pattern = !test;
        let mut result = false;
        let _ = p.fetch_update(Ordering::SeqCst, Ordering::Relaxed, |data| {
            result = (data & test) != 0;
            Some(data & !pattern)
        });
        result
    }
}

#[must_use]
pub struct InterruptGuard(usize);

impl !Send for InterruptGuard {}

impl !Sync for InterruptGuard {}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe {
                Cpu::enable_interrupt();
            }
        }
    }
}

#[macro_export]
macro_rules! without_interrupts {
    ( $f:expr ) => {{
        let rflags = Cpu::interrupt_guard();
        let r = { $f };
        drop(rflags);
        r
    }};
}
