# RepairDoc — kondisi disk fisik (HDD/SSD) menurut Windows.
# Hanya membaca, tidak mengubah apa pun.
# Data umur SSD, suhu, dan jumlah error (MSFT_StorageReliabilityCounter)
# biasanya hanya bisa dibaca dengan izin admin.
$ns = 'root/Microsoft/Windows/Storage'
$r = [ordered]@{ source = $null; reliabilityReadable = $false; disks = @() }

try {
  $pds = @(Get-CimInstance -Namespace $ns -ClassName MSFT_PhysicalDisk -ErrorAction Stop)
  if ($pds.Count -eq 0) { throw 'kosong' }
  $r.source = 'storage'
  $list = @()
  foreach ($d in $pds) {
    $letters = @()
    try {
      $num = [int]$d.DeviceId
      $parts = @(Get-CimInstance -Namespace $ns -ClassName MSFT_Partition -Filter "DiskNumber = $num" -ErrorAction Stop)
      foreach ($p in $parts) {
        $dl = [string]$p.DriveLetter
        if ($dl -match '^[A-Za-z]$') { $letters += $dl.ToUpper() }
      }
    } catch {}

    $rel = $null
    try {
      $rel = Get-CimAssociatedInstance -InputObject $d -ResultClassName MSFT_StorageReliabilityCounter -ErrorAction Stop | Select-Object -First 1
      if ($rel) { $r.reliabilityReadable = $true }
    } catch {}

    $list += [pscustomobject]@{
      id                    = [string]$d.DeviceId
      name                  = ([string]$d.FriendlyName).Trim()
      mediaType             = [string]$d.MediaType
      busType               = [string]$d.BusType
      health                = [string]$d.HealthStatus
      sizeBytes             = [uint64]$d.Size
      letters               = $letters
      wear                  = $(if ($rel) { $rel.Wear } else { $null })
      temperatureC          = $(if ($rel) { $rel.Temperature } else { $null })
      readErrorsUncorrected = $(if ($rel) { $rel.ReadErrorsUncorrected } else { $null })
      powerOnHours          = $(if ($rel) { $rel.PowerOnHours } else { $null })
    }
  }
  $r.disks = $list
} catch {
  # Cadangan untuk PC yang tidak punya modul Storage: Win32_DiskDrive (status SMART dasar).
  try {
    $dd = @(Get-CimInstance -ClassName Win32_DiskDrive -ErrorAction Stop)
    $r.source = 'win32'
    $list = @()
    foreach ($d in $dd) {
      $list += [pscustomobject]@{
        id = [string]$d.Index; name = ([string]$d.Model).Trim(); mediaType = ''
        busType = [string]$d.InterfaceType; health = [string]$d.Status; sizeBytes = [uint64]$d.Size
        letters = @(); wear = $null; temperatureC = $null; readErrorsUncorrected = $null; powerOnHours = $null
      }
    }
    $r.disks = $list
  } catch {}
}

$r | ConvertTo-Json -Compress -Depth 5
