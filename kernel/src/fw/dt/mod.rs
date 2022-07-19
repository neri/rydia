//! Device Tree

use core::{
    ffi::c_void,
    ptr::null,
    slice::{self, Iter},
    str,
};

use crate::mem::PhysicalAddress;

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
        if ptr == null() {
            return Err(());
        }
        let header = &*(ptr as *const Header);
        header.is_valid().then_some(DeviceTree { header }).ok_or(())
    }

    #[inline]
    pub const fn header(&self) -> &Header {
        self.header
    }

    /// Finds the specified property from the root node.
    pub fn find_root_prop(&self, prop_name: PropName) -> Option<(*const c_void, usize)> {
        for token in self.header().tokens() {
            match token {
                Token::BeginNode(name) => {
                    if name != NodeName::ROOT {
                        break;
                    }
                }
                Token::Prop(name, ptr, len) => {
                    if name == prop_name {
                        return Some((ptr, len));
                    }
                }
                Token::EndNode => break,
            }
        }
        None
    }

    pub fn find_first_level_prop(
        &self,
        node_name: NodeName,
        prop_name: PropName,
    ) -> Option<(*const c_void, usize)> {
        let mut current_level = -1;
        let mut root_node_matches = false;
        for token in self.header().tokens().into_iter() {
            match token {
                Token::BeginNode(name) => {
                    root_node_matches = node_name == name.without_unit();
                    current_level += 1;
                }
                Token::EndNode => {
                    current_level -= 1;
                }
                Token::Prop(name, ptr, len) => {
                    if current_level == 1 && root_node_matches && name == prop_name {
                        return Some((ptr, len));
                    }
                }
            }
        }
        None
    }

    /// Returns the `model` property of the root element.
    pub fn root_model<'a>(&self) -> Option<&'a str> {
        self.find_root_prop(PropName::MODEL)
            .and_then(|(ptr, len)| unsafe {
                (len > 1).then(|| {
                    let slice = slice::from_raw_parts(ptr as *const _, len - 1);
                    core::str::from_utf8(slice).ok()
                })
            })
            .flatten()
    }

    pub fn memory_ranges(&self) -> Option<impl Iterator<Item = (PhysicalAddress, usize)>> {
        unsafe {
            let address_cells =
                match self
                    .find_root_prop(PropName::ADDRESS_CELLS)
                    .and_then(|(ptr, len)| {
                        (len >= 4).then(|| (ptr as *const u32).read_volatile().to_be())
                    }) {
                    Some(v) => v as usize,
                    None => return None,
                };
            let size_cells =
                match self
                    .find_root_prop(PropName::SIZE_CELLS)
                    .and_then(|(ptr, len)| {
                        (len >= 4).then(|| (ptr as *const u32).read_volatile().to_be())
                    }) {
                    Some(v) => v as usize,
                    None => return None,
                };
            let slice = match self.find_first_level_prop(NodeName::MEMORY, PropName::REG) {
                Some((ptr, len)) => slice::from_raw_parts(ptr as *const u32, len / 4),
                None => return None,
            };
            Some(FdtMemoryRangeIter::new(slice, address_cells, size_cells))
        }
    }

    // /// Returns the `compatible` property of the root element.
    // pub fn root_compatible<'a>(&self) -> Option<&'a str> {
    //     self.find_root_prop(PropName::COMPATIBLE)
    //         .and_then(|(ptr, len)| unsafe {
    //             let slice = slice::from_raw_parts(ptr as *const _, len);
    //             core::str::from_utf8(slice).ok()
    //         })
    // }
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
        unsafe { p.add(self.off_dt_struct()) as *const _ }
    }

    #[inline]
    pub fn string_ptr(&self) -> *const u8 {
        let p = self as *const Self as *const u8;
        unsafe { p.add(self.off_dt_strings()) }
    }

    #[inline]
    pub fn reserve_map_ptr(&self) -> *const u64 {
        let p = self as *const Self as *const u8;
        unsafe { p.add(self.off_mem_rsvmap()) as *const _ }
    }

    #[inline]
    pub fn reserved_maps(&self) -> impl Iterator<Item = (u64, u64)> + '_ {
        FdtRsvMapIter {
            header: self,
            index: 0,
        }
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
    BeginNode(NodeName<'a>),
    EndNode,
    Prop(PropName<'a>, *const c_void, usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeName<'a>(pub &'a str);

impl NodeName<'_> {
    pub const ROOT: Self = Self("");

    pub const ALIASES: Self = Self("aliases");
    pub const MEMORY: Self = Self("memory");
    pub const RESERVED_MEMORY: Self = Self("reserved-memory");
    pub const CHOSEN: Self = Self("chosen");
    pub const CPUS: Self = Self("cpus");

    pub fn without_unit(&self) -> Self {
        if let Some(len) = self.0.find("@") {
            Self(&self.0[..len])
        } else {
            Self(&self.0)
        }
    }
}

impl<'a> NodeName<'a> {
    #[inline]
    pub const fn new(name: &'a str) -> Self {
        Self(name)
    }

    #[inline]
    pub const fn as_str(&'a self) -> &'a str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PropName<'a>(pub &'a str);

impl PropName<'_> {
    /// #address-cells <u32>
    pub const ADDRESS_CELLS: Self = Self("#address-cells");
    ///
    pub const CLOCK_CELLS: Self = Self("#clock-cells");
    /// compatible <string-list>
    pub const COMPATIBLE: Self = Self("compatible");
    /// device_type (deprecated) <string>
    pub const DEVICE_TYPE: Self = Self("device_type");
    /// dma-coherent <empty>
    pub const DMA_COHERENT: Self = Self("dma-coherent");
    /// dma-ranges <prop-encoded-array>
    pub const DMA_RANGES: Self = Self("dma-ranges");
    /// model <string>
    pub const MODEL: Self = Self("model");
    /// name (deprecated) <string>
    pub const NAME: Self = Self("name");
    /// phandle <u32>
    pub const PHANDLE: Self = Self("phandle");
    /// ranges <prop-encoded-array>
    pub const RANGES: Self = Self("ranges");
    /// reg <prop-encoded-array>
    pub const REG: Self = Self("reg");
    /// #size-cells <u32>
    pub const SIZE_CELLS: Self = Self("#size-cells");
    /// status <string>
    pub const STATUS: Self = Self("status");
    /// virtual-reg <u32>
    pub const VIRTUAL_REG: Self = Self("virtual-reg");
}

impl<'a> PropName<'a> {
    #[inline]
    pub const fn new(name: &'a str) -> Self {
        Self(name)
    }

    #[inline]
    pub const fn as_str(&'a self) -> &'a str {
        self.0
    }
}

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
                        let name = NodeName(_c_string(p));
                        index += (len + 4) / 4;
                        break Token::BeginNode(name);
                    }
                    DeviceTree::FDT_PROP => {
                        let data_len = ptr.add(1).read_volatile().to_be() as usize;
                        let name_ptr = ptr.add(2).read_volatile().to_be() as usize;
                        let name = PropName(_c_string(self.header.string_ptr().add(name_ptr)));
                        index += 3 + ((data_len + 3) / 4);
                        break Token::Prop(name, ptr.add(3) as *const c_void, data_len);
                    }
                    DeviceTree::FDT_END_NODE => {
                        index += 1;
                        break Token::EndNode;
                    }
                    // DeviceTree::FDT_END
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
    unsafe {
        let mut len = 0;
        while s.add(len).read_volatile() != 0 {
            len += 1
        }
        len
    }
}

struct FdtRsvMapIter<'a> {
    header: &'a Header,
    index: usize,
}

impl Iterator for FdtRsvMapIter<'_> {
    type Item = (u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let ptr = self.header.reserve_map_ptr().add(self.index);
            let base = ptr.read_volatile().to_be();
            let size = ptr.add(1).read_volatile().to_be();
            if size > 0 {
                self.index += 2;
                Some((base, size))
            } else {
                None
            }
        }
    }
}

struct FdtMemoryRangeIter<'a> {
    iter: Iter<'a, u32>,
    address_cells: usize,
    size_cells: usize,
}

impl<'a> FdtMemoryRangeIter<'a> {
    #[inline]
    fn new(slice: &'a [u32], address_cells: usize, size_cells: usize) -> Self {
        let iter = slice.into_iter();
        Self {
            iter,
            address_cells,
            size_cells,
        }
    }

    fn fdt_get_reg_val(&mut self, cell_size: usize) -> Result<u64, ()> {
        match cell_size {
            1 => Ok(self.iter.next().ok_or(())?.to_be() as u64),
            2 => {
                let hi = self.iter.next().ok_or(())?.to_be() as u64;
                let lo = self.iter.next().ok_or(())?.to_be() as u64;
                Ok((hi << 32) + lo)
            }
            _ => Err(()),
        }
    }
}

impl Iterator for FdtMemoryRangeIter<'_> {
    type Item = (PhysicalAddress, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let base = match self.fdt_get_reg_val(self.address_cells) {
            Ok(v) => PhysicalAddress::new(v),
            Err(_) => return None,
        };
        let size = match self.fdt_get_reg_val(self.size_cells) {
            Ok(v) => v as usize,
            Err(_) => return None,
        };

        Some((base, size))
    }
}
