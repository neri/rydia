//! codename RYDIA

use crate::{
    arch,
    drawing::*,
    fw,
    fw::dt,
    io::{emcon::EmConsole, font::FontManager},
    mem,
};
use core::{cell::UnsafeCell, ptr::null, slice};

static mut SYSTEM: UnsafeCell<System> = UnsafeCell::new(System::new());

pub struct System {
    main_screen: Option<UnsafeCell<Bitmap32<'static>>>,
    em_console: EmConsole,
    device_tree: Option<fw::dt::DeviceTree>,
    model_name: (*const u8, usize),
}

impl System {
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

        arch::init_minimal();

        if dtb != 0 {
            if let Some(dt) = dt::DeviceTree::parse(dtb as *const u8).ok() {
                mem::MemoryManager::init_first(mem::InitializationSource::DeviceTree(&dt));
                shared.device_tree = Some(dt);
            }
        }

        if let Some(dt) = shared.device_tree.as_ref() {
            for token in dt.header().tokens() {
                match token {
                    dt::Token::BeginNode(name) => {
                        if name != dt::NodeName::ROOT {
                            break;
                        }
                    }
                    dt::Token::Prop(name, ptr, len) => match name {
                        dt::PropName::MODEL => {
                            shared.model_name = (ptr as _, len);
                        }
                        _ => (),
                    },
                    dt::Token::EndNode => break,
                }
            }
        }

        let main_screen = arch::fb::Fb::init(1280, 720).expect("Fb::init failed");
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

    #[inline]
    pub fn device_tree<'a>() -> Option<&'a fw::dt::DeviceTree> {
        Self::shared().device_tree.as_ref()
    }

    #[inline]
    pub fn model_name<'a>() -> Option<&'a str> {
        let shared = Self::shared();
        (shared.model_name.1 > 0)
            .then(|| unsafe {
                let slice = slice::from_raw_parts(shared.model_name.0, shared.model_name.1);
                core::str::from_utf8(slice).ok()
            })
            .flatten()
    }
}
