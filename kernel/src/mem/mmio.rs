use core::sync::atomic::{AtomicU32, Ordering};

pub unsafe trait Mmio32 {
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
pub struct Mmio32Reg(pub usize);

unsafe impl Mmio32 for Mmio32Reg {
    #[inline]
    fn addr(&self) -> usize {
        self.0
    }
}
