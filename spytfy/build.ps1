$ErrorActionPreference = "Stop"
$ProjectRoot = $PSScriptRoot
$BuildDir = "C:\spytfy-build"

Write-Host "`n=== Spytfy Build Script ===" -ForegroundColor Cyan

# 1. Copy project to space-free path (MinGW dlltool breaks on spaces)
Write-Host "`n[1/5] Copying to $BuildDir (avoiding spaces in path)..." -ForegroundColor Yellow
if (Test-Path $BuildDir) {
    try { & npx nx daemon --stop 2>&1 | Out-Null } catch {}
    Remove-Item $BuildDir -Recurse -Force -Confirm:$false -ErrorAction SilentlyContinue
    if (Test-Path $BuildDir) { cmd /c "rmdir /s /q `"$BuildDir`"" 2>$null }
}
robocopy $ProjectRoot $BuildDir /E /XD node_modules target .nx .angular /NFL /NDL /NJH /NJS /NP | Out-Null

# 2. Symlink node_modules from original (avoids broken dependency resolution)
Write-Host "[2/5] Linking node_modules..." -ForegroundColor Yellow
New-Item -ItemType Junction -Path "$BuildDir\node_modules" -Target "$ProjectRoot\node_modules" | Out-Null

# 3. Ensure sidecar binaries exist with both GNU and MSVC naming
Write-Host "[3/5] Preparing sidecar binaries..." -ForegroundColor Yellow
$binDir = "$BuildDir\src-tauri\binaries"
$sidecars = @("yt-dlp", "ffmpeg")
foreach ($name in $sidecars) {
    $gnu  = "$binDir\$name-x86_64-pc-windows-gnu.exe"
    $msvc = "$binDir\$name-x86_64-pc-windows-msvc.exe"
    if ((Test-Path $gnu) -and -not (Test-Path $msvc)) {
        Copy-Item $gnu $msvc
    } elseif ((Test-Path $msvc) -and -not (Test-Path $gnu)) {
        Copy-Item $msvc $gnu
    }
}

# 4. Copy WebView2Loader.dll (GNU toolchain needs it at runtime)
Write-Host "[4/6] Copying WebView2Loader.dll..." -ForegroundColor Yellow
$webview2Src = Get-ChildItem "$env:USERPROFILE\.cargo\registry\src" -Filter "webview2-com-sys*" -Directory -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
if ($webview2Src) {
    $dll = "$($webview2Src.FullName)\x64\WebView2Loader.dll"
    if (Test-Path $dll) {
        Copy-Item $dll "$BuildDir\src-tauri\binaries\WebView2Loader.dll" -Force
    }
}

# 5. Clean old Rust build artifacts
Write-Host "[5/6] Cleaning previous Rust build..." -ForegroundColor Yellow
Push-Location "$BuildDir\src-tauri"
$ErrorActionPreference = "Continue"
cargo clean 2>&1 | Out-Null
$ErrorActionPreference = "Stop"
Pop-Location

# 6. Build
Write-Host "[6/6] Building installer (this takes ~3 minutes)..." -ForegroundColor Yellow
Push-Location $BuildDir
$ErrorActionPreference = "Continue"
pnpm tauri build --bundles nsis
$buildExitCode = $LASTEXITCODE
$ErrorActionPreference = "Stop"
Pop-Location

# 6. Copy result back
if ($buildExitCode -eq 0) {
    $nsisDir = "$BuildDir\src-tauri\target\release\bundle\nsis"
    if (Test-Path $nsisDir) {
        $installer = Get-ChildItem "$nsisDir\*.exe" | Select-Object -First 1
        if ($installer) {
            $dest = "$ProjectRoot\$($installer.Name)"
            Copy-Item $installer.FullName $dest -Force
            $sizeMB = [math]::Round($installer.Length / 1MB, 1)
            Write-Host "`n=== BUILD SUCCESS ===" -ForegroundColor Green
            Write-Host "Installer: $dest" -ForegroundColor Green
            Write-Host "Size: ${sizeMB} MB" -ForegroundColor Green
            exit 0
        }
    }
}

Write-Host "`n=== BUILD FAILED ===" -ForegroundColor Red
exit 1
