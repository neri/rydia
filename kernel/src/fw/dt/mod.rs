//! Device Tree

use core::{ffi::c_void, slice, str};

pub struct DeviceTree {
    header: &'static Header,
}

impl DeviceTree {
    pub const FDT_BEGIN_NODE: u32 = 1;
    pub const FDT_END_NODE: u32 = 2;
    pub const FDT_PROP: u32 = 3;
    pub const FDT_NOP: u32 = 4;
    pub const FDT_END: u32 = 9;

    #[inline]
    pub unsafe fn parse(ptr: *const u8) -> Result<DeviceTree, ()> {
        let header = &*(ptr as *const Header);
        header.is_valid().then_some(DeviceTree { header }).ok_or(())
    }

    #[inline]
    pub const fn header(&self) -> &Header {
        self.header
    }
}

#[repr(C)]
pub struct Header {
    magic: u32,
    totalsize: u32,
    off_dt_struct: u32,
    off_dt_strings: u32,
    off_mem_rsvmap: u32,
    version: u32,
    last_comp_version: u32,
    boot_cpuid_phys: u32,
    size_dt_string: u32,
    size_dt_struct: u32,
}

impl Header {
    pub const MAGIC: u32 = 0xD00DFEED;
    pub const CURRENT_VERSION: u32 = 0x11;
    pub const COMPATIBLE_VERSION: u32 = 0x10;

    #[inline]
    pub const fn is_valid(&self) -> bool {
        self.magic() == Self::MAGIC
            && self.version() == Self::CURRENT_VERSION
            && self.last_comp_version() == Self::COMPATIBLE_VERSION
    }

    #[inline]
    pub const fn magic(&self) -> u32 {
        self.magic.to_be()
    }

    #[inline]
    pub const fn total_size(&self) -> usize {
        self.totalsize.to_be() as usize
    }

    #[inline]
    pub const fn off_dt_struct(&self) -> usize {
        self.off_dt_struct.to_be() as usize
    }

    #[inline]
    pub const fn off_dt_strings(&self) -> usize {
        self.off_dt_strings.to_be() as usize
    }

    #[inline]
    pub const fn off_mem_rsvmap(&self) -> usize {
        self.off_mem_rsvmap.to_be() as usize
    }

    #[inline]
    pub const fn version(&self) -> u32 {
        self.version.to_be()
    }

    #[inline]
    pub const fn last_comp_version(&self) -> u32 {
        self.last_comp_version.to_be()
    }

    #[inline]
    pub fn struct_ptr(&self) -> *const u32 {
        let p = self as *const Self as *const u8;
        unsafe { p.add(self.off_dt_struct()) as *const u32 }
    }

    #[inline]
    pub fn string_ptr(&self) -> *const u8 {
        let p = self as *const Self as *const u8;
        unsafe { p.add(self.off_dt_strings()) }
    }

    #[inline]
    pub fn tokens(&self) -> impl Iterator<Item = Token> {
        FdtTokenIter {
            header: self,
            index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Token<'a> {
    BeginNode(&'a str),
    EndNode,
    Prop(Name<'a>, *const c_void, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Name<'a>(&'a str);

impl Name<'_> {
    pub const ADSRESS_CELLS: Self = Self("#address-cells");
    pub const CLOCK_CELLS: Self = Self("#clock-cells");
    pub const COMPATIBLE: Self = Self("compatible");
    pub const MODEL: Self = Self("model");
    pub const PHANDLE: Self = Self("phandle");
    pub const RANGES: Self = Self("ranges");
    pub const REG: Self = Self("reg");
    pub const SIZE_CELLS: Self = Self("#size-cells");
    pub const STATUS: Self = Self("status");
}

impl<'a> Name<'a> {
    #[inline]
    pub const fn new(name: &'a str) -> Self {
        Self(name)
    }

    #[inline]
    pub const fn as_str(&'a self) -> &'a str {
        self.0
    }
}

#[allow(dead_code)]
struct FdtTokenIter<'a> {
    header: &'a Header,
    index: usize,
}

impl<'a> Iterator for FdtTokenIter<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut index = self.index;
            let mut ptr = self.header.struct_ptr().add(index);
            let result = loop {
                let token = ptr.read_volatile().to_be();
                match token {
                    DeviceTree::FDT_NOP => {
                        ptr = ptr.add(1);
                        index += 1;
                    }
                    DeviceTree::FDT_BEGIN_NODE => {
                        index += 1;
                        let p = ptr.add(1) as *const u8;
                        let len = _c_strlen(p);
                        let name = _c_string(p);
                        index += (len + 4) / 4;
                        break Token::BeginNode(name);
                    }
                    DeviceTree::FDT_PROP => {
                        let data_len = ptr.add(1).read_volatile().to_be() as usize;
                        let name_ptr = ptr.add(2).read_volatile().to_be() as usize;
                        let name = Name::new(_c_string(self.header.string_ptr().add(name_ptr)));
                        index += 3 + ((data_len + 3) / 4);
                        break Token::Prop(name, ptr.add(3) as *const c_void, data_len);
                    }
                    DeviceTree::FDT_END_NODE => {
                        index += 1;
                        break Token::EndNode;
                    }
                    _ => return None,
                }
            };
            self.index = index;
            Some(result)
        }
    }
}

fn _c_string<'a>(s: *const u8) -> &'a str {
    unsafe {
        let len = _c_strlen(s);
        let slice = slice::from_raw_parts(s, len);
        str::from_utf8_unchecked(slice)
    }
}

fn _c_strlen(s: *const u8) -> usize {
    let mut len = 0;
    while unsafe { s.add(len).read_volatile() != 0 } {
        len += 1
    }
    len
}
