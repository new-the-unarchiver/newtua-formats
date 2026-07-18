# newtua-dos

Pure-Rust decoders for DOS- and CP/M-era archive formats.

## Formats

- **Squeeze** (`.SQ`) — a single-file compressor from 1981, one of the earliest
  widely used PC archivers; adaptive Huffman coding over a byte stream.
- **ARC** — Vern Buerg's/SEA's 1985 archiver, the format that dominated BBS
  file exchange before ZIP; a flat header list with several storage/packing
  methods per entry.
- **LBR** — a plain "library" container from the CP/M world (early 1980s):
  concatenated files with no compression, just a directory of names and
  offsets.
- **Crunch** — a compressor lineage that grew out of Squeeze, adding LZW
  packing on top of (or instead of) the Huffman stage; this crate covers both
  the classic and CP/M-flavoured variants.
- **Zoo** — Rahul Dhesi's 1986 archiver, an LZW-based competitor to ARC and
  ARJ with its own directory format and per-entry compression method.
- **ARJ** — Robert Jung's early-1990s DOS archiver, notable for its own LZSS
  variant and for staying popular on BBSes well into the ZIP era.

Each format is a container parser plus its own compression method(s), built on
the shared primitives in [`newtua-common`](https://crates.io/crates/newtua-common).

## Installation

```sh
cargo add newtua-dos
```

## Example

Every container in this crate follows the same shape: `open` a reader, list
`entries()`, then decode one by index into any `Write` sink.

```rust
use newtua_dos::zoo::ZooArchive;

fn main() -> std::io::Result<()> {
    let data = std::fs::read("archive.zoo")?;
    let archive = ZooArchive::open(&data[..])?;
    for (idx, entry) in archive.entries().iter().enumerate() {
        let mut out = Vec::new();
        archive.read_entry(idx, &mut out)?;
        println!("{}: {} bytes", String::from_utf8_lossy(entry.name()), out.len());
    }
    Ok(())
}
```

## Part of newtua-formats

`newtua-dos` is one of the crates in
**[newtua-formats](https://github.com/new-the-unarchiver/newtua-formats)** — a
family of pure-Rust decoders for legacy archive formats, ported from The
Unarchiver's XADMaster.

`newtua-formats` is in turn the legacy-formats layer of
**[New The Unarchiver](https://github.com/new-the-unarchiver)** (`newtua`) — a
cross-platform archive extractor written in Rust, a modern rewrite of the
macOS tool The Unarchiver. It extracts and lists archives; it never creates
them.

Every crate here stands on its own: `newtua-dos` needs nothing beyond
`newtua-common` and works fine as a standalone reader for these DOS-era
formats, whether or not you use it through `newtua`.

## License

LGPL-3.0-or-later — see [`LICENSE`](https://github.com/new-the-unarchiver/newtua-formats/blob/main/LICENSE) in the repository root.
