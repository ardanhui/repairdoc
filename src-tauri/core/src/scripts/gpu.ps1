# RepairDoc — kartu grafis: nama, driver, kode error Device Manager.
# Hanya membaca, tidak mengubah apa pun.
$r = [ordered]@{ adapters = @(); nvidia = $null }

try {
  $vcs = @(Get-CimInstance -ClassName Win32_VideoController -ErrorAction Stop)
  $list = @()
  foreach ($v in $vcs) {
    $date = $null
    if ($v.DriverDate) { try { $date = ([datetime]$v.DriverDate).ToString('yyyy-MM-dd') } catch {} }
    $list += [pscustomobject]@{
      name          = ([string]$v.Name).Trim()
      vramBytes     = $(if ($v.AdapterRAM) { [uint64]$v.AdapterRAM } else { $null })
      driverVersion = [string]$v.DriverVersion
      driverDate    = $date
      status        = [string]$v.Status
      errorCode     = $(if ($null -ne $v.ConfigManagerErrorCode) { [int]$v.ConfigManagerErrorCode } else { $null })
      width         = $(if ($v.CurrentHorizontalResolution) { [int]$v.CurrentHorizontalResolution } else { $null })
      height        = $(if ($v.CurrentVerticalResolution) { [int]$v.CurrentVerticalResolution } else { $null })
    }
  }
  $r.adapters = $list
} catch {}

# Suhu hanya bisa dibaca untuk NVIDIA (lewat nvidia-smi yang ikut terpasang bersama drivernya).
foreach ($smi in @("$env:SystemRoot\System32\nvidia-smi.exe", "$env:ProgramFiles\NVIDIA Corporation\NVSMI\nvidia-smi.exe")) {
  if ($r.nvidia -or -not (Test-Path $smi)) { continue }
  try {
    $out = & $smi --query-gpu=name,temperature.gpu,utilization.gpu,driver_version --format=csv,noheader,nounits 2>$null
    if ($LASTEXITCODE -eq 0 -and $out) {
      $parts = ([string]($out | Select-Object -First 1)).Split(',') | ForEach-Object { $_.Trim() }
      if ($parts.Count -ge 3) {
        $r.nvidia = [pscustomobject]@{
          name           = $parts[0]
          temperatureC   = $(try { [double]$parts[1] } catch { $null })
          utilizationPct = $(try { [double]$parts[2] } catch { $null })
          driver         = $(if ($parts.Count -ge 4) { $parts[3] } else { $null })
        }
      }
    }
  } catch {}
}

$r | ConvertTo-Json -Compress -Depth 4
