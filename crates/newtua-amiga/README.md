# newtua-amiga

Pure-Rust decoders for Amiga compression formats.

## Formats

- **PowerPacker** (`.pp`) — a widely used Amiga single-file compressor from
  the early 1990s, an LZ-based scheme popular for packing executables and
  data files.
- **Amiga LZX** — the Amiga-native archive format built around the LZX
  compression algorithm (later reused for Microsoft CAB/CHM); this crate
  covers the container plus the LZX codec itself.
- **DMS** (DiskMasher) — an Amiga floppy-disk image format from the early
  1990s, supporting all seven of its compression methods (NOCOMP, SIMPLE,
  QUICK, MEDIUM, DEEP, HEAVY1, HEAVY2) plus encrypted disks, and the related
  FMS file-archive variant.

Built on the shared primitives in
[`newtua-common`](https://crates.io/crates/newtua-common).

## Installation

```sh
cargo add newtua-amiga
```

## Example

Each format has its own shape (PowerPacker is a single compressed stream,
LZX and DMS are multi-entry containers). A PowerPacker file decodes in one
step:

```rust
use newtua_amiga::powerpacker::PowerPackerFile;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("data.pp")?;
    let file = PowerPackerFile::open(&data)?;
    let decoded = file.decode()?;
    println!("decoded {} bytes", decoded.len());
    Ok(())
}
```

`lzx::LzxArchive` exposes `open`/`entries`/`read_entry(&entry)`; `dms::DmsArchive`
exposes `open`/`open_with_password`, plus `files()`/`read_file()` for FMS
archives and `read_disk_image()` for a plain disk dump.

## Part of newtua-formats

`newtua-amiga` is one of the crates in
**[newtua-formats](https://github.com/new-the-unarchiver/newtua-formats)** — a
family of pure-Rust decoders for legacy archive formats, ported from The
Unarchiver's XADMaster.

`newtua-formats` is in turn the legacy-formats layer of
**[New The Unarchiver](https://github.com/new-the-unarchiver)** (`newtua`) — a
cross-platform archive extractor written in Rust, a modern rewrite of the
macOS tool The Unarchiver. It extracts and lists archives; it never creates
them.

Every crate here stands on its own: `newtua-amiga` needs nothing beyond
`newtua-common` and works fine as a standalone reader for these Amiga formats,
whether or not you use it through `newtua`.

## License

LGPL-3.0-or-later — see [`LICENSE`](https://github.com/new-the-unarchiver/newtua-formats/blob/main/LICENSE) in the repository root.
