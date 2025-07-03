$Repo = "myferr/meow"
$Arch = if ($env:PROCESSOR_ARCHITECTURE -eq "AMD64") { "windows-x86_64" } else { throw "Unsupported arch" }

Write-Host "Fetching latest release info..."
$Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"

$Asset = $Release.assets | Where-Object { $_.name -eq "meow-$Arch.exe" }
if (-not $Asset) { throw "No matching asset found for $Arch" }

$BinDir = "$HOME\.cargo\bin"
$TargetPath = Join-Path $BinDir "meow.exe"

Write-Host "Downloading $($Asset.name)..."
Invoke-WebRequest -Uri $Asset.browser_download_url -OutFile $TargetPath

Write-Host "Making sure $BinDir is in PATH..."
if (-not ($env:PATH -split ";" | Where-Object { $_ -eq $BinDir })) {
  [Environment]::SetEnvironmentVariable("PATH", "$env:PATH;$BinDir", [System.EnvironmentVariableTarget]::User)
}

Write-Host "✅ Installed meow to $TargetPath"
