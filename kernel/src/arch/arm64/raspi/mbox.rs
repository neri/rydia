use crate::{arch::cpu::Cpu, mem::mmio::Mmio32};
use core::arch::asm;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum Mbox {
    POWER = 0,
    FB = 1,
    VUART = 2,
    VCHIQ = 3,
    LEDS = 4,
    BTNS = 5,
    TOUCH = 6,
    COUNT = 7,
    PROP = 8,
}

impl Mbox {
    #[inline]
    pub fn mbox<const N: usize>(&self) -> Option<MboxContext<N>> {
        MboxContext::new(*self)
    }
}

pub struct MboxContext<const N: usize> {
    payload: Payload<N>,
    chan: Mbox,
}

impl<const N: usize> MboxContext<N> {
    const REQUEST: u32 = 0x0000_0000;
    const RESPONSE: u32 = 0x8000_0000;
    const FULL: u32 = 0x8000_0000;
    const EMPTY: u32 = 0x4000_0000;

    pub fn new(chan: Mbox) -> Option<Self> {
        if N < 3 {
            return None;
        }
        unsafe {
            let mut mbox = Self {
                payload: Payload([0xdeadbeef; N]),
                chan,
            };
            *mbox.payload.0.get_unchecked_mut(0) = 12;
            *mbox.payload.0.get_unchecked_mut(1) = Self::REQUEST;
            *mbox.payload.0.get_unchecked_mut(2) = RawTag::LAST.as_u32();
            Some(mbox)
        }
    }

    #[inline]
    pub fn append(&mut self, tag: Tag) -> Result<usize, ()> {
        tag.append_to(&mut self.payload.0)
    }

    pub fn slice(&self) -> &[u32] {
        &self.payload.0
    }

    pub fn mbox_addr(&self) -> u32 {
        let p = self.payload.0.as_ptr() as usize as u32;
        p | (self.chan as u32)
    }

    #[inline]
    unsafe fn flush(&self) {
        for p in self.payload.0.iter() {
            asm!("dc ivac, {}", in(reg)p);
        }
    }

    pub fn call(&mut self) -> Result<(), ()> {
        unsafe {
            let len0 = *self.payload.0.get_unchecked(0) as usize;
            if len0 < 12 || self.payload.0.len() * 4 <= len0 {
                return Err(());
            }

            self.flush();
            let mbox_addr = self.mbox_addr();

            while (Regs::STATUS.read() & Self::FULL) != 0 {
                Cpu::no_op();
            }

            Regs::WRITE.write(mbox_addr);

            loop {
                while (Regs::STATUS.read() & Self::EMPTY) != 0 {
                    Cpu::no_op();
                }

                if Regs::READ.read() == mbox_addr {
                    return (*self.payload.0.get_unchecked(1) == Self::RESPONSE)
                        .then_some(())
                        .ok_or(());
                }
            }
        }
    }
}

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
enum Regs {
    READ = 0x00,
    POLL = 0x10,
    SENDER = 0x14,
    STATUS = 0x18,
    CONFIG = 0x1C,
    WRITE = 0x20,
}

impl Regs {
    #[inline]
    pub fn base_addr() -> usize {
        super::mmio_base() + 0x0000_B880
    }
}

unsafe impl Mmio32 for Regs {
    #[inline]
    fn addr(&self) -> usize {
        Self::base_addr() + *self as usize
    }
}

#[repr(align(16))]
pub struct Payload<const N: usize>([u32; N]);

#[allow(dead_code)]
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum RawTag {
    LAST = 0,

    GETSERIAL = 0x10004,

    SETPOWER = 0x28001,
    GetClockState = 0x00030001,
    SetClockState = 0x00038001,
    GetClockRate = 0x00030002,
    SETCLKRATE = 0x38002,
    SETPHYWH = 0x48003,
    SETVIRTWH = 0x48004,
    SETVIRTOFF = 0x48009,
    SETDEPTH = 0x48005,
    SETPXLORDR = 0x48006,
    GETFB = 0x40001,
    GETPITCH = 0x40008,
}

impl RawTag {
    #[inline]
    pub const fn as_u32(&self) -> u32 {
        *self as u32
    }
}

#[allow(dead_code)]
#[repr(u32)]
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
pub enum ClockId {
    UART = 0x000000002,
    ARM = 0x000000003,
    CORE = 0x000000004,
    V3D = 0x000000005,
    H264 = 0x000000006,
    ISP = 0x000000007,
    SDRAM = 0x000000008,
    PIXEL = 0x000000009,
    PWM = 0x00000000a,
    HEVC = 0x00000000b,
    EMMC2 = 0x00000000c,
    M2MC = 0x00000000d,
    PIXEL_BVB = 0x00000000e,
}

#[allow(non_camel_case_types)]
pub enum Tag {
    SET_CLKRATE(ClockId, u32, u32),
    SET_PHYWH(u32, u32),
    SET_VIRTWH(u32, u32),
    SET_VIRTOFF(u32, u32),
    SET_DEPTH(u32),
    SET_PXLORDR(u32),
    GET_FB(u32, u32),
    GET_PITCH,
}

impl Tag {
    #[inline]
    const fn info(&self) -> (RawTag, u32, u32) {
        match *self {
            Tag::SET_CLKRATE(_, _, _) => (RawTag::SETCLKRATE, 3, 2),
            Tag::SET_PHYWH(_, _) => (RawTag::SETPHYWH, 2, 0),
            Tag::SET_VIRTWH(_, _) => (RawTag::SETVIRTWH, 2, 2),
            Tag::SET_VIRTOFF(_, _) => (RawTag::SETVIRTOFF, 2, 2),
            Tag::SET_DEPTH(_) => (RawTag::SETDEPTH, 1, 1),
            Tag::SET_PXLORDR(_) => (RawTag::SETPXLORDR, 1, 1),
            Tag::GET_FB(_, _) => (RawTag::GETFB, 2, 2),
            Tag::GET_PITCH => (RawTag::GETPITCH, 1, 1),
        }
    }

    #[inline]
    fn _push(slice: &mut [u32], index: usize, val: u32) -> Result<usize, ()> {
        match slice.get_mut(index) {
            Some(p) => {
                *p = val;
                Ok(index + 1)
            }
            None => Err(()),
        }
    }

    #[inline]
    fn _push_slice(slice: &mut [u32], mut index: usize, data: &[u32]) -> Result<usize, ()> {
        for val in data {
            index = Self::_push(slice, index, *val)?;
        }
        Ok(index)
    }

    pub fn append_to(&self, slice: &mut [u32]) -> Result<usize, ()> {
        let (tag, len1, len2) = self.info();
        let len0 = match slice.get(0) {
            Some(v) => ((*v as usize) / 4) - 1,
            None => return Err(()),
        };
        let new_len = len0 + (len1 as usize) + 4;
        if new_len > slice.len() {
            return Err(());
        }
        let index = len0;
        let index = Self::_push(slice, index, tag.as_u32())?;
        let index = Self::_push(slice, index, len1 * 4)?;
        let index = Self::_push(slice, index, len2 * 4)?;
        let result = index;

        let index = match *self {
            Tag::SET_CLKRATE(x, y, z) => Self::_push_slice(slice, index, &[x as u32, y, z])?,
            Tag::SET_PHYWH(x, y) => Self::_push_slice(slice, index, &[x, y])?,
            Tag::SET_VIRTWH(x, y) => Self::_push_slice(slice, index, &[x, y])?,
            Tag::SET_VIRTOFF(x, y) => Self::_push_slice(slice, index, &[x, y])?,
            Tag::SET_DEPTH(x) => Self::_push(slice, index, x)?,
            Tag::SET_PXLORDR(x) => Self::_push(slice, index, x)?,
            Tag::GET_FB(x, y) => Self::_push_slice(slice, index, &[x, y])?,
            Tag::GET_PITCH => Self::_push(slice, index, 0)?,
        };

        let index = Self::_push(slice, index, RawTag::LAST.as_u32())?;
        slice[0] = (index as u32) * 4;
        if new_len != index {
            return Err(());
        }

        Ok(result)
    }
}
