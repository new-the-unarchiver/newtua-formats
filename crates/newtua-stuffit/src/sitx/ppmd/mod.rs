//! PPMd engine (`XADMaster/PPMd/`), ported only as far as variant G needs:
//! its own range coder, the Brimstone sub-allocator, the shared model core,
//! and variant G itself. Variant H/I and their allocators are out of scope
//! for this stage (see `task-19g-sitx-brimstone-ppmd.md`).

mod alloc;
mod context;
#[cfg(test)]
mod encoder_mirror;
mod rangecoder;
pub(crate) mod variant_g;
