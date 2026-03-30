# rmtide installer — downloads latest release from GitHub and installs to Program Files
# Run as Administrator: powershell -ExecutionPolicy Bypass -File install.ps1

param(
    [string]$Repo = "shivemind/EchoAtlasDeep"
)

$InstallDir = "C:\Program Files\rmtide"
$ExePath    = "$InstallDir\rmtide.exe"
$ShortcutPath = "$env:USERPROFILE\Desktop\rmtide.lnk"

Write-Host "Fetching latest release from $Repo..."

try {
    $release = Invoke-RestMethod "https://api.github.com/repos/$Repo/releases/latest"
} catch {
    Write-Error "Could not fetch release. Make sure the repo is correct and has a release."
    exit 1
}

$asset = $release.assets | Where-Object { $_.name -like "*windows*" } | Select-Object -First 1
if (-not $asset) {
    Write-Error "No Windows asset found in latest release."
    exit 1
}

Write-Host "Downloading $($asset.name) v$($release.tag_name)..."
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $ExePath

Write-Host "Installed to $ExePath"

# Create desktop shortcut
$ws  = New-Object -ComObject WScript.Shell
$lnk = $ws.CreateShortcut($ShortcutPath)
$lnk.TargetPath       = $ExePath
$lnk.WorkingDirectory = $InstallDir
$lnk.Description      = "rmtide terminal IDE"
$lnk.WindowStyle      = 1
$lnk.Save()

Write-Host "Desktop shortcut created."
Write-Host ""
Write-Host "Done! Double-click rmtide on your desktop to launch."
