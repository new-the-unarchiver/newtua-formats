# newtua-common

Shared low-level primitives used to build decoders for legacy archive formats.

## What's inside

- **Bit readers** (`bitreader`) — LSB/MSB bit-at-a-time reading over a byte
  stream, the basis for most of the codecs below.
- **Prefix / Huffman codes** (`prefixcode`, `stuffit_huffman`) — general
  prefix-code table construction and decoding, plus the specific Huffman
  variant used by StuffIt.
- **LZSS window** (`lzss`) — a sliding-window match/literal decoder shared by
  the LZSS-family codecs.
- **LZW** (`lzw`) — variable-width LZW decoding as used by ARC, Zoo, and
  Unix `compress`-derived formats.
- **Deflate** (`deflate`) — a from-scratch inflate implementation, including
  the parameterised meta-table order needed by non-standard deflate variants
  (e.g. ALZip's obfuscated deflate, NSIS's modified deflate).
- **RLE90** (`rle90`) — the run-length scheme used by several CP/M and Mac
  formats.
- **Checksums** (`crc16`, `crc32`) — CRC variants used across the format
  family.
- **Crypto** (`md5`, `rc4`, `zipcrypt`) — MD5, RC4, and traditional PKWARE
  ZipCrypto, used for the family's encrypted-archive support.
- **Byte helpers** (`bytes`) and a small `compress` module of shared utilities.

## Installation

```sh
cargo add newtua-common
```

This crate is a set of building blocks, not an end-user API — see the format
crates below for `open`/`entries`/`read_entry`-style usage.

## Part of newtua-formats

`newtua-common` is one of the crates in
**[newtua-formats](https://github.com/new-the-unarchiver/newtua-formats)** — a
family of pure-Rust decoders for legacy archive formats, ported from The
Unarchiver's XADMaster. It is the shared foundation the other crates
(`newtua-dos`, `newtua-mac`, `newtua-stuffit`, `newtua-amiga`, `newtua-alz`,
`newtua-nsis`) build on.

`newtua-formats` is in turn the legacy-formats layer of
**[New The Unarchiver](https://github.com/new-the-unarchiver)** (`newtua`) — a
cross-platform archive extractor written in Rust, a modern rewrite of the
macOS tool The Unarchiver. It extracts and lists archives; it never creates
them.

Unlike the format crates, `newtua-common` isn't really meant to be reached for
on its own — its value is as a shared toolbox behind the decoders, not as a
standalone archive reader.

## License

LGPL-3.0-or-later — see [`LICENSE`](https://github.com/new-the-unarchiver/newtua-formats/blob/main/LICENSE) in the repository root.
