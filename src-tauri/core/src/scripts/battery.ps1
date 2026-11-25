# RepairDoc — baterai laptop: kapasitas sekarang vs kapasitas saat baru.
# Hanya membaca, tidak mengubah apa pun.
$r = [ordered]@{
  chassis = @(); hasBattery = $false; chargePct = $null; statusCode = $null; onAc = $null
  fullCapacity = $null; designCapacity = $null; cycles = $null
}

try {
  foreach ($e in @(Get-CimInstance -ClassName Win32_SystemEnclosure -ErrorAction Stop)) {
    foreach ($c in @($e.ChassisTypes)) { $r.chassis += [int]$c }
  }
} catch {}

try {
  $b = @(Get-CimInstance -ClassName Win32_Battery -ErrorAction Stop)
  if ($b.Count -gt 0) {
    $r.hasBattery = $true
    if ($null -ne $b[0].EstimatedChargeRemaining) { $r.chargePct = [int]$b[0].EstimatedChargeRemaining }
    if ($null -ne $b[0].BatteryStatus) { $r.statusCode = [int]$b[0].BatteryStatus }
  }
} catch {}

try {
  $bs = @(Get-CimInstance -Namespace root/wmi -ClassName BatteryStatus -ErrorAction Stop)
  if ($bs.Count -gt 0) { $r.onAc = [bool]$bs[0].PowerOnline }
} catch {}
try {
  $f = @(Get-CimInstance -Namespace root/wmi -ClassName BatteryFullChargedCapacity -ErrorAction Stop)
  if ($f.Count -gt 0 -and $f[0].FullChargedCapacity) { $r.fullCapacity = [int64]$f[0].FullChargedCapacity }
} catch {}
try {
  $s = @(Get-CimInstance -Namespace root/wmi -ClassName BatteryStaticData -ErrorAction Stop)
  if ($s.Count -gt 0 -and $s[0].DesignedCapacity) { $r.designCapacity = [int64]$s[0].DesignedCapacity }
} catch {}
try {
  $c = @(Get-CimInstance -Namespace root/wmi -ClassName BatteryCycleCount -ErrorAction Stop)
  if ($c.Count -gt 0 -and $c[0].CycleCount) { $r.cycles = [int64]$c[0].CycleCount }
} catch {}

$r | ConvertTo-Json -Compress -Depth 3
