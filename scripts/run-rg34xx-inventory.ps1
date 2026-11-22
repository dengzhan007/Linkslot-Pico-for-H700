[CmdletBinding()]
param(
    [string]$DeviceSerial = "",
    [string]$BlockDevice = "",
    [string]$OutputDirectory = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$Collector = Join-Path $PSScriptRoot "collect-rg34xx-readonly-inventory.sh"
$Boot0Inspector = Join-Path $RepoRoot "tools\inspect_h700_boot0.py"
$Toc1Parser = Join-Path $RepoRoot "tools\parse_h700_toc1.py"
$FdtInspector = Join-Path $RepoRoot "tools\inspect_h700_fdt.py"
$EnvInspector = Join-Path $RepoRoot "tools\inspect_h700_env.py"
$MemoryMapInspector = Join-Path $RepoRoot "tools\inspect_h700_memory_map.py"

foreach ($path in @($Collector, $Boot0Inspector, $Toc1Parser, $FdtInspector, $EnvInspector, $MemoryMapInspector)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required project file not found: $path"
    }
}

$adbCommand = Get-Command adb -ErrorAction SilentlyContinue
if (-not $adbCommand) {
    throw "adb was not found in PATH. Install Android platform-tools first."
}

$pythonCommand = Get-Command py -ErrorAction SilentlyContinue
$pythonArgs = @()
if ($pythonCommand) {
    $pythonExe = $pythonCommand.Source
    $pythonArgs = @("-3")
} else {
    $pythonCommand = Get-Command python -ErrorAction SilentlyContinue
    if (-not $pythonCommand) {
        throw "Python 3 was not found. Install Python 3 or the Windows py launcher."
    }
    $pythonExe = $pythonCommand.Source
    $pythonArgs = @()
}

$deviceRows = @(& adb devices)
if ($LASTEXITCODE -ne 0) {
    throw "adb devices failed."
}
$availableSerials = @()
foreach ($line in $deviceRows) {
    if ($line -match "^\s*(\S+)\s+device(?:\s|$)") {
        $availableSerials += $Matches[1]
    }
}

if ([string]::IsNullOrWhiteSpace($DeviceSerial)) {
    if ($availableSerials.Count -ne 1) {
        throw "Expected exactly one authorized ADB device; found $($availableSerials.Count). Pass -DeviceSerial when multiple devices are connected."
    }
    $DeviceSerial = $availableSerials[0]
} elseif ($availableSerials -notcontains $DeviceSerial) {
    throw "ADB device '$DeviceSerial' is not in the authorized 'device' state."
}

$adbPrefix = @("-s", $DeviceSerial)
$uidOutput = @(& adb @adbPrefix shell id -u)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to query ADB shell identity."
}
$uid = ($uidOutput -join "").Trim()
if ($uid -ne "0") {
    throw "ADB shell is not root (uid=$uid). Use a root ADB connection or run the documented collector from root SSH."
}

$cmdlineOutput = @(& adb @adbPrefix shell cat /proc/cmdline)
if ($LASTEXITCODE -ne 0) {
    throw "Unable to read /proc/cmdline from the device."
}
$cmdline = ($cmdlineOutput -join " ").Trim()
$rootMatch = [regex]::Match($cmdline, "root=(/dev/[^\s]+)")
if ([string]::IsNullOrWhiteSpace($BlockDevice)) {
    if (-not $rootMatch.Success) {
        throw "No root=/dev/... entry found. Pass -BlockDevice explicitly after confirming the SD disk."
    }
    $rootDevice = $rootMatch.Groups[1].Value
    if ($rootDevice -match "^(/dev/mmcblk\d+)p\d+$") {
        $BlockDevice = $Matches[1]
    } elseif ($rootDevice -match "^(/dev/nvme\d+n\d+)p\d+$") {
        $BlockDevice = $Matches[1]
    } elseif ($rootDevice -match "^(/dev/sd[a-z]+)\d+$") {
        $BlockDevice = $Matches[1]
    } else {
        throw "Cannot derive the whole-disk device from root=$rootDevice. Pass -BlockDevice after checking the live partition table."
    }
} elseif ($rootMatch.Success) {
    $rootDevice = $rootMatch.Groups[1].Value
    $derivedDisk = $null
    if ($rootDevice -match "^(/dev/mmcblk\d+)p\d+$") {
        $derivedDisk = $Matches[1]
    } elseif ($rootDevice -match "^(/dev/nvme\d+n\d+)p\d+$") {
        $derivedDisk = $Matches[1]
    } elseif ($rootDevice -match "^(/dev/sd[a-z]+)\d+$") {
        $derivedDisk = $Matches[1]
    }
    if ($derivedDisk -and $derivedDisk -ne $BlockDevice) {
        throw "BlockDevice '$BlockDevice' does not match root disk '$derivedDisk' from /proc/cmdline."
    }
}

$stamp = Get-Date -Format "yyyyMMdd-HHmmss"
$remoteScript = "/tmp/rg34xx-inventory-$stamp.sh"
$remoteOutput = "/tmp/rg34xx-inventory-$stamp"
if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $RepoRoot "build\rg34xx-inventory-$stamp"
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $OutputDirectory) {
    throw "Output directory already exists: $OutputDirectory"
}
$parent = Split-Path -Parent $OutputDirectory
New-Item -ItemType Directory -Force -Path $parent | Out-Null

Write-Host "ADB device: $DeviceSerial"
Write-Host "Block device: $BlockDevice"
Write-Host "Remote output: $remoteOutput"
Write-Host "Local output: $OutputDirectory"

& adb @adbPrefix push $Collector $remoteScript
if ($LASTEXITCODE -ne 0) {
    throw "Failed to push the read-only collector."
}

& adb @adbPrefix shell sh $remoteScript $BlockDevice $remoteOutput
if ($LASTEXITCODE -ne 0) {
    throw "Device-side inventory collection failed. No firmware write is performed by this collector."
}

& adb @adbPrefix pull $remoteOutput $OutputDirectory
if ($LASTEXITCODE -ne 0) {
    throw "Failed to pull the inventory directory."
}

$primary = Join-Path $OutputDirectory "boot0-primary.bin"
$secondary = Join-Path $OutputDirectory "boot0-secondary.bin"
$toc1 = Join-Path $OutputDirectory "toc1-package.bin"
foreach ($path in @($primary, $secondary, $toc1)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Expected inventory file missing after pull: $path"
    }
}

& $pythonExe @pythonArgs $Boot0Inspector $primary -o (Join-Path $OutputDirectory "boot0-primary.json")
if ($LASTEXITCODE -ne 0) { throw "Primary Boot0 inspection failed." }

& $pythonExe @pythonArgs $Boot0Inspector $secondary -o (Join-Path $OutputDirectory "boot0-secondary.json")
if ($LASTEXITCODE -ne 0) { throw "Secondary Boot0 inspection failed." }

& $pythonExe @pythonArgs $Toc1Parser $toc1 -o (Join-Path $OutputDirectory "toc1-extracted")
if ($LASTEXITCODE -ne 0) { throw "TOC1 validation/extraction failed." }

$liveFdt = Join-Path $OutputDirectory "live-fdt.dtb"
if (Test-Path -LiteralPath $liveFdt -PathType Leaf) {
    & $pythonExe @pythonArgs $FdtInspector $liveFdt -o (Join-Path $OutputDirectory "live-fdt.json")
    if ($LASTEXITCODE -ne 0) { throw "Live FDT inspection failed." }
}

$packageDtb = Join-Path $OutputDirectory "toc1-extracted\items\dtb.bin"
if (Test-Path -LiteralPath $packageDtb -PathType Leaf) {
    & $pythonExe @pythonArgs $FdtInspector $packageDtb -o (Join-Path $OutputDirectory "toc1-extracted\dtb-report.json")
    if ($LASTEXITCODE -ne 0) { throw "TOC1 DTB inspection failed." }
}

$envImage = Join-Path $OutputDirectory "env-partition.bin"
if (Test-Path -LiteralPath $envImage -PathType Leaf) {
    & $pythonExe @pythonArgs $EnvInspector $envImage -o (Join-Path $OutputDirectory "env-report.json")
    if ($LASTEXITCODE -ne 0) { throw "U-Boot environment inspection failed." }
}

$memorySnapshot = Join-Path $OutputDirectory "debugfs-memory-reservations.txt"
$pageSizeFile = Join-Path $OutputDirectory "page-size.txt"
$liveFdtJson = Join-Path $OutputDirectory "live-fdt.json"
$meminfoFile = Join-Path $OutputDirectory "meminfo.txt"
if (Test-Path -LiteralPath $memorySnapshot -PathType Leaf) {
    $memoryMapArgs = @($MemoryMapInspector, $memorySnapshot)
    if (Test-Path -LiteralPath $pageSizeFile -PathType Leaf) {
        $pageSizeText = (Get-Content -LiteralPath $pageSizeFile -Raw).Trim()
        if ($pageSizeText -match '^[0-9]+$') {
            $memoryMapArgs += @("--page-size", $pageSizeFile)
        } else {
            Write-Warning "Page size was not exported; CMA PFNs will remain raw while memblock byte ranges are still parsed."
        }
    } else {
        Write-Warning "Page-size file is missing; CMA PFNs will remain raw while memblock byte ranges are still parsed."
    }
    if (Test-Path -LiteralPath $meminfoFile -PathType Leaf) {
        $memoryMapArgs += @("--meminfo", $meminfoFile)
    }
    if (Test-Path -LiteralPath $liveFdtJson -PathType Leaf) {
        $memoryMapArgs += @("--fdt-json", $liveFdtJson)
    }
    $memoryMapArgs += @("-o", (Join-Path $OutputDirectory "memory-map-report.json"))
    $memoryMapCommandArgs = @()
    $memoryMapCommandArgs += $pythonArgs
    $memoryMapCommandArgs += $memoryMapArgs
    & $pythonExe @memoryMapCommandArgs
    if ($LASTEXITCODE -ne 0) { throw "CMA/memblock memory map inspection failed." }
}

Write-Host ""
Write-Host "Inventory collected and parsed successfully."
Write-Host "Review: $OutputDirectory"
Write-Host "The device-side collector only reads the source block device; output files were written under /tmp."
