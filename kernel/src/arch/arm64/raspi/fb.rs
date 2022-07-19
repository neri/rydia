use super::mbox::{Mbox, Tag};
use crate::{drawing::*, mem::PhysicalAddress};

#[allow(dead_code)]
pub struct Fb;

#[allow(dead_code)]
impl Fb {
    pub fn init(width: u32, height: u32) -> Result<(*mut TrueColor, isize, isize, usize), ()> {
        let mut mbox = Mbox::PROP.mbox::<36>().ok_or(())?;

        mbox.append(Tag::SET_PHYWH(width, height))?;

        let index_vwh = mbox.append(Tag::SET_VIRTWH(width, height))?;

        mbox.append(Tag::SET_VIRTOFF(0, 0))?;

        mbox.append(Tag::SET_DEPTH(32))?;

        mbox.append(Tag::SET_PXLORDR(0))?;

        let index_fb = mbox.append(Tag::GET_FB(4096, 0))?;

        let index_pitch = mbox.append(Tag::GET_PITCH)?;

        match mbox.call() {
            Ok(_) => {
                let ptr = (mbox.slice()[index_fb] & 0x3FFFFFFF) as usize as *mut TrueColor;
                let w = mbox.slice()[index_vwh] as isize;
                let h = mbox.slice()[index_vwh + 1] as isize;
                let stride = mbox.slice()[index_pitch] as usize / 4;
                Ok((ptr, w, h, stride))
            }
            Err(_) => Err(()),
        }
    }

    pub fn get_fb() -> Result<(PhysicalAddress, usize), ()> {
        let mut mbox = Mbox::PROP.mbox::<36>().ok_or(())?;

        let index_fb = mbox.append(Tag::GET_FB(0, 0))?;

        match mbox.call() {
            Ok(_) => {
                let ptr =
                    PhysicalAddress::from_usize((mbox.slice()[index_fb] & 0x3FFFFFFF) as usize);
                let size = (mbox.slice()[index_fb + 1]) as usize;
                Ok((ptr, size))
            }
            Err(_) => Err(()),
        }
    }
}
