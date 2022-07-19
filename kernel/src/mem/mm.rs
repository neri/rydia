use super::{fixedvec::FixedVec, slab::*};
use crate::{arch, fw::dt, sync::spinlock::SpinMutex, system::System};
use alloc::boxed::Box;
use bitflags::*;
use core::{
    alloc::Layout,
    cell::UnsafeCell,
    ffi::c_void,
    fmt::Write,
    mem::{size_of, transmute},
    num::*,
    sync::atomic::*,
};

pub use crate::arch::page::{NonNullPhysicalAddress, PhysicalAddress};

static mut MM: UnsafeCell<MemoryManager> = UnsafeCell::new(MemoryManager::new());

static LAST_ALLOC_PTR: AtomicUsize = AtomicUsize::new(0);

/// Memory Manager
pub struct MemoryManager {
    reserved_memory_size: usize,
    page_size_min: usize,
    lost_size: AtomicUsize,
    free_pages: AtomicUsize,
    n_fragments: AtomicUsize,
    mem_list: SpinMutex<FixedVec<MemFreePair, { Self::MAX_FREE_PAIRS }>>,
    slab: Option<Box<SlabAllocator>>,

    early_start: PhysicalAddress,
    early_end: PhysicalAddress,

    #[allow(dead_code)]
    real_bitmap: [u32; 8],
}

pub enum InitializationSource<'a> {
    /// Device Tree
    DeviceTree(&'a dt::DeviceTree),
    /// TODO:
    Uefi(usize),
}

impl MemoryManager {
    const MAX_FREE_PAIRS: usize = 1024;
    pub const PAGE_SIZE_MIN: usize = 0x1000;

    const fn new() -> Self {
        Self {
            reserved_memory_size: 0,
            page_size_min: 0x1000,
            lost_size: AtomicUsize::new(0),
            free_pages: AtomicUsize::new(0),
            n_fragments: AtomicUsize::new(0),
            mem_list: SpinMutex::new(FixedVec::new(MemFreePair::empty())),
            slab: None,

            early_start: PhysicalAddress::NULL,
            early_end: PhysicalAddress::NULL,

            real_bitmap: [0; 8],
        }
    }

    pub(crate) unsafe fn init_early(start: PhysicalAddress, len: usize) {
        let shared = Self::shared_mut();
        shared.early_start = start;
        shared.early_end = start + len;
    }

    pub(crate) unsafe fn init(via: InitializationSource) {
        let shared = Self::shared_mut();

        let free_count;
        match via {
            InitializationSource::DeviceTree(dt) => {
                free_count = Self::_init_dt(dt).unwrap();
            }
            InitializationSource::Uefi(_) => {
                todo!()

                // let mm: &[BootMemoryMapDescriptor] =
                //     slice::from_raw_parts(info.mmap_base as usize as *const _, info.mmap_len as usize);

                // let mut list = shared.mem_list.lock();
                // for mem_desc in mm {
                //     if mem_desc.mem_type == BootMemoryType::Available {
                //         let size = mem_desc.page_count as usize * Self::PAGE_SIZE_MIN;
                //         list.push(MemFreePair::new(mem_desc.base.into(), size))
                //             .unwrap();
                //         free_count += size;
                //     }
                // }
                // shared.n_fragments.store(list.len(), Ordering::Release);
                // drop(list);

                // shared.reserved_memory_size = info.total_memory_size as usize - free_count;

                // if cfg!(any(target_arch = "x86_64")) {
                //     shared.real_bitmap = info.real_bitmap;
                // }
            }
        }

        shared.free_pages.store(free_count, Ordering::SeqCst);

        // shared.slab = Some(Box::new(SlabAllocator::new()));

        // shared.fifo.write(EventQueue::new(100));
    }

    #[inline(never)]
    unsafe fn _init_dt(dt: &dt::DeviceTree) -> Result<usize, ()> {
        return Ok(0);
        let stdout = System::stdout();

        let shared = Self::shared();

        let mut free_count = 0;

        let dt_ptr = dt.header() as *const _ as usize;
        writeln!(
            stdout,
            "DeviceTree {:012x}-{:012x}",
            dt_ptr,
            dt_ptr + dt.header().total_size(),
        )
        .unwrap();

        for item in dt.header().reserved_maps().into_iter() {
            writeln!(
                stdout,
                "RESERVED {:012x}-{:012x}",
                item.0,
                item.0 + item.1 - 1
            )
            .unwrap();
        }

        let mut list = shared.mem_list.lock();
        for range in dt.memory_ranges().unwrap() {
            let (base, size) = range;
            writeln!(
                stdout,
                "before: {:012x}-{:012x} ({})",
                base.as_u64(),
                (base + size - 1).as_u64(),
                size >> 12
            )
            .unwrap();
            let (base, size) = arch::fix_memlist(base, size);
            writeln!(
                stdout,
                "after_: {:012x}-{:012x} ({})",
                base.as_u64(),
                (base + size - 1).as_u64(),
                size >> 12
            )
            .unwrap();
            if size > 0 {
                list.push(MemFreePair::new(base, size)).unwrap();
            }
            free_count += size;
        }
        shared.n_fragments.store(list.len(), Ordering::Release);
        drop(list);

        Ok(free_count)
    }

    #[inline]
    unsafe fn shared_mut() -> &'static mut Self {
        MM.get_mut()
    }

    #[inline]
    fn shared() -> &'static Self {
        unsafe { &*MM.get() }
    }

    #[inline]
    pub unsafe fn mmap(_request: MemoryMapRequest) -> Option<NonZeroUsize> {
        unimplemented!()
        // if Scheduler::is_enabled() {
        //     let fifo = &*Self::shared().fifo.as_ptr();
        //     let event = Arc::new(AsyncMmapRequest {
        //         request,
        //         result: AtomicUsize::new(0),
        //         sem: Semaphore::new(0),
        //     });
        //     match fifo.post(event.clone()) {
        //         Ok(_) => (),
        //         Err(_) => todo!(),
        //     }
        //     event.sem.wait();
        //     NonZeroUsize::new(event.result.load(Ordering::SeqCst))
        // } else {
        //     NonZeroUsize::new(PageManager::mmap(request))
        // }
    }

    #[inline]
    pub fn page_size_min(&self) -> usize {
        self.page_size_min
    }

    #[inline]
    pub fn reserved_memory_size() -> usize {
        let shared = Self::shared();
        shared.reserved_memory_size
    }

    #[inline]
    pub fn free_memory_size() -> usize {
        let shared = Self::shared();
        shared.free_pages.load(Ordering::Acquire)
    }

    ///
    /// # SAFETY
    ///
    /// THREAD UNSAFE
    pub unsafe fn early_alloc(layout: Layout) -> Option<NonNullPhysicalAddress> {
        let shared = Self::shared_mut();
        let start = shared
            .early_start
            .rounding_up(usize::max(0x1000, layout.align()));
        let new_start = start + layout.size();
        if new_start <= shared.early_end {
            shared.early_start = new_start;
            NonNullPhysicalAddress::new(start)
        } else {
            None
        }
    }

    /// Allocate pages
    #[must_use]
    pub unsafe fn pg_alloc(layout: Layout) -> Option<NonNullPhysicalAddress> {
        if layout.align() > Self::PAGE_SIZE_MIN {
            return None;
        }
        let shared = Self::shared();
        let align_m1 = Self::PAGE_SIZE_MIN - 1;
        let size = (layout.size() + align_m1) & !(align_m1);

        let list = shared.mem_list.lock();
        for pair in list.as_slice() {
            match pair.alloc(size) {
                Ok(v) => {
                    shared.free_pages.fetch_sub(size, Ordering::SeqCst);
                    return NonNullPhysicalAddress::new(v);
                }
                Err(_) => (),
            }
        }

        None
    }

    pub unsafe fn pg_dealloc(base: PhysicalAddress, layout: Layout) {
        let shared = Self::shared();
        let align_m1 = Self::PAGE_SIZE_MIN - 1;
        let size = (layout.size() + align_m1) & !(align_m1);
        let new_entry = MemFreePair::new(base, size);
        shared.free_pages.fetch_add(size, Ordering::Relaxed);

        let mut list = shared.mem_list.lock();
        shared.n_fragments.store(list.len(), Ordering::Release);
        for pair in list.as_slice() {
            if pair.try_merge(new_entry).is_ok() {
                return;
            }
        }
        match list.push(new_entry) {
            Ok(_) => {
                drop(list);
                return;
            }
            Err(_) => {
                shared.lost_size.fetch_add(size, Ordering::SeqCst);
            }
        }
    }

    #[must_use]
    pub unsafe fn alloc_pages(size: usize) -> Option<NonNullPhysicalAddress> {
        let result = Self::pg_alloc(Layout::from_size_align_unchecked(size, Self::PAGE_SIZE_MIN));
        if let Some(p) = result {
            let p = p.get().direct_mapped::<c_void>();
            p.write_bytes(0, size);
        }
        result
    }

    #[inline]
    #[must_use]
    pub unsafe fn alloc_dma<T>(len: usize) -> Option<(PhysicalAddress, *mut T)> {
        Self::alloc_pages(size_of::<T>() * len).map(|v| {
            let pa = v.get();
            (pa, pa.direct_mapped())
        })
    }

    /// Allocate kernel memory
    #[must_use]
    pub unsafe fn zalloc(layout: Layout) -> Option<NonZeroUsize> {
        let shared = Self::shared();
        if let Some(slab) = &shared.slab {
            match slab.alloc(layout) {
                Ok(result) => return Some(result),
                Err(AllocationError::Unsupported) => (),
                Err(_err) => return None,
            }
        }
        Self::zalloc2(layout)
    }

    #[must_use]
    pub unsafe fn zalloc2(layout: Layout) -> Option<NonZeroUsize> {
        Self::pg_alloc(layout)
            .and_then(|v| NonZeroUsize::new(v.get().direct_mapped::<c_void>() as usize))
            .map(|v| {
                LAST_ALLOC_PTR.store(v.get(), core::sync::atomic::Ordering::SeqCst);
                v
            })
    }

    #[inline]
    pub fn last_alloc_ptr() -> usize {
        LAST_ALLOC_PTR.load(core::sync::atomic::Ordering::Relaxed)
    }

    /// Deallocate kernel memory
    pub unsafe fn zfree(
        base: Option<NonZeroUsize>,
        layout: Layout,
    ) -> Result<(), DeallocationError> {
        if let Some(base) = base {
            (base.get() as *mut u8).write_bytes(0xCC, layout.size());

            let shared = Self::shared();
            if let Some(slab) = &shared.slab {
                if slab.free(base, layout).is_ok() {
                    return Ok(());
                }
            }
            // Self::pg_dealloc(PageManager::direct_unmap(base.get()), layout);
            Ok(())
        } else {
            Ok(())
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct MemFreePair(u64);

#[allow(dead_code)]
impl MemFreePair {
    const PAGE_SIZE: usize = 0x1000;

    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    #[inline]
    pub const fn new(base: PhysicalAddress, size: usize) -> Self {
        let base = base.as_u64() / Self::PAGE_SIZE as u64;
        let size = (size / Self::PAGE_SIZE) as u64;
        Self(base | (size << 32))
    }

    #[inline]
    fn inner(&self) -> &AtomicU64 {
        unsafe { transmute(&self.0) }
    }

    #[inline]
    pub fn raw(&self) -> u64 {
        self.inner().load(Ordering::SeqCst)
    }

    #[inline]
    fn split(data: u64) -> (PhysicalAddress, usize) {
        (
            PhysicalAddress::new(data & 0xFFFF_FFFF),
            (data >> 32) as usize,
        )
    }

    #[inline]
    pub fn base(&self) -> PhysicalAddress {
        Self::split(self.raw()).0 * Self::PAGE_SIZE
    }

    #[inline]
    pub fn size(&self) -> usize {
        Self::split(self.raw()).1 * Self::PAGE_SIZE
    }

    #[inline]
    pub fn try_merge(&self, other: Self) -> Result<(), ()> {
        let p = self.inner();
        p.fetch_update(Ordering::SeqCst, Ordering::Relaxed, |data| {
            let (base0, size0) = Self::split(data);
            let (base1, size1) = Self::split(other.raw());
            if base0 + size0 == base1 {
                Some((base0.as_u64()) | (((size0 + size1) as u64) << 32))
            } else if base1 + size1 == base0 {
                Some((base1.as_u64()) | (((size0 + size1) as u64) << 32))
            } else {
                None
            }
        })
        .map(|_| ())
        .map_err(|_| ())
    }

    #[inline]
    pub fn alloc(&self, size: usize) -> Result<PhysicalAddress, ()> {
        let size = (size + Self::PAGE_SIZE - 1) / Self::PAGE_SIZE;
        let p = self.inner();
        p.fetch_update(Ordering::SeqCst, Ordering::Relaxed, |data| {
            let (base, limit) = Self::split(data);
            if limit < size {
                return None;
            }
            let new_size = limit - size;
            let new_data = ((base + size).as_u64()) | ((new_size as u64) << 32);
            Some(new_data)
        })
        .map(|data| Self::split(data).0 * Self::PAGE_SIZE)
        .map_err(|_| ())
    }
}

bitflags! {
    pub struct MProtect: usize {
        const READ  = 0x4;
        const WRITE = 0x2;
        const EXEC  = 0x1;
        const NONE  = 0x0;

        const READ_WRITE = Self::READ.bits | Self::WRITE.bits;
        const READ_EXEC = Self::READ.bits | Self::WRITE.bits;
    }
}

#[derive(Debug, Copy, Clone)]
pub enum AllocationError {
    Unexpected,
    OutOfMemory,
    InvalidArgument,
    Unsupported,
}

#[derive(Debug, Copy, Clone)]
pub enum DeallocationError {
    Unexpected,
    InvalidArgument,
    Unsupported,
}

#[derive(Debug, Clone, Copy)]
pub enum MemoryMapRequest {
    /// for MMIO (physical_address, length)
    Mmio(PhysicalAddress, usize),
    /// for VRAM (physical_address, length)
    Vram(PhysicalAddress, usize),
    /// for Kernel Mode Heap (base, length, attr)
    Kernel(usize, usize, MProtect),
    /// for User Mode Heap (base, length, attr)
    User(usize, usize, MProtect),
}
