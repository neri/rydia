//! codename RYDIA

use crate::{
    arch,
    drawing::*,
    fw,
    fw::dt,
    io::{emcon::EmConsole, font::FontManager, uart::Uart},
    mem,
};
use core::{
    cell::UnsafeCell,
    fmt::{self},
    ptr::null,
};

static mut SYSTEM: UnsafeCell<System> = UnsafeCell::new(System::new());

pub struct System {
    main_screen: Option<UnsafeCell<Bitmap32<'static>>>,
    em_console: EmConsole,
    device_tree: Option<fw::dt::DeviceTree>,
    #[allow(dead_code)]
    model_name: (*const u8, usize),
}

impl System {
    const SYSTEM_NAME: &'static str = "rydia";
    const SYSTEMN_CODENAME: &'static str = "RYDIA";
    const SYSTEM_SHORT_NAME: &'static str = "rydia";
    const RELEASE: &'static str = "";
    const VERSION: Version<'static> = Version::new(0, 0, 1, Self::RELEASE);

    const fn new() -> Self {
        Self {
            main_screen: None,
            em_console: EmConsole::new(FontManager::preferred_console_font()),
            device_tree: None,
            model_name: (null(), 0),
        }
    }

    #[inline]
    unsafe fn shared_mut<'a>() -> &'a mut Self {
        &mut *SYSTEM.get()
    }

    #[inline]
    #[allow(dead_code)]
    fn shared() -> &'static Self {
        unsafe { &*SYSTEM.get() }
    }

    pub unsafe fn init(dtb: usize) {
        let shared = Self::shared_mut();

        arch::init_early(dtb);

        if dtb != 0 {
            if let Some(dt) = dt::DeviceTree::parse(dtb as *const u8).ok() {
                mem::MemoryManager::init(mem::InitializationSource::DeviceTree(&dt));
                shared.device_tree = Some(dt);
            }
        }

        // let main_screen = arch::fb::Fb::init(1280, 720).expect("Fb::init failed");
        // shared.main_screen = Some(UnsafeCell::new(main_screen));
    }

    /// Returns the name of the current system.
    #[inline]
    pub const fn name() -> &'static str {
        &Self::SYSTEM_NAME
    }

    /// Returns the codename of the current system.
    #[inline]
    pub const fn codename() -> &'static str {
        &Self::SYSTEMN_CODENAME
    }

    /// Returns abbreviated name of the current system.
    #[inline]
    pub const fn short_name() -> &'static str {
        &Self::SYSTEM_SHORT_NAME
    }

    /// Returns the version of the current system.
    #[inline]
    pub const fn version<'a>() -> &'a Version<'a> {
        &Self::VERSION
    }

    #[inline]
    pub fn main_screen() -> Bitmap<'static> {
        unsafe { &mut *Self::shared_mut().main_screen.as_mut().unwrap().get() }.into()
    }

    #[inline]
    pub fn em_console<'a>() -> &'a mut EmConsole {
        unsafe { &mut Self::shared_mut().em_console }
    }

    #[inline]
    pub fn device_tree<'a>() -> Option<&'a fw::dt::DeviceTree> {
        Self::shared().device_tree.as_ref()
    }

    #[inline]
    pub fn stdout<'a>() -> &'a mut dyn Uart {
        arch::std_uart()
    }

    #[inline]
    pub fn model_name<'a>() -> Option<&'a str> {
        Self::device_tree().and_then(|dt| dt.root_model())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version<'a> {
    versions: u32,
    rel: &'a str,
}

impl Version<'_> {
    #[inline]
    pub const fn new<'a>(maj: u8, min: u8, patch: u16, rel: &'a str) -> Version<'a> {
        let versions = ((maj as u32) << 24) | ((min as u32) << 16) | (patch as u32);
        Version { versions, rel }
    }

    #[inline]
    pub const fn as_u32(&self) -> u32 {
        self.versions
    }

    #[inline]
    pub const fn maj(&self) -> usize {
        ((self.versions >> 24) & 0xFF) as usize
    }

    #[inline]
    pub const fn min(&self) -> usize {
        ((self.versions >> 16) & 0xFF) as usize
    }

    #[inline]
    pub const fn patch(&self) -> usize {
        (self.versions & 0xFFFF) as usize
    }

    #[inline]
    pub const fn rel(&self) -> &str {
        &self.rel
    }
}

impl fmt::Display for Version<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.rel().len() > 0 {
            write!(
                f,
                "{}.{}.{}-{}",
                self.maj(),
                self.min(),
                self.patch(),
                self.rel(),
            )
        } else {
            write!(f, "{}.{}.{}", self.maj(), self.min(), self.patch())
        }
    }
}
