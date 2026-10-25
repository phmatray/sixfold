//! Lists the card illustrations in `cards/` (`<slug>.webp`, see
//! `art::slug`) with a hash of each file, so the app asks only for art that
//! exists and the URL changes when a picture does (the service worker caches
//! art forever by URL).

use std::{env, fs, path::Path};

fn main() {
    println!("cargo:rerun-if-changed=cards");
    let mut art: Vec<(String, u64)> = fs::read_dir("cards")
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let slug = path
                .file_name()?
                .to_str()?
                .strip_suffix(".webp")?
                .to_string();
            // FNV-1a of the bytes.
            let hash = fs::read(&path)
                .ok()?
                .iter()
                .fold(0xcbf2_9ce4_8422_2325u64, |h, &b| {
                    (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
                });
            Some((slug, hash))
        })
        .collect();
    art.sort();
    let rows: String = art
        .iter()
        .map(|(slug, hash)| format!("    (\"{slug}\", \"{:08x}\"),\n", *hash as u32))
        .collect();
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("art_files.rs");
    fs::write(
        out,
        format!("pub const ART_FILES: &[(&str, &str)] = &[\n{rows}];\n"),
    )
    .unwrap();
}
