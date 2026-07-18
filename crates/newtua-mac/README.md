# newtua-mac

Pure-Rust decoders for classic Macintosh archive and encoding formats.

## Formats

- **BinHex 4.0** (`.hqx`) — a 1985 ASCII-safe encoding for Mac files (data
  fork + resource fork + Finder metadata), built for transport over 7-bit
  channels like email and early BBSes.
- **MacBinary** (I/II/III) — a binary container that bundles a Mac file's data
  fork, resource fork, and Finder info into one file, standardised from 1985
  onward so non-Mac systems could carry Mac files intact.
- **AppleSingle / AppleDouble** — Apple's own successors to MacBinary (mid/late
  1980s onward): AppleSingle packs both forks and metadata into one file,
  AppleDouble splits them into a visible data file and a companion `._`
  metadata file.
- **Compact Pro** (`.cpt`) — Bill Goodman's 1991 Mac archiver, a popular
  alternative to StuffIt with its own container format and compression.
- **PackIt** (`.pit`) — one of the earliest Mac archivers (mid-1980s),
  predating StuffIt; supports simple password-based obfuscation.

Each format is a container parser plus its codec, built on the shared
primitives in [`newtua-common`](https://crates.io/crates/newtua-common).

## Installation

```sh
cargo add newtua-mac
```

## Example

```rust
use newtua_mac::macbinary::MacBinaryArchive;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("archive.bin")?;
    let archive = MacBinaryArchive::open(&data[..])?;
    let mut out = Vec::new();
    archive.read_entry(0, &mut out)?;
    println!("decoded {} bytes", out.len());
    Ok(())
}
```

## Part of newtua-formats

`newtua-mac` is one of the crates in
**[newtua-formats](https://github.com/new-the-unarchiver/newtua-formats)** — a
family of pure-Rust decoders for legacy archive formats, ported from The
Unarchiver's XADMaster.

`newtua-formats` is in turn the legacy-formats layer of
**[New The Unarchiver](https://github.com/new-the-unarchiver)** (`newtua`) — a
cross-platform archive extractor written in Rust, a modern rewrite of the
macOS tool The Unarchiver. It extracts and lists archives; it never creates
them.

Every crate here stands on its own: `newtua-mac` needs nothing beyond
`newtua-common` and works fine as a standalone reader for these classic-Mac
formats, whether or not you use it through `newtua`.

## License

LGPL-3.0-or-later — see [`LICENSE`](https://github.com/new-the-unarchiver/newtua-formats/blob/main/LICENSE) in the repository root.
