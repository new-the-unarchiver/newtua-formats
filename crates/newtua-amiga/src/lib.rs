//! Decoders for Amiga compression formats.
//!
//! Pure-Rust ports of legacy Amiga formats (LGPL-2.1). Modules are added
//! one format at a time, test-first.
//!
//! Formats: PowerPacker, Amiga LZX (container + LZX codec). Planned: DMS
//! (+ libxad bridge).

#![forbid(unsafe_code)]

pub mod lzx;
pub mod powerpacker;
