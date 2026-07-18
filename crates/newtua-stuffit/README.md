# newtua-stuffit

Pure-Rust decoders for the StuffIt family of classic Macintosh archives.

## Formats

- **Classic StuffIt** (`.sit`, signature `SIT!` + `rLau`) — Raymond Lau's
  1987 archiver, the dominant Mac compression format through the 1990s.
- **StuffIt 5** — the 1997 banner-based container format (including its
  self-extracting `.exe` variant), a redesign that added stronger compression
  methods and optional encryption.
- **StuffItX** (`.sitx`) — the post-2002 format, restructured around a
  different container layout with its own codec set and an x86
  preprocessing filter.

All three containers share the same family of compression methods — store,
RLE90, Unix `compress`/LZW, StuffIt-Huffman, LZAH, LZ+Huffman, and Arsenic —
dispatched through shared internal code, plus (for StuffItX) None/Deflate/RC4.
StuffIt 5 also supports RC4+MD5 password-encrypted members. Built on the
shared primitives in [`newtua-common`](https://crates.io/crates/newtua-common).

## Installation

```sh
cargo add newtua-stuffit
```

## Example

```rust
use newtua_stuffit::stuffit::StuffItArchive;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("archive.sit")?;
    let archive = StuffItArchive::open(&data[..])?;
    for (idx, entry) in archive.entries().iter().enumerate() {
        let mut out = Vec::new();
        archive.read_entry(idx, &mut out)?;
        println!("{}: {} bytes", String::from_utf8_lossy(entry.name()), out.len());
    }
    Ok(())
}
```

The StuffIt 5 (`sit5`) and StuffItX (`sitx`) modules follow the same
`open`/`entries`/`read_entry` shape; `sit5` also has `open_with_password` for
encrypted archives.

## Part of newtua-formats

`newtua-stuffit` is one of the crates in
**[newtua-formats](https://github.com/new-the-unarchiver/newtua-formats)** — a
family of pure-Rust decoders for legacy archive formats, ported from The
Unarchiver's XADMaster.

`newtua-formats` is in turn the legacy-formats layer of
**[New The Unarchiver](https://github.com/new-the-unarchiver)** (`newtua`) — a
cross-platform archive extractor written in Rust, a modern rewrite of the
macOS tool The Unarchiver. It extracts and lists archives; it never creates
them.

Every crate here stands on its own: `newtua-stuffit` needs nothing beyond
`newtua-common` and works fine as a standalone reader for the whole StuffIt
lineage, whether or not you use it through `newtua`.

## License

LGPL-3.0-or-later — see [`LICENSE`](https://github.com/new-the-unarchiver/newtua-formats/blob/main/LICENSE) in the repository root.
