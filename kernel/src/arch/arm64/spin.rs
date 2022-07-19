use super::cpu::Cpu;
use core::{
    arch::asm,
    sync::atomic::{AtomicUsize, Ordering},
};

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
        let result: u32;
        unsafe {
            asm!("
            ldaxr {0:w}, [{1}]
            cbnz {0:w}, 1f
            stxr {0:w}, {2:w}, [{1}]
            1:
            ", out(reg)result, in(reg)&self.value, in(reg)1);
        }
        result == 0
    }

    pub fn lock(&self) {
        unsafe {
            asm!(
                "
                sevl
                1: wfe
                2: ldaxr {0:w}, [{1}]
                cbnz {0:w}, 1b
                stxr {0:w}, {2:w}, [{1}]
                cbnz {0:w}, 2b
                ", out(reg)_, in(reg)&self.value, in(reg)1);
        }
    }

    #[inline]
    pub unsafe fn force_unlock(&self) {
        self.value.store(Self::UNLOCKED_VALUE, Ordering::Release);
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
