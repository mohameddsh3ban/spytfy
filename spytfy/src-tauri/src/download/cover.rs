use image::GenericImageView;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub struct CoverResult {
    pub bytes: Vec<u8>,
    pub hash: Vec<u8>,
}

fn validate_cover(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 4 {
        return Err("Cover too small".to_string());
    }

    let is_jpeg = bytes[0] == 0xFF && bytes[1] == 0xD8 && bytes[2] == 0xFF;
    let is_png = bytes[0] == 0x89 && bytes[1] == 0x50 && bytes[2] == 0x4E && bytes[3] == 0x47;

    if !is_jpeg && !is_png {
        return Err("Not a JPEG or PNG".to_string());
    }

    let img = image::load_from_memory(bytes).map_err(|e| format!("Invalid image: {e}"))?;
    let (w, h) = img.dimensions();
    if w < 300 || h < 300 {
        return Err(format!("Cover too small: {w}x{h}, need ≥300x300"));
    }

    Ok(())
}

fn ensure_jpeg(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let is_jpeg = bytes[0] == 0xFF && bytes[1] == 0xD8;
    if is_jpeg && bytes.len() <= 1_048_576 {
        return Ok(bytes.to_vec());
    }

    let img = image::load_from_memory(bytes).map_err(|e| format!("Failed to load image: {e}"))?;

    let (w, h) = img.dimensions();
    let img = if w > 1000 || h > 1000 {
        img.resize(1000, 1000, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Crop to square
    let (w, h) = img.dimensions();
    let img = if w != h {
        let size = w.min(h);
        let x = (w - size) / 2;
        let y = (h - size) / 2;
        img.crop_imm(x, y, size, size)
    } else {
        img
    };

    let mut jpeg_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut jpeg_bytes);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)
        .map_err(|e| format!("JPEG encode failed: {e}"))?;

    Ok(jpeg_bytes)
}

pub async fn fetch_cover(
    cache_dir: &Path,
    album_id: &str,
    cover_url: Option<&str>,
) -> Result<CoverResult, String> {
    let cache_path = cache_dir.join(format!("{album_id}.jpg"));

    // Check cache first
    if cache_path.exists() {
        let bytes = std::fs::read(&cache_path).map_err(|e| format!("Cache read failed: {e}"))?;
        let hash = Sha256::digest(&bytes).to_vec();
        return Ok(CoverResult { bytes, hash });
    }

    let url = cover_url.ok_or("No cover URL available")?;

    let response = reqwest::get(url)
        .await
        .map_err(|e| format!("Cover download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Cover URL returned {}", response.status()));
    }

    let raw_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read cover bytes: {e}"))?
        .to_vec();

    validate_cover(&raw_bytes)?;
    let jpeg_bytes = ensure_jpeg(&raw_bytes)?;

    // Cache
    std::fs::create_dir_all(cache_dir).ok();
    std::fs::write(&cache_path, &jpeg_bytes).ok();

    let hash = Sha256::digest(&jpeg_bytes).to_vec();
    Ok(CoverResult {
        bytes: jpeg_bytes,
        hash,
    })
}

pub fn cover_cache_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(".spytfy").join("cache").join("covers")
}
