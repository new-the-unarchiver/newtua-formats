//! CP/M Crunch (`.?Z?`) — the standalone LZW compressor, type `0xfe`.
//!
//! This is a different algorithm from the ARC-era [`crunch`](crate::crunch)
//! module: it is the CP/M `CRUNCH` utility's adaptive LZW, with a 4096-entry
//! string table indexed through a separate hash table, variable-width codes
//! (9–12 bits) in the "new" variant or fixed 12-bit codes in the "old" one,
//! and special reset / filler codes. The LZW output is then run through RLE90
//! ("type 2"), and an optional trailing byte-sum checksum guards the result.
//!
//! Faithful port of XADMaster's `XADCrunchHandles.m` (`CRUNCHuncrunch` and the
//! `XADCrunchZHandle` wiring). The LZHUF variant (type `0xfd`) is a separate
//! roadmap item and is recognized but not decoded here.

use std::io::{self, Read, Write};

use newtua_common::bitreader::BitReaderMsb;
use newtua_common::rle90::Rle90Reader;

const TABLE_SIZE: usize = 4096; // main LZW table, for 12-bit codes
const XLATBL_SIZE: usize = 5003; // auxiliary physical translation (hash) table

const NOPRED: u16 = 0x3fff; // no predecessor in table
const EMPTY: u16 = 0x8000; // empty table entry
const REFERENCED: u16 = 0x2000; // entry referenced (OR'd into predecessor)
const IMPRED: u16 = 0x7fff; // impossible predecessor (reserved codes)

const EOFCOD: u16 = 0x100; // end of file
const RSTCOD: u16 = 0x101; // adaptive reset
const NULCOD: u16 = 0x102; // filler
const SPRCOD: u16 = 0x103; // spare

fn decrunch(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg)
}

/// Streaming CP/M Crunch LZW decoder. Yields the *pre-RLE90* byte stream; wrap
/// it in [`newtua_common::rle90::Rle90Reader::new_type2`] for the final output.
pub struct CrunchCpmReader<R> {
    bits: BitReaderMsb<R>,
    old: bool,

    table_pred: Vec<u16>,
    table_suffix: Vec<u8>,
    xlatbl: Vec<u16>,
    stack: Vec<u8>,

    entry: u16,
    codlen: u8,
    fulflg: u8,
    entflg: u8,
    finchar: u8,
    lastpr: u16,
    pred: u16,

    out: Vec<u8>,
    out_pos: usize,
    done: bool,
}

impl<R: Read> CrunchCpmReader<R> {
    /// Wrap `inner`, selecting the "old" (fixed 12-bit) or "new" (variable
    /// 9–12-bit) variant.
    pub fn new(inner: R, old: bool) -> io::Result<Self> {
        let mut this = CrunchCpmReader {
            bits: BitReaderMsb::new(inner),
            old,
            table_pred: vec![EMPTY; TABLE_SIZE],
            table_suffix: vec![0; TABLE_SIZE],
            xlatbl: vec![EMPTY; XLATBL_SIZE],
            stack: vec![0; TABLE_SIZE],
            entry: 0,
            codlen: 9,
            fulflg: 0,
            entflg: 1,
            finchar: 0,
            lastpr: NOPRED,
            pred: NOPRED,
            out: Vec::new(),
            out_pos: 0,
            done: false,
        };
        if old {
            this.init_old()?;
        } else {
            this.init_new()?;
        }
        Ok(this)
    }

    // --- new variant ------------------------------------------------------

    /// `CRUNCHinitb2`: reset both tables and enter the atomic + reserved codes.
    fn init_new(&mut self) -> io::Result<()> {
        self.entry = 0;
        self.fulflg = 0;
        self.codlen = 9;
        self.entflg = 1;
        self.xlatbl.iter_mut().for_each(|x| *x = EMPTY);
        for i in 0..0x100u16 {
            self.enterx(NOPRED, i as u8)?;
        }
        for _ in 0..4 {
            self.enterx(IMPRED, 0)?;
        }
        Ok(())
    }

    /// Index into `xlatbl` for `(pred, suff)` in the new variant.
    fn disp(pred: u16, suff: u8) -> i32 {
        (((((pred >> 4) & 0xff) ^ u16::from(suff)) | ((pred & 0xf) << 8)) as i32) + 1
    }

    /// Advance an `xlatbl` probe and wrap it back into range, mirroring the
    /// pointer wrap in the C source (`p += XLATBL_SIZE` on overrun).
    fn next_probe(p: i32, step: i32) -> i32 {
        let p = p + step;
        if (0..=XLATBL_SIZE as i32).contains(&p) {
            p
        } else {
            p + XLATBL_SIZE as i32
        }
    }

    /// `CRUNCHenterx`: append `(pred, suff)` at the next table slot and hash it
    /// into `xlatbl`, growing the code width as the table fills.
    fn enterx(&mut self, pred: u16, suff: u8) -> io::Result<()> {
        let e = self.entry as usize;
        if e >= TABLE_SIZE {
            return Err(decrunch("crunch: table overflow"));
        }
        let mut p = Self::disp(pred, suff);
        let step = p - XLATBL_SIZE as i32;
        let mut guard = 0;
        while self.xlatbl[p as usize] != EMPTY {
            p = Self::next_probe(p, step);
            guard += 1;
            if guard > XLATBL_SIZE || !(0..XLATBL_SIZE as i32).contains(&p) {
                return Err(decrunch("crunch: hash chain corrupt"));
            }
        }
        self.xlatbl[p as usize] = self.entry;

        self.table_pred[e] = pred;
        self.table_suffix[e] = suff;
        self.entry += 1;

        // The main loop reads one code ahead, so widen as soon as only one code
        // of the current width remains.
        if self.entry as usize >= (1usize << self.codlen) - 1 {
            if self.codlen < 12 {
                self.codlen += 1;
            } else {
                self.fulflg += 1;
            }
        }
        Ok(())
    }

    /// `CRUNCHentfil`: when the table is full, reassign a defined-but-never-
    /// referenced code that hashes from `(pred, suff)`.
    fn entfil(&mut self, pred: u16, suff: u8) {
        let mut p = Self::disp(pred, suff);
        let step = p - XLATBL_SIZE as i32;
        while self.xlatbl[p as usize] != EMPTY {
            let cand = self.xlatbl[p as usize] as usize;
            if self.table_pred[cand] & REFERENCED == 0 {
                self.table_pred[cand] = pred;
                self.table_suffix[cand] = suff;
                break;
            }
            p = Self::next_probe(p, step);
            if !(0..XLATBL_SIZE as i32).contains(&p) {
                break;
            }
        }
    }

    /// `CRUNCHdecode`: emit the byte string for `code`, returning the previous
    /// `entflg`. Appends decoded bytes to `self.out`.
    fn decode(&mut self, code: u16) -> io::Result<u8> {
        let code = code as usize;
        if code >= self.entry as usize {
            // The "WsWsW" exception: the code names the entry we are about to
            // create.
            self.entflg = 1;
            self.enterx(self.lastpr, self.finchar)?;
        }
        self.table_pred[code] |= REFERENCED;

        let mut sp = 0;
        let mut ep = code;
        while ep > 255 {
            if sp >= TABLE_SIZE {
                return Err(decrunch("crunch: code string too long"));
            }
            self.stack[sp] = self.table_suffix[ep];
            sp += 1;
            ep = (self.table_pred[ep] & 0xFFF) as usize;
        }
        self.finchar = self.table_suffix[ep];
        self.out.push(self.finchar);
        while sp > 0 {
            sp -= 1;
            self.out.push(self.stack[sp]);
        }
        Ok(self.entflg)
    }

    /// One iteration of the new-variant main loop. Returns `false` at EOF.
    fn step_new(&mut self) -> io::Result<bool> {
        self.lastpr = self.pred;
        let code = match self.bits.read(self.codlen)? {
            Some(c) => c as u16,
            None => return Err(decrunch("crunch: truncated stream (no EOF code)")),
        };
        self.pred = code;
        match code {
            EOFCOD => return Ok(false),
            RSTCOD => {
                self.pred = NOPRED;
                self.init_new()?;
            }
            NULCOD | SPRCOD => self.pred = self.lastpr,
            _ => {
                if self.fulflg != 2 {
                    if self.decode(code)? == 0 {
                        self.enterx(self.lastpr, self.finchar)?;
                    } else {
                        self.entflg = 0;
                    }
                } else {
                    self.decode(code)?;
                    self.entfil(self.lastpr, self.finchar);
                }
            }
        }
        Ok(true)
    }

    // --- old variant ------------------------------------------------------

    /// Initialise the tables for the old (fixed 12-bit) variant.
    fn init_old(&mut self) -> io::Result<()> {
        self.entry = 0;
        self.xlatbl.iter_mut().for_each(|x| *x = EMPTY);
        self.table_pred[0] = NOPRED;
        for i in 1..TABLE_SIZE {
            self.table_pred[i] = EMPTY;
        }
        for i in 0..0x100u16 {
            self.enterx_old(NOPRED, i as u8)?;
        }
        Ok(())
    }

    /// `CRUNCHenterxOLD`: the old variant's quadratic hash placement.
    fn enterx_old(&mut self, pred: u16, suff: u8) -> io::Result<()> {
        let mut hashval: i32 = if pred == NOPRED && suff == 0 {
            0x800
        } else {
            let a = ((i32::from(pred) + i32::from(suff)) | 0x800) & 0x1FFF;
            let h = a >> 1;
            ((h * (h + (a & 1))) >> 4) & 0xfff
        };

        while self.xlatbl[hashval as usize] != EMPTY {
            hashval = self.xlatbl[hashval as usize] as i32;
        }
        if hashval >= TABLE_SIZE as i32 {
            return Err(decrunch("crunch: hash out of range"));
        }

        if self.table_pred[hashval as usize] != EMPTY {
            let lasthash = hashval as usize;
            hashval = (hashval + 101) & 0xfff;
            let mut a = 0;
            while self.table_pred[hashval as usize] != EMPTY && a < TABLE_SIZE as i32 {
                hashval = (hashval + 1) & 0xfff;
                a += 1;
            }
            self.xlatbl[lasthash] = hashval as u16;
        }

        self.table_pred[hashval as usize] = pred;
        self.table_suffix[hashval as usize] = suff;
        self.entry += 1;
        Ok(())
    }

    /// One iteration of the old-variant main loop. Returns `false` at EOF.
    fn step_old(&mut self) -> io::Result<bool> {
        self.lastpr = self.pred;
        let code = match self.bits.read(12)? {
            Some(c) => c as u16,
            None => return Err(decrunch("crunch: truncated stream (no EOF code)")),
        };
        self.pred = code;
        if code == 0 {
            return Ok(false); // old variant's EOF is code 0
        }

        let start = if self.table_pred[code as usize] == EMPTY {
            self.lastpr
        } else {
            code
        };
        let mut ep = start as usize;
        if ep >= TABLE_SIZE {
            return Err(decrunch("crunch: bad code"));
        }

        let mut sp = 0;
        while self.table_pred[ep] < TABLE_SIZE as u16 {
            if sp >= TABLE_SIZE - 2 {
                return Err(decrunch("crunch: code string too long"));
            }
            self.stack[sp] = self.table_suffix[ep];
            sp += 1;
            ep = self.table_pred[ep] as usize;
        }
        if self.table_pred[ep] != EMPTY {
            self.stack[sp] = self.table_suffix[ep];
            sp += 1;
        }
        if sp == 0 {
            return Err(decrunch("crunch: empty code expansion"));
        }
        self.finchar = self.stack[sp - 1];

        while sp > 0 {
            sp -= 1;
            self.out.push(self.stack[sp]);
        }
        if self.table_pred[code as usize] == EMPTY {
            self.out.push(self.finchar);
        }

        if (self.entry as usize) < TABLE_SIZE - 1 && self.lastpr != NOPRED {
            self.enterx_old(self.lastpr, self.finchar)?;
        }
        Ok(true)
    }
}

impl<R: Read> Read for CrunchCpmReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut written = 0;
        while written < buf.len() {
            if self.out_pos < self.out.len() {
                buf[written] = self.out[self.out_pos];
                self.out_pos += 1;
                written += 1;
                continue;
            }
            if self.done {
                break;
            }
            self.out.clear();
            self.out_pos = 0;
            let more = if self.old {
                self.step_old()?
            } else {
                self.step_new()?
            };
            if !more {
                self.done = true;
            }
        }
        Ok(written)
    }
}

// --- container ------------------------------------------------------------

/// The single member of a Crunch file.
pub struct CrunchEntry {
    name: Vec<u8>,
    compression_name: &'static str,
}

impl CrunchEntry {
    /// The stored file name as raw bytes (charset decoding is the caller's job).
    pub fn name(&self) -> &[u8] {
        &self.name
    }
    /// Human-readable codec name, e.g. `"LZW 2.0"` or `"LZHUF 1.0"`.
    pub fn compression_name(&self) -> &str {
        self.compression_name
    }
}

/// A parsed Crunch file. Crunch is a single-file compressor, so there is always
/// exactly one entry.
pub struct CrunchArchive {
    data: Vec<u8>,
    entry: CrunchEntry,
    data_offset: usize,
    old: bool,
    haschecksum: bool,
    crunch_type: u8,
}

/// Build a member's name from the header c-string, mirroring `XADCrunchParser`:
/// truncate at the first `.` plus a 3-char extension, trimming trailing spaces;
/// with no `.`, keep the whole string.
fn extract_name(cstr: &[u8]) -> Vec<u8> {
    let length = cstr.len();
    for i in 0..length {
        if cstr[i] == b'.' {
            let mut namelength = i + 4;
            for _ in 0..3 {
                if namelength > length || cstr[namelength - 1] == b' ' {
                    namelength -= 1;
                }
            }
            return cstr[..namelength.min(length)].to_vec();
        }
    }
    cstr.to_vec()
}

impl CrunchArchive {
    /// Recognizer mirroring `XADCrunchParser`: the `0x76` magic, a Crunch type
    /// byte, a non-empty name, and version bytes in `0x10..=0x2f`.
    pub fn recognize(data: &[u8]) -> bool {
        let length = data.len();
        if length < 9 {
            return false;
        }
        if data[0] != 0x76 || (data[1] != 0xfe && data[1] != 0xfd) {
            return false;
        }
        if data[2] == 0 {
            return false;
        }
        for i in 2..length {
            if data[i] == 0 {
                if i + 4 > length {
                    return false;
                }
                if !(0x10..=0x2f).contains(&data[i + 1]) {
                    return false;
                }
                if !(0x10..=0x2f).contains(&data[i + 2]) {
                    return false;
                }
                return true;
            }
        }
        false
    }

    /// Parse the header of a Crunch file read from `r`.
    pub fn open<R: Read>(mut r: R) -> io::Result<Self> {
        let mut data = Vec::new();
        r.read_to_end(&mut data)?;
        let len = data.len();
        if len < 9 || data[0] != 0x76 {
            return Err(decrunch("crunch: not a Crunch file"));
        }
        let crunch_type = data[1];
        if crunch_type != 0xfe && crunch_type != 0xfd {
            return Err(decrunch("crunch: unknown type"));
        }
        if data[2] == 0 {
            return Err(decrunch("crunch: empty name"));
        }

        let mut n0 = 2;
        while n0 < len && data[n0] != 0 {
            n0 += 1;
        }
        // Need the NUL plus four header bytes (version1, version2,
        // errordetection, reserved) after the name.
        if n0 >= len || n0 + 5 > len {
            return Err(decrunch("crunch: truncated header"));
        }

        let version2 = data[n0 + 2];
        let errordetection = data[n0 + 3];
        let data_offset = n0 + 5;
        let old = (version2 & 0xf0) == 0x10;
        let haschecksum = errordetection == 0;
        let name = extract_name(&data[2..n0]);

        let compression_name = match (crunch_type, old) {
            (0xfe, false) => "LZW 2.0",
            (0xfe, true) => "LZW 1.0",
            (_, false) => "LZHUF 2.0",
            (_, true) => "LZHUF 1.0",
        };

        Ok(Self {
            data,
            entry: CrunchEntry {
                name,
                compression_name,
            },
            data_offset,
            old,
            haschecksum,
            crunch_type,
        })
    }

    /// The single member, as a one-element slice (matching the other formats).
    pub fn entries(&self) -> &[CrunchEntry] {
        std::slice::from_ref(&self.entry)
    }

    /// Decode the member and write it to `out`. LZW (`0xfe`) is decoded through
    /// RLE90 ("type 2") with optional checksum verification; LZHUF (`0xfd`) is
    /// a separate roadmap item and returns [`io::ErrorKind::Unsupported`].
    pub fn read_entry(&self, idx: usize, out: &mut dyn Write) -> io::Result<()> {
        if idx != 0 {
            return Err(decrunch("crunch: index out of range"));
        }
        if self.crunch_type != 0xfe {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "crunch: LZHUF (type 0xfd) decoding is a separate roadmap item",
            ));
        }

        // The compressed data runs to the end, less the 2-byte checksum if any.
        let end = if self.haschecksum {
            self.data
                .len()
                .checked_sub(2)
                .filter(|&e| e >= self.data_offset)
                .ok_or_else(|| decrunch("crunch: truncated (no checksum)"))?
        } else {
            self.data.len()
        };
        let comp = &self.data[self.data_offset..end];

        let lzw = CrunchCpmReader::new(comp, self.old)?;
        let mut rle = Rle90Reader::new_type2(lzw);
        let mut decoded = Vec::new();
        rle.read_to_end(&mut decoded)?;

        if self.haschecksum {
            let sum: u32 = decoded.iter().map(|&b| u32::from(b)).sum();
            let n = self.data.len();
            let correct = u16::from_le_bytes([self.data[n - 2], self.data[n - 1]]);
            if (sum & 0xffff) as u16 != correct {
                return Err(decrunch("crunch: checksum mismatch"));
            }
        }

        out.write_all(&decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decode a raw "new"-variant LZW stream (pre-RLE90 bytes).
    fn decode_new(stream: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        CrunchCpmReader::new(stream, false)
            .unwrap()
            .read_to_end(&mut out)
            .unwrap();
        out
    }

    #[test]
    fn decodes_two_literals() {
        // Codes 0x41 'A', 0x42 'B', EOF 0x100, 9 bits each, MSB-first.
        assert_eq!(decode_new(&[0x20, 0x90, 0xA0, 0x00]), b"AB");
    }

    #[test]
    fn decodes_backreference() {
        // 'A', 'B', then code 260 = the freshly-entered string "AB", then EOF.
        assert_eq!(decode_new(&[0x20, 0x90, 0xA0, 0x90, 0x00]), b"ABAB");
    }

    #[test]
    fn decodes_empty_stream() {
        // Just the EOF code 0x100.
        assert_eq!(decode_new(&[0x80, 0x00]), b"");
    }

    #[test]
    fn emits_raw_0x90_before_rle90() {
        // Atomic code 0x90, then EOF: the decoder yields a literal 0x90 byte —
        // RLE90 interpretation happens in the wrapping layer, not here.
        assert_eq!(decode_new(&[0x48, 0x40, 0x00]), vec![0x90]);
    }

    #[test]
    fn truncated_stream_without_eof_errors() {
        // Fewer than one code's worth of bits and no EOF code.
        let mut out = Vec::new();
        let err = CrunchCpmReader::new(&[0x20][..], false)
            .unwrap()
            .read_to_end(&mut out)
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    // --- container --------------------------------------------------------

    /// The new-variant LZW body that decodes (after identity RLE90) to "AB".
    const AB_BODY: &[u8] = &[0x20, 0x90, 0xA0, 0x00];

    /// Assemble a Crunch file: magic, type, NUL-terminated name, the four
    /// header bytes, the compressed body, and an optional trailing checksum.
    fn container(
        ctype: u8,
        name: &[u8],
        version2: u8,
        errordetection: u8,
        body: &[u8],
        checksum: Option<u16>,
    ) -> Vec<u8> {
        let mut v = vec![0x76, ctype];
        v.extend_from_slice(name);
        v.push(0);
        v.push(0x20); // version1
        v.push(version2);
        v.push(errordetection);
        v.push(0); // reserved
        v.extend_from_slice(body);
        if let Some(c) = checksum {
            v.extend_from_slice(&c.to_le_bytes());
        }
        v
    }

    fn read0(arc: &CrunchArchive) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        arc.read_entry(0, &mut out)?;
        Ok(out)
    }

    #[test]
    fn recognizes_valid_header() {
        let data = container(0xfe, b"AB.TXT", 0x20, 1, AB_BODY, None);
        assert!(CrunchArchive::recognize(&data));
    }

    #[test]
    fn rejects_short_and_bad_magic() {
        assert!(!CrunchArchive::recognize(&[0u8; 4]));
        let mut data = container(0xfe, b"AB.TXT", 0x20, 1, AB_BODY, None);
        data[0] = 0x00;
        assert!(!CrunchArchive::recognize(&data));
    }

    #[test]
    fn rejects_bad_version_bytes() {
        // version2 outside 0x10..=0x2f must fail recognition.
        let data = container(0xfe, b"AB.TXT", 0x40, 1, AB_BODY, None);
        assert!(!CrunchArchive::recognize(&data));
    }

    #[test]
    fn parses_name_and_extracts_stored_member() {
        let data = container(0xfe, b"AB.TXT", 0x20, 1, AB_BODY, None);
        let arc = CrunchArchive::open(&data[..]).unwrap();
        let e = arc.entries();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].name(), b"AB.TXT");
        assert_eq!(read0(&arc).unwrap(), b"AB");
    }

    #[test]
    fn reports_compression_variant_from_version() {
        let new = container(0xfe, b"AB.TXT", 0x20, 1, AB_BODY, None);
        assert_eq!(
            CrunchArchive::open(&new[..]).unwrap().entries()[0].compression_name(),
            "LZW 2.0"
        );
        // High nibble 0x10 marks the old variant.
        let old = container(0xfe, b"AB.TXT", 0x10, 1, AB_BODY, None);
        assert_eq!(
            CrunchArchive::open(&old[..]).unwrap().entries()[0].compression_name(),
            "LZW 1.0"
        );
    }

    #[test]
    fn name_without_dot_is_whole_string() {
        let data = container(0xfe, b"README", 0x20, 1, AB_BODY, None);
        let arc = CrunchArchive::open(&data[..]).unwrap();
        assert_eq!(arc.entries()[0].name(), b"README");
    }

    #[test]
    fn verifies_trailing_checksum() {
        // Byte-sum of the decoded output "AB" = 0x41 + 0x42 = 0x83.
        let ok = container(0xfe, b"AB.TXT", 0x20, 0, AB_BODY, Some(0x83));
        assert_eq!(
            read0(&CrunchArchive::open(&ok[..]).unwrap()).unwrap(),
            b"AB"
        );

        let bad = container(0xfe, b"AB.TXT", 0x20, 0, AB_BODY, Some(0x84));
        assert!(read0(&CrunchArchive::open(&bad[..]).unwrap()).is_err());
    }

    #[test]
    fn truncated_body_errors() {
        let data = container(0xfe, b"AB.TXT", 0x20, 1, &AB_BODY[..2], None);
        assert!(read0(&CrunchArchive::open(&data[..]).unwrap()).is_err());
    }

    #[test]
    fn lzhuf_type_is_recognized_but_unsupported() {
        let data = container(0xfd, b"AB.TXT", 0x20, 1, AB_BODY, None);
        assert!(CrunchArchive::recognize(&data));
        let arc = CrunchArchive::open(&data[..]).unwrap();
        assert_eq!(arc.entries()[0].compression_name(), "LZHUF 2.0");
        assert_eq!(read0(&arc).unwrap_err().kind(), io::ErrorKind::Unsupported);
    }
}
