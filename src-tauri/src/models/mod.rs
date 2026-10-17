//! Speech model catalog, on-disk discovery, and integrity checks (P0 §0.2). Weights live in a
//! user-selectable directory, never in the database. A file counts as installed only when its size
//! matches the catalog; it is trusted only after its SHA-256 matches the pinned value.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Pinned catalog entry. Hashes/sizes are the published LFS object ids of the files in the
/// whisper.cpp model repository (MIT); they are rechecked locally before a file is trusted.
#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub file_name: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
    pub multilingual: bool,
    /// Rough peak RAM/VRAM while loaded, for labelling only.
    pub memory_mb: u32,
}

pub const MODEL_SOURCE_BASE: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";
pub const MODEL_LICENSE: &str = "MIT (OpenAI Whisper weights; whisper.cpp GGML conversion)";

pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        id: "whisper-base.en-q5_1",
        name: "Whisper Base, English (Q5_1)",
        file_name: "ggml-base.en-q5_1.bin",
        size_bytes: 59_721_011,
        sha256: "4baf70dd0d7c4247ba2b81fafd9c01005ac77c2f9ef064e00dcf195d0e2fdd2f",
        multilingual: false,
        memory_mb: 200,
    },
    CatalogEntry {
        id: "whisper-base-q5_1",
        name: "Whisper Base (Q5_1)",
        file_name: "ggml-base-q5_1.bin",
        size_bytes: 59_707_625,
        sha256: "422f1ae452ade6f30a004d7e5c6a43195e4433bc370bf23fac9cc591f01a8898",
        multilingual: true,
        memory_mb: 200,
    },
    CatalogEntry {
        id: "whisper-small.en-q5_1",
        name: "Whisper Small, English (Q5_1)",
        file_name: "ggml-small.en-q5_1.bin",
        size_bytes: 190_098_681,
        sha256: "bfdff4894dcb76bbf647d56263ea2a96645423f1669176f4844a1bf8e478ad30",
        multilingual: false,
        memory_mb: 500,
    },
    CatalogEntry {
        id: "whisper-small-q5_1",
        name: "Whisper Small (Q5_1)",
        file_name: "ggml-small-q5_1.bin",
        size_bytes: 190_085_487,
        sha256: "ae85e4a935d7a567bd102fe55afc16bb595bdb618e11b2fc7591bc08120411bb",
        multilingual: true,
        memory_mb: 500,
    },
    CatalogEntry {
        id: "whisper-medium-q5_0",
        name: "Whisper Medium (Q5_0)",
        file_name: "ggml-medium-q5_0.bin",
        size_bytes: 539_212_467,
        sha256: "19fea4b380c3a618ec4723c3eef2eb785ffba0d0538cf43f8f235e7b3b34220f",
        multilingual: true,
        memory_mb: 1_000,
    },
    CatalogEntry {
        id: "whisper-large-v3-turbo-q5_0",
        name: "Whisper Large v3 Turbo (Q5_0)",
        file_name: "ggml-large-v3-turbo-q5_0.bin",
        size_bytes: 574_041_195,
        sha256: "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2",
        multilingual: true,
        memory_mb: 1_200,
    },
];

pub fn catalog_entry(id: &str) -> Option<&'static CatalogEntry> {
    CATALOG.iter().find(|e| e.id == id)
}

/// What the UI shows for each model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub purpose: String,  // "speech" in P0
    pub locality: String, // "local": inference is on-device
    pub size_bytes: u64,
    pub memory_mb: u32,
    pub languages: Vec<String>,
    pub license: String,
    pub source: String,
    pub sha256: String,
    pub installed: bool,
    pub path: Option<String>,
    pub selected: bool,
    pub integrity_verified: bool,
    /// Real-time factor measured on this machine (processing time / audio time). None until run.
    pub measured_rtf: Option<f32>,
    pub measured_backend: Option<String>,
}

/// Per-model facts learned at runtime (verification + benchmark), kept by the app.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRuntimeFacts {
    pub integrity_verified: bool,
    pub measured_rtf: Option<f32>,
    pub measured_backend: Option<String>,
}

pub fn model_path(dir: &Path, entry: &CatalogEntry) -> PathBuf {
    dir.join(entry.file_name)
}

/// Installed = file present with the exact catalog size (cheap check; hashing is `verify_file`).
pub fn is_installed(dir: &Path, entry: &CatalogEntry) -> bool {
    std::fs::metadata(model_path(dir, entry))
        .map(|m| m.is_file() && m.len() == entry.size_bytes)
        .unwrap_or(false)
}

pub fn describe(
    dir: &Path,
    selected_id: Option<&str>,
    facts: &dyn Fn(&str) -> ModelRuntimeFacts,
) -> Vec<ModelInfo> {
    CATALOG
        .iter()
        .map(|e| {
            let installed = is_installed(dir, e);
            let f = facts(e.id);
            ModelInfo {
                id: e.id.into(),
                name: e.name.into(),
                purpose: "speech".into(),
                locality: "local".into(),
                size_bytes: e.size_bytes,
                memory_mb: e.memory_mb,
                languages: vec![if e.multilingual { "multilingual" } else { "en" }.into()],
                license: MODEL_LICENSE.into(),
                source: format!("{MODEL_SOURCE_BASE}{}", e.file_name),
                sha256: e.sha256.into(),
                installed,
                path: installed.then(|| model_path(dir, e).display().to_string()),
                selected: selected_id == Some(e.id),
                integrity_verified: installed && f.integrity_verified,
                measured_rtf: f.measured_rtf,
                measured_backend: f.measured_backend,
            }
        })
        .collect()
}

pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut reader = BufReader::with_capacity(1 << 20, File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, PartialEq, Eq)]
pub enum Verify {
    Ok,
    Missing,
    WrongSize { actual: u64 },
    HashMismatch { actual: String },
}

pub fn verify_file(path: &Path, size: u64, sha256: &str) -> std::io::Result<Verify> {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Verify::Missing),
        Err(e) => return Err(e),
    };
    if meta.len() != size {
        return Ok(Verify::WrongSize { actual: meta.len() });
    }
    let actual = sha256_file(path)?;
    if actual.eq_ignore_ascii_case(sha256) {
        Ok(Verify::Ok)
    } else {
        Ok(Verify::HashMismatch { actual })
    }
}

/// whisper.cpp model files start with the GGML magic ("ggml" as a little-endian u32). Reject
/// anything else (pickles, archives, executables) before it reaches the native loader.
pub fn has_ggml_magic(path: &Path) -> std::io::Result<bool> {
    let mut magic = [0u8; 4];
    let mut f = File::open(path)?;
    if f.read(&mut magic)? < 4 {
        return Ok(false);
    }
    Ok(u32::from_le_bytes(magic) == 0x6767_6d6c)
}

/// Stop→transcript p50 target for a ~5 s utterance (docs/decisions.md 2026-10-14).
pub const LATENCY_TARGET_MS: u32 = 1000;

/// p50 ms per ~5 s clip measured on the dev machine (GTX 1080 Vulkan / Xeon W-2235 CPU, 6 threads),
/// used only as the starting recommendation until this machine has its own measurements.
pub fn reference_p50_ms(id: &str, gpu: bool) -> Option<u32> {
    let v = match (id, gpu) {
        ("whisper-base-q5_1" | "whisper-base.en-q5_1", true) => 144,
        ("whisper-small-q5_1" | "whisper-small.en-q5_1", true) => 299,
        ("whisper-medium-q5_0", true) => 720,
        ("whisper-large-v3-turbo-q5_0", true) => 947,
        ("whisper-base-q5_1" | "whisper-base.en-q5_1", false) => 1_774,
        ("whisper-small-q5_1" | "whisper-small.en-q5_1", false) => 5_235,
        ("whisper-medium-q5_0", false) => 15_085,
        ("whisper-large-v3-turbo-q5_0", false) => 25_870,
        _ => return None,
    };
    Some(v)
}

/// Largest installed model (catalog order = ascending size) whose p50 meets the target; if none
/// does, the fastest installed one. `p50` supplies a measurement per model id.
pub fn recommend<'a>(installed: &[&'a str], p50: impl Fn(&str) -> Option<u32>) -> Option<&'a str> {
    let ordered: Vec<&str> = CATALOG.iter().map(|e| e.id).filter(|id| installed.contains(id)).collect();
    let pick = ordered
        .iter()
        .rev()
        .find(|id| p50(id).is_some_and(|ms| ms <= LATENCY_TARGET_MS))
        .or_else(|| ordered.iter().min_by_key(|id| p50(id).unwrap_or(u32::MAX)))?;
    installed.iter().copied().find(|i| i == pick)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("voxelle-models-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn recommends_largest_model_meeting_target_else_fastest() {
        let all = ["whisper-base-q5_1", "whisper-small-q5_1", "whisper-medium-q5_0", "whisper-large-v3-turbo-q5_0"];
        assert_eq!(recommend(&all, |id| reference_p50_ms(id, true)), Some("whisper-large-v3-turbo-q5_0"));
        assert_eq!(recommend(&all, |id| reference_p50_ms(id, false)), Some("whisper-base-q5_1"));
        assert_eq!(recommend(&["whisper-small-q5_1"], |id| reference_p50_ms(id, true)), Some("whisper-small-q5_1"));
        assert_eq!(recommend(&[], |id| reference_p50_ms(id, true)), None);
    }

    #[test]
    fn catalog_is_well_formed() {
        let mut ids: Vec<_> = CATALOG.iter().map(|e| e.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len(), "duplicate ids");
        for e in CATALOG {
            assert_eq!(e.sha256.len(), 64, "{}", e.id);
            assert!(e.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
            assert!(e.file_name.ends_with(".bin") && !e.file_name.contains(['/', '\\']));
            assert!(e.size_bytes > 1_000_000);
        }
    }

    #[test]
    fn sha256_matches_known_vector() {
        let d = tmp("sha");
        let p = d.join("abc.txt");
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(
            sha256_file(&p).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn verify_reports_missing_size_and_hash_problems() {
        let d = tmp("verify");
        let p = d.join("m.bin");
        assert_eq!(verify_file(&p, 3, "00").unwrap(), Verify::Missing);
        std::fs::write(&p, b"abc").unwrap();
        assert_eq!(verify_file(&p, 4, "00").unwrap(), Verify::WrongSize { actual: 3 });
        assert!(matches!(verify_file(&p, 3, &"0".repeat(64)).unwrap(), Verify::HashMismatch { .. }));
        assert_eq!(
            verify_file(&p, 3, "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD").unwrap(),
            Verify::Ok
        );
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn ggml_magic_check() {
        let d = tmp("magic");
        let good = d.join("good.bin");
        std::fs::write(&good, [0x6c, 0x6d, 0x67, 0x67, 1, 2]).unwrap();
        assert!(has_ggml_magic(&good).unwrap());
        let pickle = d.join("pickle.bin");
        std::fs::write(&pickle, [0x80, 0x02, b'c', b'_']).unwrap();
        assert!(!has_ggml_magic(&pickle).unwrap());
        let short = d.join("short.bin");
        std::fs::write(&short, [0x6c]).unwrap();
        assert!(!has_ggml_magic(&short).unwrap());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn describe_marks_only_size_matching_files_installed() {
        let d = tmp("describe");
        // Wrong-size file with a catalog name must not count as installed.
        std::fs::write(d.join(CATALOG[0].file_name), b"partial").unwrap();
        let infos = describe(&d, Some(CATALOG[0].id), &|_| ModelRuntimeFacts {
            integrity_verified: true,
            ..Default::default()
        });
        assert!(!infos[0].installed);
        assert!(!infos[0].integrity_verified, "never verified when not installed");
        assert!(infos[0].selected);
        assert!(infos[0].path.is_none());
        let _ = std::fs::remove_dir_all(d);
    }
}
