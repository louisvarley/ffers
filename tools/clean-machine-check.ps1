<#
.SYNOPSIS
  Clean-machine import check (burndown release-5): a fresh folder holding only
  the executable and a copy of an installed game must start, import its data,
  start again, and never change a source file.

.DESCRIPTION
  Stages a new empty folder with ffe-rs.exe (nothing else of this checkout: no
  data.bin, no .cache, no saves, no music pack) and a copy of the game folder,
  then checks, printing PASS or FAIL for each:

    1. every file of the copied game has the same SHA-256 as the source;
    2. a first run (--skip-intro --smoke-frames) imports the game and exits 0;
    3. it left no file inside the game folder and no unexpected file beside
       the executable (saves, settings and logs are the allowed ones);
    4. a second run exits 0 and the game folder is still byte-identical;
    5. an empty game folder is refused with a message that names the folder and
       FIRSTENC.EXE (exit code 2), not a panic.

  The game builds its content in memory on every start (there is no cache file
  to reuse), so "relaunch reuses the cache" is not a check here; the launcher's
  music.bin is the only generated file and is covered by the launcher's tests.
  This script cannot prove a machine without Rust or developer tools: run it
  from a copy of the packaged zip (tools\package.ps1) on the target machine to
  cover that.

.PARAMETER Game
  The installed game folder to copy (default: .\game). It is only read.
.PARAMETER Exe
  The executable to test (default: target\release\ffe-rs.exe).
.PARAMETER Keep
  Keep the staged folder afterwards (it is removed by default).
#>
param(
    [string]$Game = '',
    [string]$Exe = '',
    [switch]$Keep
)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not $Game) { $Game = Join-Path $root 'game' }
if (-not $Exe) { $Exe = Join-Path $root 'target\release\ffe-rs.exe' }
if (-not (Test-Path -LiteralPath (Join-Path $Game 'FIRSTENC.EXE'))) { throw "No FIRSTENC.EXE in $Game" }
if (-not (Test-Path -LiteralPath $Exe)) { throw "Missing $Exe; build first or pass -Exe" }

$stage = Join-Path ([IO.Path]::GetTempPath()) ("ffe-clean-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $stage | Out-Null
$failures = 0
function Check([string]$name, [bool]$ok, [string]$detail = '') {
    if ($ok) { Write-Host "PASS $name" } else { Write-Host "FAIL $name $detail"; $script:failures++ }
}
function Hashes([string]$folder) {
    Get-ChildItem -LiteralPath $folder -Recurse -File | Sort-Object FullName | ForEach-Object {
        "{0}  {1}" -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash, $_.FullName.Substring($folder.Length)
    }
}
function Run([string]$exe, [string[]]$arguments) {
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $exe
    $info.Arguments = ($arguments -join ' ')
    $info.WorkingDirectory = Split-Path $exe
    $info.RedirectStandardError = $true
    $info.RedirectStandardOutput = $true
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $process = [Diagnostics.Process]::Start($info)
    $err = $process.StandardError.ReadToEndAsync()
    $out = $process.StandardOutput.ReadToEndAsync()
    if (-not $process.WaitForExit(120000)) { $process.Kill(); return @{ Code = -1; Text = 'timed out after 120 s' } }
    @{ Code = $process.ExitCode; Text = ($out.Result + $err.Result) }
}

try {
    Copy-Item -LiteralPath $Exe -Destination (Join-Path $stage 'ffe-rs.exe')
    Copy-Item -LiteralPath $Game -Destination (Join-Path $stage 'game') -Recurse
    $staged = Join-Path $stage 'game'
    $sourceHashes = Hashes $Game
    Check 'the copied game matches the source byte for byte' (($sourceHashes -join "`n") -eq ((Hashes $staged) -join "`n"))

    $before = @(Get-ChildItem -LiteralPath $stage -Recurse -File | ForEach-Object { $_.FullName.Substring($stage.Length) })
    $first = Run (Join-Path $stage 'ffe-rs.exe') @('--skip-intro', '--smoke-frames', '30')
    Check 'first run imports the game and exits 0' ($first.Code -eq 0) ("(exit $($first.Code)): " + $first.Text.Trim())

    $after = @(Get-ChildItem -LiteralPath $stage -Recurse -File | ForEach-Object { $_.FullName.Substring($stage.Length) })
    $new = @($after | Where-Object { $before -notcontains $_ })
    $inGame = @($new | Where-Object { $_ -like '\game\*' })
    $allowed = '^\\(saves\\.*|.*\.cfg|.*\.log)$'
    $unexpected = @($new | Where-Object { $_ -notlike '\game\*' -and $_ -notmatch $allowed })
    Check 'no file was added inside the game folder' ($inGame.Count -eq 0) ($inGame -join ', ')
    Check 'only saves, settings and logs appear beside the executable' ($unexpected.Count -eq 0) ($unexpected -join ', ')

    $second = Run (Join-Path $stage 'ffe-rs.exe') @('--skip-intro', '--smoke-frames', '30')
    Check 'second run exits 0' ($second.Code -eq 0) ("(exit $($second.Code)): " + $second.Text.Trim())
    Check 'the game folder is still byte-identical after both runs' (($sourceHashes -join "`n") -eq ((Hashes $staged) -join "`n"))

    $empty = Join-Path $stage 'empty-game'
    New-Item -ItemType Directory -Path $empty | Out-Null
    $refused = Run (Join-Path $stage 'ffe-rs.exe') @('--game', "`"$empty`"", '--skip-intro', '--smoke-frames', '3')
    Check 'an empty game folder is refused clearly (exit 2, names FIRSTENC.EXE)' `
        ($refused.Code -eq 2 -and $refused.Text -match 'FIRSTENC\.EXE' -and $refused.Text -notmatch 'panicked') `
        ("(exit $($refused.Code)): " + $refused.Text.Trim())
}
finally {
    if ($Keep) { Write-Host "Kept $stage" } else { Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue }
}
if ($failures) { Write-Host "$failures check(s) failed."; exit 1 }
Write-Host 'Clean-machine check passed.'
