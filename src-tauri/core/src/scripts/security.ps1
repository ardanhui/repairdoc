# RepairDoc — keamanan: Windows Defender / antivirus lain, firewall, UAC, daftar program terpasang.
# Hanya membaca, tidak mengubah apa pun.
$r = [ordered]@{
  defender = $null; avProducts = @()
  firewall = [ordered]@{ domain = $null; private = $null; public = $null }
  uacEnabled = $null; programs = @()
}

try {
  $d = Get-MpComputerStatus -ErrorAction Stop
  $r.defender = [pscustomobject]@{
    serviceEnabled   = [bool]$d.AMServiceEnabled
    antivirusEnabled = [bool]$d.AntivirusEnabled
    realTime         = [bool]$d.RealTimeProtectionEnabled
    signatureAgeDays = $(if ($null -ne $d.AntivirusSignatureAge) { [int64]$d.AntivirusSignatureAge } else { $null })
    signatureDate    = $(if ($d.AntivirusSignatureLastUpdated) { ([datetime]$d.AntivirusSignatureLastUpdated).ToString('yyyy-MM-dd') } else { $null })
    quickScanAgeDays = $(if ($null -ne $d.QuickScanAge) { [int64]$d.QuickScanAge } else { $null })
    fullScanAgeDays  = $(if ($null -ne $d.FullScanAge) { [int64]$d.FullScanAge } else { $null })
  }
} catch {}

# Antivirus yang terdaftar di Windows Security Center (termasuk Defender sendiri).
try {
  $list = @()
  foreach ($a in @(Get-CimInstance -Namespace root/SecurityCenter2 -ClassName AntiVirusProduct -ErrorAction Stop)) {
    $hex = ('{0:x6}' -f [int64]$a.productState)
    $list += [pscustomobject]@{
      name     = [string]$a.displayName
      state    = $hex
      enabled  = ($hex.Substring(2, 2) -eq '10' -or $hex.Substring(2, 2) -eq '11')
      upToDate = ($hex.Substring(4, 2) -eq '00')
    }
  }
  $r.avProducts = $list
} catch {}

# Firewall: status efektif per profil; cadangan dari registry bila cmdlet tidak ada.
try {
  foreach ($p in @(Get-NetFirewallProfile -ErrorAction Stop)) {
    $key = ([string]$p.Name).ToLower()
    if ($r.firewall.Contains($key)) { $r.firewall[$key] = [bool]$p.Enabled }
  }
} catch {}
$fwBase = 'HKLM:\SYSTEM\CurrentControlSet\Services\SharedAccess\Parameters\FirewallPolicy'
foreach ($pair in @(@('domain', 'DomainProfile'), @('private', 'StandardProfile'), @('public', 'PublicProfile'))) {
  if ($null -ne $r.firewall[$pair[0]]) { continue }
  try {
    $v = (Get-ItemProperty -Path "$fwBase\$($pair[1])" -ErrorAction Stop).EnableFirewall
    if ($null -ne $v) { $r.firewall[$pair[0]] = ([int]$v -eq 1) }
  } catch {}
}

try {
  $u = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System' -ErrorAction Stop
  if ($null -ne $u.EnableLUA) { $r.uacEnabled = ([int]$u.EnableLUA -eq 1) }
} catch {}

# Daftar program terpasang (sumber yang sama dengan Settings → Apps).
try {
  $paths = @(
    'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*',
    'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*'
  )
  $list = @()
  foreach ($p in $paths) {
    foreach ($i in @(Get-ItemProperty -Path $p -ErrorAction SilentlyContinue)) {
      if (-not $i.DisplayName) { continue }
      if ($i.SystemComponent -eq 1) { continue }
      $list += [pscustomobject]@{
        name        = ([string]$i.DisplayName).Trim()
        publisher   = ([string]$i.Publisher).Trim()
        installDate = [string]$i.InstallDate
        sizeKb      = $(if ($i.EstimatedSize) { [int64]$i.EstimatedSize } else { $null })
      }
    }
  }
  $r.programs = @($list | Sort-Object -Property name -Unique)
} catch {}

$r | ConvertTo-Json -Compress -Depth 4
