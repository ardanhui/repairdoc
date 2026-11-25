# RepairDoc — info RAM terpasang, slot, dan hasil tes memori Windows terakhir.
# Hanya membaca, tidak mengubah apa pun.
$r = [ordered]@{ installedBytes = $null; slotsTotal = $null; slotsUsed = $null; modules = @(); memtest = $null }

try {
  $mods = @(Get-CimInstance -ClassName Win32_PhysicalMemory -ErrorAction Stop)
  if ($mods.Count -gt 0) {
    $sum = [uint64]0
    $list = @()
    foreach ($m in $mods) {
      $sum += [uint64]$m.Capacity
      $speed = $m.ConfiguredClockSpeed
      if (-not $speed) { $speed = $m.Speed }
      $list += [pscustomobject]@{
        capacityBytes = [uint64]$m.Capacity
        speedMhz      = $speed
        maker         = ([string]$m.Manufacturer).Trim()
        slot          = ([string]$m.DeviceLocator).Trim()
      }
    }
    $r.installedBytes = $sum
    $r.slotsUsed = $mods.Count
    $r.modules = $list
  }
} catch {}

try {
  $arrays = @(Get-CimInstance -ClassName Win32_PhysicalMemoryArray -ErrorAction Stop)
  $sys = @($arrays | Where-Object { $_.Use -eq 3 })
  if ($sys.Count -eq 0) { $sys = $arrays }
  $slots = ($sys | Measure-Object -Property MemoryDevices -Sum).Sum
  if ($slots -gt 0) { $r.slotsTotal = [int]$slots }
} catch {}

# Hasil Windows Memory Diagnostic (mdsched.exe): ID 1201 = tidak ada error, 1202 = ada error.
try {
  $ev = Get-WinEvent -FilterHashtable @{ LogName = 'System'; ProviderName = 'Microsoft-Windows-MemoryDiagnostics-Results' } -MaxEvents 1 -ErrorAction Stop
  if ($ev) { $r.memtest = [ordered]@{ eventId = [int]$ev.Id; date = $ev.TimeCreated.ToString('yyyy-MM-dd') } }
} catch {}

$r | ConvertTo-Json -Compress -Depth 4
