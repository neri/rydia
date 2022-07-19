use crate::mem::mmio::Mmio32;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum SystemTimer {
    CS = 0x00,
    Lo = 0x04,
    Hi = 0x08,
    C0 = 0x0C,
    C1 = 0x10,
    C2 = 0x14,
    C3 = 0x18,
}

unsafe impl Mmio32 for SystemTimer {
    #[inline]
    fn addr(&self) -> usize {
        super::mmio_base() + 0x3000 + *self as usize
    }
}

#[allow(dead_code)]
impl SystemTimer {
    #[inline]
    fn get() -> u64 {
        unsafe {
            loop {
                let hi = Self::Hi.read();
                let lo = Self::Lo.read();

                if hi == Self::Hi.read() {
                    break ((hi as u64) << 32) | (lo as u64);
                }
            }
        }
    }
}
