# newtua-alz

Pure-Rust decoder for ALZip (`.alz`) archives.

## Format

**ALZip** is the flat multi-file container produced by the Korean ALZip
compression tool (ESTsoft, early 2000s). Each entry is stored, bzip2-packed,
or deflate-packed (including a byte-obfuscated deflate variant), and members
can be encrypted with traditional PKWARE ZipCrypto. Split (multi-volume)
archives are supported by reassembling their ordered volumes.

Built on the shared primitives in
[`newtua-common`](https://crates.io/crates/newtua-common); bzip2 decoding
uses the pure-Rust `bzip2-rs` crate.

## Installation

```sh
cargo add newtua-alz
```

## Example

```rust
use newtua_alz::AlzArchive;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("archive.alz")?;
    let archive = AlzArchive::open(&data[..])?;
    for (idx, entry) in archive.entries().iter().enumerate() {
        let mut out = Vec::new();
        archive.read_entry(idx, &mut out)?;
        println!("{}: {} bytes", String::from_utf8_lossy(entry.name()), out.len());
    }
    Ok(())
}
```

Encrypted archives use `AlzArchive::open_with_password`; split archives use
`AlzArchive::open_volumes` (or `open_volumes_with_password`).

## Part of newtua-formats

`newtua-alz` is one of the crates in
**[newtua-formats](https://github.com/new-the-unarchiver/newtua-formats)** — a
family of pure-Rust decoders for legacy archive formats, ported from The
Unarchiver's XADMaster.

`newtua-formats` is in turn the legacy-formats layer of
**[New The Unarchiver](https://github.com/new-the-unarchiver)** (`newtua`) — a
cross-platform archive extractor written in Rust, a modern rewrite of the
macOS tool The Unarchiver. It extracts and lists archives; it never creates
them.

Every crate here stands on its own: `newtua-alz` needs nothing beyond
`newtua-common` and works fine as a standalone ALZip reader, whether or not
you use it through `newtua`.

## License

LGPL-3.0-or-later — see [`LICENSE`](https://github.com/new-the-unarchiver/newtua-formats/blob/main/LICENSE) in the repository root.
