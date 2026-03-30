$ws  = New-Object -ComObject WScript.Shell
$lnk = $ws.CreateShortcut("$env:USERPROFILE\Desktop\rmtide.lnk")
$lnk.TargetPath       = "cmd.exe"
$lnk.Arguments        = "/k C:\Users\shive\Desktop\EchoAtlasDeep\run.bat"
$lnk.WorkingDirectory = "C:\Users\shive\Desktop\EchoAtlasDeep"
$lnk.Description      = "Launch rmtide terminal IDE"
$lnk.WindowStyle      = 1
$lnk.Save()

# Set shortcut to run as Administrator
$bytes = [System.IO.File]::ReadAllBytes("$env:USERPROFILE\Desktop\rmtide.lnk")
$bytes[0x15] = $bytes[0x15] -bor 0x20
[System.IO.File]::WriteAllBytes("$env:USERPROFILE\Desktop\rmtide.lnk", $bytes)

Write-Host "Shortcut updated (runs as Administrator)."
