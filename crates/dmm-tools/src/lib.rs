//! SS13 minimap generation tool
#![deny(unsafe_code)] // NB deny rather than forbid, ndarray macros use unsafe

#[macro_use]
extern crate bytemuck;

#[cfg(feature = "gfx_core")]
extern crate gfx_core;

pub mod dmi;
pub mod dmm;
mod icon_cache;
pub mod minimap;
pub mod render_passes;

pub use icon_cache::IconCache;
