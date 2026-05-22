use id3::Tag;
use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::path::Path;

pub enum VerifyResult {
    Ok,
    Warning(String),
}

pub fn verify_mp3(mp3_path: &Path, expected_cover_hash: &[u8]) -> Result<VerifyResult, String> {
    let tag =
        Tag::read_from_path(mp3_path).map_err(|e| format!("Failed to read tags for verification: {e}"))?;

    let pictures: Vec<_> = tag.pictures().collect();
    if pictures.is_empty() {
        return Ok(VerifyResult::Warning("no APIC frame found".to_string()));
    }

    let pic = &pictures[0];
    let actual_hash = Sha256::digest(&pic.data).to_vec();
    if actual_hash != expected_cover_hash {
        return Ok(VerifyResult::Warning("cover art hash mismatch".to_string()));
    }

    let img = match image::load_from_memory(&pic.data) {
        std::result::Result::Ok(img) => img,
        Err(e) => {
            return Ok(VerifyResult::Warning(format!("invalid embedded image: {e}")));
        }
    };

    let (w, h) = img.dimensions();
    if w < 300 || h < 300 {
        return Ok(VerifyResult::Warning(format!("embedded cover {w}x{h} < 300x300")));
    }

    Ok(VerifyResult::Ok)
}
