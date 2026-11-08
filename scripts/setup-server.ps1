param(
  [string]$Release = 'v0.5.3',
  [switch]$DryRun,
  [switch]$BootstrapOnly,
  [switch]$UseLocalNode,
  [string]$BundleDirectory,
  [string]$WorkerName
)
$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Add-Type -AssemblyName System.IO.Compression.FileSystem
if ($Release -notmatch '^v\d+\.\d+\.\d+(?:-[a-zA-Z0-9.-]+)?$') { throw 'Invalid release version.' }
$toolHome = $env:HUSH_SETUP_HOME
if (-not $toolHome) { $toolHome = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Hush\server-tools' }
[IO.Directory]::CreateDirectory($toolHome) | Out-Null
$scratch = Join-Path $toolHome ([Guid]::NewGuid().ToString())
[IO.Directory]::CreateDirectory($scratch) | Out-Null
function Get-Text([string]$Url) {
  # Release assets may use application/octet-stream even for SHA256SUMS.txt.
  $textFile = Join-Path $scratch ([Guid]::NewGuid().ToString() + '.txt')
  Invoke-WebRequest -Uri $Url -UseBasicParsing -OutFile $textFile -TimeoutSec 120
  return [IO.File]::ReadAllText($textFile)
}
function Test-Checksum([string]$File, [string]$Expected) {
  $stream = [IO.File]::OpenRead($File)
  $sha = [Security.Cryptography.SHA256]::Create()
  try { $actual = [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '').ToLowerInvariant() }
  finally { $stream.Dispose(); $sha.Dispose() }
  if ($actual -ne $Expected.ToLowerInvariant()) { throw "Checksum verification failed: $File" }
}
function Get-Checksum([string]$Text, [string]$Name) {
  $pattern = '(?m)^([a-fA-F0-9]{64})\s+\*?' + [regex]::Escape($Name) + '\r?$'
  $found = [regex]::Matches($Text, $pattern)
  if ($found.Count -ne 1) { throw "Missing or ambiguous checksum for $Name" }
  return $found[0].Groups[1].Value
}
try {
  Write-Host 'Hush host setup. GitHub distributes the tools; Cloudflare runs the room server.'
  $node = $null
  $existing = Get-Command node -ErrorAction SilentlyContinue
  if (-not $UseLocalNode -and $existing -and (Get-Command npm -ErrorAction SilentlyContinue)) {
    $major = & $existing.Source -p "process.versions.node.split('.')[0]"
    if ($LASTEXITCODE -eq 0 -and [int]$major -ge 22) { $node = $existing.Source }
  }
  if (-not $node) {
    $runtime = Join-Path $toolHome 'runtime'
    $candidate = Join-Path $runtime 'node.exe'
    if (Test-Path -LiteralPath $candidate) {
      $major = & $candidate -p "process.versions.node.split('.')[0]"
      if ($LASTEXITCODE -eq 0 -and [int]$major -ge 22) { $node = $candidate }
    }
    if (-not $node) {
      $architecture = $env:PROCESSOR_ARCHITEW6432
      if (-not $architecture) { $architecture = $env:PROCESSOR_ARCHITECTURE }
      $arch = switch ($architecture) { 'AMD64' { 'x64' } 'ARM64' { 'arm64' } default { throw 'Windows x64 or ARM64 is required.' } }
      Write-Host 'Installing a private Node 24 runtime for the host tools. No administrator access is needed.'
      $base = 'https://nodejs.org/dist/latest-v24.x/'
      $sums = Get-Text ($base + 'SHASUMS256.txt')
      $found = [regex]::Matches($sums, ('(?m)^([a-fA-F0-9]{64})\s+(node-v24\.\d+\.\d+-win-' + $arch + '\.zip)\r?$'))
      if ($found.Count -ne 1) { throw 'Could not locate the official Node archive.' }
      $filename = $found[0].Groups[2].Value
      $archive = Join-Path $scratch 'node.zip'
      Invoke-WebRequest -Uri ($base + $filename) -OutFile $archive -UseBasicParsing -TimeoutSec 300
      Test-Checksum $archive $found[0].Groups[1].Value
      $expanded = Join-Path $scratch 'node'
      [IO.Compression.ZipFile]::ExtractToDirectory($archive, $expanded)
      if (Test-Path -LiteralPath $runtime) { Remove-Item -LiteralPath $runtime -Recurse -Force }
      [IO.Directory]::Move((Join-Path $expanded ($filename -replace '\.zip$', '')), $runtime)
      $node = Join-Path $runtime 'node.exe'
    }
  }
  if ($BootstrapOnly) { & $node --version; if ($LASTEXITCODE -ne 0) { throw 'Node did not start.' }; return }
  $base = "https://github.com/basilaslam/hush-native/releases/download/$Release/"
  $archive = Join-Path $scratch 'server.zip'
  if ($BundleDirectory) {
    $BundleDirectory = (Resolve-Path -LiteralPath $BundleDirectory).ProviderPath
    $sums = [IO.File]::ReadAllText((Join-Path $BundleDirectory 'SHA256SUMS.txt'))
    $expected = Get-Checksum $sums 'Hush-Server-Setup.zip'
    [IO.File]::Copy((Join-Path $BundleDirectory 'Hush-Server-Setup.zip'), $archive)
  } else {
    $sums = Get-Text ($base + 'SHA256SUMS.txt')
    $expected = Get-Checksum $sums 'Hush-Server-Setup.zip'
    Invoke-WebRequest -Uri ($base + 'Hush-Server-Setup.zip') -OutFile $archive -UseBasicParsing -TimeoutSec 180
  }
  Test-Checksum $archive $expected
  $expanded = Join-Path $scratch 'server'
  [IO.Compression.ZipFile]::ExtractToDirectory($archive, $expanded)
  $application = Join-Path $toolHome "releases\$Release"
  [IO.Directory]::CreateDirectory($application) | Out-Null
  foreach ($name in @('setup.mjs', 'worker.mjs', 'package.json', 'package-lock.json', 'README.md')) {
    [IO.File]::Copy((Join-Path $expanded $name), (Join-Path $application $name), $true)
  }
  $arguments = @((Join-Path $application 'setup.mjs'))
  if ($DryRun) { $arguments += '--dry-run' }
  if ($WorkerName) { $arguments += @('--worker', $WorkerName) }
  & $node @arguments
  if ($LASTEXITCODE -ne 0) { throw 'Server setup did not finish. Fix the reported issue and rerun this script.' }
} finally {
  if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
}
