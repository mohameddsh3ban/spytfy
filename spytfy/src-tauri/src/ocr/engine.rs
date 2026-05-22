use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;

pub async fn ocr_image(image_path: &str) -> Result<String, String> {
    // Pre-process: crop left side to remove album art thumbnails
    let cropped_path = crop_left_margin(image_path)?;
    let path_to_ocr = cropped_path.as_deref().unwrap_or(image_path);

    // Try Tesseract first
    if let Ok(text) = run_tesseract(path_to_ocr).await {
        return Ok(text);
    }

    // Fallback: Windows PowerShell OCR
    run_windows_ocr(path_to_ocr).await
}

fn crop_left_margin(image_path: &str) -> Result<Option<String>, String> {
    let img = image::open(image_path).map_err(|e| format!("Failed to open image: {e}"))?;
    let (w, h) = img.dimensions();

    // Crop left 15% to remove album art thumbnails + track number column noise
    let crop_x = (w as f32 * 0.04) as u32;
    let crop_w = w - crop_x;

    if crop_w < 100 {
        return Ok(None);
    }

    let cropped = img.crop_imm(crop_x, 0, crop_w, h);
    let cropped_path = format!("{}-cropped.png", image_path);
    cropped.save(&cropped_path).map_err(|e| format!("Failed to save cropped: {e}"))?;

    Ok(Some(cropped_path))
}

use image::GenericImageView;

async fn run_tesseract(image_path: &str) -> Result<String, String> {
    let tesseract_paths = [
        "C:\\Program Files\\Tesseract-OCR\\tesseract.exe",
        "tesseract",
    ];

    for tess_path in &tesseract_paths {
        let result = Command::new(tess_path)
            .args([image_path, "stdout", "--psm", "6", "-l", "eng"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await;

        if let Ok(output) = result {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }
        }
    }

    Err("Tesseract not available".to_string())
}

async fn run_windows_ocr(image_path: &str) -> Result<String, String> {
    let script = format!(
        r#"
        Add-Type -AssemblyName System.Runtime.WindowsRuntime
        $null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType=WindowsRuntime]
        $null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType=WindowsRuntime]
        $null = [Windows.Storage.StorageFile, Windows.Foundation, ContentType=WindowsRuntime]

        $path = '{}'

        $asyncOp = [Windows.Storage.StorageFile]::GetFileFromPathAsync($path)
        $typeName = 'System.WindowsRuntimeSystemExtensions'
        $awaiter = $asyncOp.GetAwaiter()
        $awaiter.GetResult() | Out-Null
        $file = $asyncOp.GetResults()

        $stream = $file.OpenAsync([Windows.Storage.FileAccessMode]::Read).GetAwaiter().GetResult()
        $decoder = [Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream).GetAwaiter().GetResult()
        $bitmap = $decoder.GetSoftwareBitmapAsync().GetAwaiter().GetResult()

        $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
        $result = $engine.RecognizeAsync($bitmap).GetAwaiter().GetResult()
        Write-Output $result.Text
        "#,
        image_path.replace('\'', "''")
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("Windows OCR failed: {e}"))?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        if !text.trim().is_empty() {
            return Ok(text);
        }
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!("OCR failed: {stderr}"))
}
