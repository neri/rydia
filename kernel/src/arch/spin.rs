use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

use super::cpu::Cpu;

#[derive(Default)]
pub struct Spinlock {
    value: AtomicUsize,
}

impl Spinlock {
    pub const LOCKED_VALUE: usize = 1;
    pub const UNLOCKED_VALUE: usize = 0;

    #[inline]
    pub const fn new() -> Self {
        Self {
            value: AtomicUsize::new(Self::UNLOCKED_VALUE),
        }
    }

    #[must_use]
    pub fn try_lock(&self) -> bool {
        self.value
            .compare_exchange_weak(
                Self::UNLOCKED_VALUE,
                Self::LOCKED_VALUE,
                Ordering::Acquire,
                Ordering::Relaxed,
            )
            .is_ok()
    }

    pub fn lock(&self) {
        if !self.try_lock() {
            todo!()
            // while self
            //     .value
            //     .compare_exchange_weak(
            //         Self::UNLOCKED_VALUE,
            //         Self::LOCKED_VALUE,
            //         Ordering::Acquire,
            //         Ordering::Relaxed,
            //     )
            //     .is_err()
            // {
            //     unsafe {
            //         asm!("wfe");
            //     }
            // }
        }
    }

    #[inline]
    pub unsafe fn force_unlock(&self) {
        self.value.store(Self::UNLOCKED_VALUE, Ordering::SeqCst);
        asm!("sev");
    }

    #[inline]
    pub fn synchronized<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        self.lock();
        let result = f();
        unsafe {
            self.force_unlock();
        }
        result
    }
}

#[derive(Debug, Default)]
pub struct SpinLoopWait;

impl SpinLoopWait {
    #[inline]
    pub const fn new() -> Self {
        Self {}
    }

    #[inline]
    pub fn reset(&mut self) {}

    pub fn wait(&mut self) {
        Cpu::spin_loop_hint();
    }
}
