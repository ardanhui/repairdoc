# RepairDoc — baca suhu dari sensor "thermal zone" (ACPI) motherboard.
# Hanya membaca, tidak mengubah apa pun.
# Catatan: ini suhu zona termal, belum tentu suhu inti prosesor. Banyak PC
# tidak menyediakan sensor ini sama sekali.
$r = [ordered]@{ perf = $null; acpi = $null }

# 1) Penghitung kinerja Windows — biasanya bisa dibaca tanpa izin admin.
try {
  $zones = @(Get-CimInstance -ClassName Win32_PerfFormattedData_Counters_ThermalZoneInformation -ErrorAction Stop)
  $vals = @()
  foreach ($z in $zones) {
    if ($z.HighPrecisionTemperature -gt 0) { $vals += ($z.HighPrecisionTemperature / 10.0) - 273.15 }
    elseif ($z.Temperature -gt 0) { $vals += $z.Temperature - 273.15 }
  }
  if ($vals.Count -gt 0) { $r.perf = [math]::Round((($vals | Measure-Object -Maximum).Maximum), 1) }
} catch {}

# 2) WMI ACPI — biasanya butuh izin admin.
try {
  $acpi = @(Get-CimInstance -Namespace root/wmi -ClassName MSAcpi_ThermalZoneTemperature -ErrorAction Stop)
  $vals = @($acpi | Where-Object { $_.CurrentTemperature -gt 0 } | ForEach-Object { ($_.CurrentTemperature / 10.0) - 273.15 })
  if ($vals.Count -gt 0) { $r.acpi = [math]::Round((($vals | Measure-Object -Maximum).Maximum), 1) }
} catch {}

$r | ConvertTo-Json -Compress
