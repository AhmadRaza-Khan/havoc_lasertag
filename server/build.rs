//! Provides the OCR models (used to read game statistics PDFs) to `include_bytes!`, so the
//! server binary is self-contained. Each model is always verified against a pinned SHA-256
//! and taken from the first of:
//!
//! 1. `$OCR_MODELS_DIR/<name>`
//! 2. `~/.cache/ocrs/<name>` (where the `ocrs` CLI keeps them)
//! 3. a download from the ocrs model bucket (cached in OUT_DIR for later builds)

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

const BASE_URL: &str = "https://ocrs-models.s3-accelerate.amazonaws.com";
const MODELS: [(&str, &str); 2] = [
    (
        "text-detection.onnx",
        "a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe",
    ),
    (
        "text-recognition.onnx",
        "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5",
    ),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=OCR_MODELS_DIR");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    for (name, sha) in MODELS {
        let target = out_dir.join(name);
        if verified(&target, sha) {
            continue;
        }

        let candidates = [
            env::var_os("OCR_MODELS_DIR").map(|d| PathBuf::from(d).join(name)),
            env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/ocrs").join(name)),
        ];
        if let Some(found) = candidates.into_iter().flatten().find(|p| verified(p, sha)) {
            fs::copy(&found, &target).expect("copy OCR model");
            continue;
        }

        let url = format!("{BASE_URL}/{name}");
        println!("cargo:warning=downloading OCR model {url}");
        let mut bytes = Vec::new();
        ureq::get(&url)
            .call()
            .unwrap_or_else(|e| panic!("download {url}: {e} (or set OCR_MODELS_DIR)"))
            .into_body()
            .into_reader()
            .read_to_end(&mut bytes)
            .expect("read OCR model");
        let got = hex::encode(Sha256::digest(&bytes));
        assert_eq!(got, sha, "checksum mismatch for downloaded {name}");
        fs::write(&target, bytes).expect("write OCR model");
    }
}

fn verified(path: &Path, sha: &str) -> bool {
    fs::read(path)
        .map(|b| hex::encode(Sha256::digest(&b)) == sha)
        .unwrap_or(false)
}
