param(
    [Parameter(Mandatory = $true)]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$OutputDir = "dist"
)

$ErrorActionPreference = "Stop"
$BinaryName = "cockroach-reminder-gpui"
$BinaryPath = Join-Path "target/$Target/release" "$BinaryName.exe"
$Version = $Version.TrimStart("v")
$Architecture = $Target.Split("-")[0]

if (-not (Test-Path -Path $BinaryPath -PathType Leaf)) {
    throw "Release binary not found: $BinaryPath"
}

New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
$OutputDir = (Resolve-Path $OutputDir).Path
$StagingDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString())
$PackageDir = Join-Path $StagingDir "cockroach-reminder"

try {
    New-Item -ItemType Directory -Path $PackageDir -Force | Out-Null
    Copy-Item $BinaryPath (Join-Path $PackageDir "$BinaryName.exe")
    Copy-Item README.md, README.zh-CN.md $PackageDir

    $Archive = Join-Path $OutputDir "cockroach-reminder-v$Version-windows-$Architecture.zip"
    Compress-Archive -Path (Join-Path $PackageDir "*") -DestinationPath $Archive -CompressionLevel Optimal -Force
    Write-Output "Created $Archive"
}
finally {
    if (Test-Path $StagingDir) {
        Remove-Item $StagingDir -Recurse -Force
    }
}
