use core::{
    fmt,
    num::NonZeroU64,
    ops::{Add, BitAnd, BitOr, Mul, Not, Sub},
};

// type PageTableRepr = u64;

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
    pub const unsafe fn identity_map<T>(&self) -> *mut T {
        self.0 as usize as *mut T
    }

    /// Gets the pointer corresponding to the specified physical address.
    #[inline]
    pub const fn direct_map<T>(&self) -> *mut T {
        // TODO:
        // PageManager::direct_map(*self) as *mut T
        unsafe { self.identity_map() }
    }
}

impl Default for PhysicalAddress {
    #[inline]
    fn default() -> Self {
        Self(Default::default())
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
