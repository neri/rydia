//! codename RYDIA

use crate::{
    arch,
    drawing::*,
    io::{emcon::EmConsole, font::FontManager},
};
use core::cell::UnsafeCell;

static mut SYSTEM: UnsafeCell<System> = UnsafeCell::new(System::new());

pub struct System {
    main_screen: Option<UnsafeCell<Bitmap32<'static>>>,
    em_console: EmConsole,
}

impl System {
    const fn new() -> Self {
        Self {
            main_screen: None,
            em_console: EmConsole::new(FontManager::preferred_console_font()),
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

    pub unsafe fn init() {
        let shared = Self::shared_mut();

        arch::init();

        let main_screen = arch::fb::Fb::init(800, 600).expect("Fb::init failed");
        shared.main_screen = Some(UnsafeCell::new(main_screen));
    }

    #[inline]
    pub fn main_screen() -> Bitmap<'static> {
        unsafe { &mut *Self::shared_mut().main_screen.as_mut().unwrap().get() }.into()
    }

    #[inline]
    pub fn em_console<'a>() -> &'a mut EmConsole {
        unsafe { &mut Self::shared_mut().em_console }
    }
}
