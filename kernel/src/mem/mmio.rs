use core::sync::atomic::{AtomicU32, Ordering};

pub unsafe trait Mmio {
    fn addr(&self) -> usize;

    #[inline]
    unsafe fn write(&self, val: u32) {
        let p = self.addr() as *const AtomicU32;
        (&*p).store(val, Ordering::SeqCst);
    }

    #[inline]
    unsafe fn read(&self) -> u32 {
        let p = self.addr() as *const AtomicU32;
        (&*p).load(Ordering::SeqCst)
    }
}

#[repr(transparent)]
pub struct MmioReg(pub usize);

unsafe impl Mmio for MmioReg {
    #[inline]
    fn addr(&self) -> usize {
        self.0
    }
}
