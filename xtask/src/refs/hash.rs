//! SHA-256 of files and `sha256sum`-style manifests.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

/// The lowercase hex SHA-256 of a byte slice.
#[cfg(test)]
pub fn sha256_bytes(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

/// The lowercase hex SHA-256 of a file, read in chunks so large downloads stay out of memory.
pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|err| format!("could not open {}: {err}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 1 << 16];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|err| format!("could not read {}: {err}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// One line of a manifest: the expected hash of a file, by path relative to the manifest's base.
#[derive(Debug, PartialEq, Eq)]
pub struct ManifestEntry {
    pub sha256: String,
    pub path: String,
}

/// Parses `sha256sum` output: `<64 hex><space><space or *><path>` per line. Blank lines are
/// skipped. Paths must be relative, without `..`.
pub fn parse_manifest(text: &str) -> Result<Vec<ManifestEntry>, String> {
    let mut entries = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let bad = || format!("manifest line {} is not `<sha256>  <path>`", index + 1);
        let (sha256, rest) = line.split_at_checked(64).ok_or_else(bad)?;
        let path = rest
            .strip_prefix("  ")
            .or_else(|| rest.strip_prefix(" *"))
            .ok_or_else(bad)?;
        let hex_ok = sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        let path_ok = !path.is_empty()
            && !Path::new(path).has_root()
            && path.split(['/', '\\']).all(|part| part != "..");
        if !hex_ok || !path_ok {
            return Err(bad());
        }
        entries.push(ManifestEntry {
            sha256: sha256.to_owned(),
            path: path.to_owned(),
        });
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn hashes_match_the_fips_180_2_test_vectors() {
        assert_eq!(sha256_bytes(b""), EMPTY);
        assert_eq!(sha256_bytes(b"abc"), ABC);
    }

    #[test]
    fn hashes_a_file_larger_than_one_chunk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.bin");
        let bytes: Vec<u8> = (0..200_000_u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(sha256_file(&path).unwrap(), sha256_bytes(&bytes));
    }

    #[test]
    fn parses_text_and_binary_manifest_lines() {
        let text = format!("{EMPTY}  a/empty.txt\r\n\n{ABC} *b.bin\n");
        let entries = parse_manifest(&text).unwrap();
        assert_eq!(
            entries,
            [
                ManifestEntry {
                    sha256: EMPTY.into(),
                    path: "a/empty.txt".into()
                },
                ManifestEntry {
                    sha256: ABC.into(),
                    path: "b.bin".into()
                },
            ]
        );
    }

    #[test]
    fn rejects_malformed_manifest_lines() {
        for bad in [
            format!("{EMPTY} one-space.txt"),
            format!("{}  upper.txt", EMPTY.to_uppercase()),
            format!("{EMPTY}  ../escape.txt"),
            format!("{EMPTY}  /abs.txt"),
            "short  file.txt".to_owned(),
        ] {
            assert!(parse_manifest(&bad).is_err(), "{bad}");
        }
    }
}
