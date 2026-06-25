//! End-to-end golden test: extract a multi-member `.arc` with our crate AND with
//! the reference `unar`, and assert every member agrees byte-for-byte.
//!
//! The fixture (verified against `unar`) holds a stored, a packed (RLE90), and a
//! squeezed member. Skipped when `unar` is not installed.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use newtua_dos::arc::ArcArchive;
use newtua_testutil::{unar_extract_all, unar_installed};

fn fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read(path).unwrap()
}

#[test]
fn our_decode_matches_unar() {
    if !unar_installed() {
        eprintln!("skipping: `unar` not installed");
        return;
    }

    let data = fixture("multi.arc");
    let arc = ArcArchive::open(&data[..]).unwrap();

    let mut ours = BTreeMap::new();
    for (i, entry) in arc.entries().iter().enumerate() {
        if entry.is_dir() {
            continue;
        }
        let mut out = Vec::new();
        arc.read_entry(i, &mut out).unwrap();
        let name = String::from_utf8(entry.name().to_vec()).unwrap();
        ours.insert(name, out);
    }

    let theirs = unar_extract_all(&data, "multi.arc");
    assert_eq!(ours, theirs, "our extraction disagrees with unar");
}
