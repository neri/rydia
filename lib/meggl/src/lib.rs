//! MEG-OS Standard Graphics Drawing Library
#![no_std]
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
#![feature(const_swap)]

extern crate alloc;

mod bitmap;
mod canvas;
mod color;
mod coords;
mod drawable;

pub use bitmap::*;
pub use canvas::*;
pub use color::*;
pub use coords::*;
pub use drawable::*;

pub mod image;
