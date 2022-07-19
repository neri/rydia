use crate::{fw::dt::DeviceTree, mem::MemoryManager};
use bitflags::*;
use core::{
    alloc::Layout,
    arch::asm,
    ffi::c_void,
    fmt,
    iter::Step,
    num::NonZeroU64,
    ops::{Add, BitAnd, BitOr, Mul, Not, Sub},
    sync::atomic::{AtomicU64, Ordering},
};

#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    pub const NULL: Self = Self(0);

    #[inline]
    pub const fn new(val: u64) -> Self {
        Self(val)
    }

    #[inline]
    pub const fn from_usize(val: usize) -> Self {
        Self(val as u64)
    }

    #[inline]
    pub const fn as_u64(&self) -> u64 {
        self.0 as u64
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0 as usize
    }

    #[inline]
    pub const fn rounding_up(&self, align: usize) -> Self {
        let delta = align as u64 - 1;
        let mask = !delta;
        Self(self.0.wrapping_add(delta) & mask)
    }

    #[inline]
    pub const fn round_up(&mut self, align: usize) {
        *self = self.rounding_up(align);
    }

    /// Gets a pointer identical to the specified physical address.
    ///
    /// # Safety
    ///
    /// Pointers of this form may not map to some memory.
    #[inline]
    pub const unsafe fn identity_mapped<T>(&self) -> *mut T {
        self.0 as usize as *mut T
    }

    /// Gets the pointer corresponding to the specified physical address.
    #[inline]
    pub const fn direct_mapped<T>(&self) -> *mut T {
        PageManager::direct_mapped(*self) as *mut T
    }
}

impl const Default for PhysicalAddress {
    #[inline]
    fn default() -> Self {
        Self::NULL
    }
}

impl const Add<usize> for PhysicalAddress {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs as u64)
    }
}

impl const Add<u64> for PhysicalAddress {
    type Output = Self;

    #[inline]
    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl const Sub<PhysicalAddress> for PhysicalAddress {
    type Output = usize;

    #[inline]
    fn sub(self, rhs: PhysicalAddress) -> Self::Output {
        (self.0 - rhs.0) as usize
    }
}

impl const Sub<usize> for PhysicalAddress {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs as u64)
    }
}

impl const Mul<usize> for PhysicalAddress {
    type Output = Self;

    fn mul(self, rhs: usize) -> Self::Output {
        Self(self.0 * rhs as u64)
    }
}

impl const Mul<u64> for PhysicalAddress {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl const BitAnd<u64> for PhysicalAddress {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: u64) -> Self::Output {
        Self(self.0 & rhs)
    }
}

impl const BitAnd<PhysicalAddress> for u64 {
    type Output = Self;

    fn bitand(self, rhs: PhysicalAddress) -> Self::Output {
        self & rhs.0
    }
}

impl const BitOr<u64> for PhysicalAddress {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: u64) -> Self::Output {
        Self(self.0 | rhs)
    }
}

impl const Not for PhysicalAddress {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self(!self.0)
    }
}

impl const From<u64> for PhysicalAddress {
    #[inline]
    fn from(val: u64) -> Self {
        Self::new(val)
    }
}

impl const From<PhysicalAddress> for u64 {
    #[inline]
    fn from(val: PhysicalAddress) -> Self {
        val.as_u64()
    }
}

impl fmt::Debug for PhysicalAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:012x}", self.0)
    }
}

impl Step for PhysicalAddress {
    #[inline]
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        u64::steps_between(&start.0, &end.0)
    }

    #[inline]
    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        u64::forward_checked(start.0, count).map(|v| PhysicalAddress(v))
    }

    #[inline]
    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        u64::backward_checked(start.0, count).map(|v| PhysicalAddress(v))
    }
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NonNullPhysicalAddress(NonZeroU64);

impl NonNullPhysicalAddress {
    #[inline]
    pub const fn get(&self) -> PhysicalAddress {
        PhysicalAddress(self.0.get())
    }

    #[inline]
    pub const fn new(val: PhysicalAddress) -> Option<Self> {
        match NonZeroU64::new(val.as_u64()) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    #[inline]
    pub const unsafe fn new_unchecked(val: PhysicalAddress) -> Self {
        Self(NonZeroU64::new_unchecked(val.as_u64()))
    }
}

impl const From<NonNullPhysicalAddress> for PhysicalAddress {
    #[inline]
    fn from(val: NonNullPhysicalAddress) -> Self {
        val.get()
    }
}

pub struct PageManager;

static MAIR: MemmoryAttributeIndirectionRegister = MemmoryAttributeIndirectionRegister::default();
static TCR: TranslationControlRegister = TranslationControlRegister::default();
static TTBR0: AtomicU64 = AtomicU64::new(0);
static SCTLR: AtomicU64 = AtomicU64::new(0);

impl PageManager {
    const PAGE_SIZE_MIN: usize = 0x0000_1000;
    const PAGE_SIZE_2M: usize = 0x0020_0000;
    const PAGE_SIZE_1G: usize = 0x4000_0000;
    const PAGE_SIZE_M1: usize = 0xFFF;
    // const PAGE_SIZE_2M_M1: usize = 0x1F_FFFF;
    // const PAGE_SIZE_1G_M1: usize = 0x3FFF_FFFF;
    // const PAGE_KERNEL_PREFIX: usize = 0xFFFF_0000_0000_0000;
    // const PAGE_RECURSIVE: usize = 0x1FE;
    // const PAGE_KERNEL_HEAP: usize = 0x1FC;
    // const PAGE_DIRECT_MAP: usize = 0x180;
    // const DIRECT_BASE: usize = Self::PAGE_KERNEL_PREFIX | (Self::PAGE_DIRECT_MAP << 39);
    // const HEAP_BASE: usize = Self::PAGE_KERNEL_PREFIX | (Self::PAGE_KERNEL_HEAP << 39);

    #[inline]
    pub unsafe fn init_early(dtb: usize) {
        asm!("dsb sy");

        if let Ok(dt) = DeviceTree::parse(dtb as *const u8) {
            let max_pa = dt
                .memory_ranges()
                .unwrap()
                .fold(0, |acc, val| acc.max(val.0.as_u64() + (val.1 as u64)));
            let max_pa = PhysicalAddress::new(u64::max(super::max_pa().as_u64(), max_pa))
                .rounding_up(Self::PAGE_SIZE_1G);
            let size_l2 = max_pa.as_usize() / Self::PAGE_SIZE_1G * Self::PAGE_SIZE_MIN;
            let size_l1 = (size_l2 + Self::PAGE_SIZE_M1) / Self::PAGE_SIZE_MIN;
            let alloc_l2_size = (size_l2 * 8 + Self::PAGE_SIZE_M1) & !Self::PAGE_SIZE_M1;
            let alloc_l1_size = (size_l1 * 8 + Self::PAGE_SIZE_M1) & !Self::PAGE_SIZE_M1;

            let table_l2 = MemoryManager::early_alloc(Layout::from_size_align_unchecked(
                alloc_l2_size,
                Self::PAGE_SIZE_MIN,
            ))
            .unwrap()
            .get()
            .identity_mapped::<TranslationTableDescriptor>();
            table_l2.write_bytes(0, alloc_l2_size);

            for range in dt.memory_ranges().unwrap() {
                let start = range.0;
                let end = (range.0 + range.1).rounding_up(Self::PAGE_SIZE_2M);
                let base = table_l2.add(start.as_usize() / Self::PAGE_SIZE_2M);
                let attr = PageAttributes::block(Some(Shareable::Inner), AttributeIndex::Normal);
                for (index, oa) in (start..end)
                    .into_iter()
                    .step_by(Self::PAGE_SIZE_2M)
                    .enumerate()
                {
                    base.add(index)
                        .write_volatile(TranslationTableDescriptor::new(oa, attr));
                }
            }

            for range in super::vram_memlist() {
                if range.1 == 0 {
                    continue;
                }
                let start = range.0;
                let end = (range.0 + range.1).rounding_up(Self::PAGE_SIZE_2M);
                let base = table_l2.add(start.as_usize() / Self::PAGE_SIZE_2M);
                let attr = PageAttributes::block(Some(Shareable::Outer), AttributeIndex::Vram);
                for (index, oa) in (start..end)
                    .into_iter()
                    .step_by(Self::PAGE_SIZE_2M)
                    .enumerate()
                {
                    base.add(index)
                        .write_volatile(TranslationTableDescriptor::new(oa, attr));
                }
            }

            for range in super::device_memlist() {
                if range.1 == 0 {
                    continue;
                }
                let start = range.0;
                let end = (range.0 + range.1).rounding_up(Self::PAGE_SIZE_2M);
                let base = table_l2.add(start.as_usize() / Self::PAGE_SIZE_2M);
                let attr = PageAttributes::block(None, AttributeIndex::Device);
                for (index, oa) in (start..end)
                    .into_iter()
                    .step_by(Self::PAGE_SIZE_2M)
                    .enumerate()
                {
                    base.add(index)
                        .write_volatile(TranslationTableDescriptor::new(oa, attr));
                }
            }

            let table_l1 = MemoryManager::early_alloc(Layout::from_size_align_unchecked(
                alloc_l1_size,
                Self::PAGE_SIZE_MIN,
            ))
            .unwrap()
            .get()
            .identity_mapped::<TranslationTableDescriptor>();
            table_l1.write_bytes(0, alloc_l1_size);

            let attr = PageAttributes::table();
            for (index, addr) in (0..size_l2)
                .into_iter()
                .step_by(Self::PAGE_SIZE_MIN)
                .enumerate()
            {
                let oa = PhysicalAddress::from_usize(table_l2 as usize + addr);
                table_l1
                    .add(index)
                    .write_volatile(TranslationTableDescriptor::new(oa, attr));
            }

            TTBR0.store(table_l1 as usize as u64, Ordering::SeqCst);

            SCTLR.store(0x00C0_181F, Ordering::SeqCst);
        }
    }

    #[inline]
    pub unsafe fn init_mp() {
        asm!("dsb sy");

        MAIR.load_el1();

        asm!("msr ttbr0_el1, {}", in(reg)TTBR0.load(Ordering::Relaxed));

        asm!("isb");

        TCR.load_el1();

        asm!("isb
        msr sctlr_el1, {}
        isb", in(reg)SCTLR.load(Ordering::Relaxed));
    }

    #[inline]
    pub const fn direct_mapped(val: PhysicalAddress) -> *mut c_void {
        val.0 as usize as *mut c_void
    }
}

bitflags! {
    pub struct TranslationControlRegister: u64 {
        const T0SZ_MASK     = 0b111111;
        const EPD0          = 1 << 7;
        const IRGN0_MASK    = 0b11 << 8;
        const ORGN0_MASK    = 0b11 << 10;
        const SH0_MASK      = 0b11 << 12;
        const TG0_MASK      = 0b11 << 14;

        const T1SZ_MASK     = 0b111111 << 16;
        const A1            = 1 << 22;
        const EPD1          = 1 << 23;
        const IRGN1_MASK    = 0b11 << 24;
        const ORGN1_MASK    = 0b11 << 26;
        const SH1_MASK      = 0b11 << 28;
        const TG1_MASK      = 0b11 << 30;

        const IPS_MASK      = 0b111 << 32;
        const AS            = 1 << 36;
        const TBI0          = 1 << 37;
        const TBI1          = 1 << 38;
        const HA            = 1 << 39;
        const HD            = 1 << 40;
        const HPD0          = 1 << 41;
        const HPD1          = 1 << 42;
        const HWU059        = 1 << 43;
        const HWU060        = 1 << 44;
        const HWU061        = 1 << 45;
        const HWU062        = 1 << 46;
        const HWU159        = 1 << 47;
        const HWU160        = 1 << 48;
        const HWU161        = 1 << 49;
        const HWU162        = 1 << 50;
        const TBID0         = 1 << 51;
        const TBID1         = 1 << 52;
        const NFD0          = 1 << 53;
        const NFD1          = 1 << 54;
        const E0PD0         = 1 << 55;
        const E0PD1         = 1 << 56;
        const TCMA0         = 1 << 57;
        const TCMA1         = 1 << 58;
        const DS            = 1 << 59;
    }
}

impl TranslationControlRegister {
    #[inline]
    pub unsafe fn load_el1(&self) {
        asm!("msr tcr_el1, {}", in(reg)self.bits());
    }

    #[inline]
    pub const fn tg0(&self, val: TcrGranule0) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::TG0_MASK.bits()) | ((val as u64) << 14))
    }

    #[inline]
    pub const fn sh0(&self, val: Shareable) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::SH0_MASK.bits()) | ((val as u64) << 12))
    }

    #[inline]
    pub const fn orgn0(&self, val: Cacheable) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::ORGN0_MASK.bits()) | ((val as u64) << 10))
    }

    #[inline]
    pub const fn irgn0(&self, val: Cacheable) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::IRGN0_MASK.bits()) | ((val as u64) << 8))
    }

    #[inline]
    pub const fn t0sz(&self, val: usize) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::T0SZ_MASK.bits()) | ((val as u64) << 0))
    }

    #[inline]
    pub const fn tg1(&self, val: TcrGranule1) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::TG1_MASK.bits()) | ((val as u64) << 30))
    }

    #[inline]
    pub const fn sh1(&self, val: Shareable) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::SH1_MASK.bits()) | ((val as u64) << 28))
    }

    #[inline]
    pub const fn orgn1(&self, val: Cacheable) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::ORGN1_MASK.bits()) | ((val as u64) << 26))
    }

    #[inline]
    pub const fn irgn1(&self, val: Cacheable) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::IRGN1_MASK.bits()) | ((val as u64) << 24))
    }

    #[inline]
    pub const fn t1sz(&self, val: usize) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::T1SZ_MASK.bits()) | ((val as u64) << 16))
    }

    #[inline]
    pub const fn ips(&self, val: TcrIps) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::IPS_MASK.bits()) | ((val as u64) << 32))
    }

    #[inline]
    pub const fn config(
        &self,
        ips: TcrIps,
        t0sz: usize,
        tg0: TcrGranule0,
        sh0: Shareable,
        irgn0: Cacheable,
        orgn0: Cacheable,
        t1sz: usize,
        tg1: TcrGranule1,
        sh1: Shareable,
        irgn1: Cacheable,
        orgn1: Cacheable,
    ) -> Self {
        self.ips(ips)
            .t0sz(t0sz)
            .tg0(tg0)
            .sh0(sh0)
            .irgn0(irgn0)
            .orgn0(orgn0)
            .t1sz(t1sz)
            .tg1(tg1)
            .sh1(sh1)
            .irgn1(irgn1)
            .orgn1(orgn1)
    }
}

impl const Default for TranslationControlRegister {
    #[inline]
    fn default() -> Self {
        Self::empty().config(
            TcrIps::_36bit,
            25,
            TcrGranule0::_4KB,
            Shareable::Inner,
            Cacheable::WriteThru,
            Cacheable::WriteThru,
            25,
            TcrGranule1::_4KB,
            Shareable::Inner,
            Cacheable::WriteThru,
            Cacheable::WriteThru,
        )
    }
}

#[repr(transparent)]
pub struct MemmoryAttributeIndirectionRegister(u64);

impl MemmoryAttributeIndirectionRegister {
    #[inline]
    pub unsafe fn load_el1(&self) {
        asm!("msr mair_el1, {}", in(reg)self.bits());
    }

    #[inline]
    pub const fn bits(&self) -> u64 {
        self.0
    }
}

impl const Default for MemmoryAttributeIndirectionRegister {
    #[inline]
    fn default() -> Self {
        Self(0x4e_00_44_ff)
    }
}

#[repr(u64)]
#[derive(Debug, Clone, Copy)]
pub enum AttributeIndex {
    Normal,
    NoCache,
    Device,
    Vram,
}

#[repr(transparent)]
pub struct TranslationTableBaseRegister(u64);

#[derive(Debug, Clone, Copy)]
pub enum TcrIps {
    /// 32bits, 4GB
    _32bit = 0b000,
    /// 36bits, 64GB
    _36bit = 0b001,
    /// 40bits, 1TB
    _40bit = 0b010,
    /// 42bits, 4TB
    _42bit = 0b011,
    /// 44bits, 16TB
    _44bit = 0b100,
    /// 48bits, 256TB
    _48bit = 0b101,
    /// 52bits, 4PB
    _52bit = 0b110,
}

#[derive(Debug, Clone, Copy)]
pub enum TcrGranule0 {
    _4KB = 0b00,
    _64KB = 0b01,
    _16KB = 0b10,
}

#[derive(Debug, Clone, Copy)]
pub enum TcrGranule1 {
    _16KB = 0b01,
    _4KB = 0b10,
    _64KB = 0b11,
}

#[derive(Debug, Clone, Copy)]
pub enum Shareable {
    Outer = 0b10,
    Inner = 0b11,
}

#[derive(Debug, Clone, Copy)]
pub enum Cacheable {
    WriteBack = 0b01,
    WriteThru = 0b10,
    WriteBackNoCache = 0b11,
}

#[repr(transparent)]
pub struct TranslationTableDescriptor(u64);

impl TranslationTableDescriptor {
    /// Descriptor is valid
    const VALID: Self = Self(1);

    const OA48_MASK: u64 = 0xFFFF_FFFF_F000;

    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn new(oa: PhysicalAddress, attr: PageAttributes) -> Self {
        Self((oa.0 & Self::OA48_MASK) | attr.bits() | Self::VALID.0)
    }

    #[inline]
    pub const fn is_valid(&self) -> bool {
        (self.0 & Self::VALID.0) == Self::VALID.0
    }

    #[inline]
    pub const fn oa48(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.0 & Self::OA48_MASK)
    }

    #[inline]
    pub const fn bits(&self) -> u64 {
        self.0
    }
}

impl const Default for TranslationTableDescriptor {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageAttributes(u64);

impl PageAttributes {
    /// Descriptor points to next-level table or last-level descriptor
    const TABLE: u64 = 1 << 1;

    const NS: u64 = 1 << 5;
    const AF: u64 = 1 << 10;

    #[inline]
    pub const fn block(sh: Option<Shareable>, attr_index: AttributeIndex) -> Self {
        let sh = match sh {
            Some(v) => v as u64,
            None => 0,
        };
        Self(Self::AF | (sh << 8) | ((attr_index as u64) << 2))
    }

    #[inline]
    pub const fn table() -> Self {
        Self(Self::TABLE)
    }

    #[inline]
    pub const fn bits(&self) -> u64 {
        self.0
    }
}
