# RepairDoc — jaringan: profil koneksi, adapter aktif, sinyal Wi-Fi, server DNS.
# Hanya membaca, tidak mengubah apa pun. Tes internet & kecepatan dilakukan di RepairDoc sendiri.
$r = [ordered]@{ profiles = @(); adapters = @(); wifiSignalPct = $null; wifiSsid = $null; dnsServers = @() }

try {
  foreach ($p in @(Get-NetConnectionProfile -ErrorAction Stop)) {
    $r.profiles += [pscustomobject]@{ name = [string]$p.Name; alias = [string]$p.InterfaceAlias; ipv4 = [string]$p.IPv4Connectivity; category = [string]$p.NetworkCategory }
  }
} catch {}

try {
  foreach ($a in @(Get-NetAdapter -Physical -ErrorAction Stop | Where-Object { $_.Status -eq 'Up' })) {
    $desc = [string]$a.InterfaceDescription
    $r.adapters += [pscustomobject]@{
      name        = [string]$a.Name
      description = $desc
      linkSpeed   = [string]$a.LinkSpeed
      wifi        = (([string]$a.PhysicalMediaType) -like '*802.11*' -or $desc -like '*Wi-Fi*' -or $desc -like '*Wireless*' -or $desc -like '*WLAN*')
    }
  }
} catch {}

try {
  $w = (& "$env:SystemRoot\System32\netsh.exe" wlan show interfaces 2>$null) | Out-String
  if ($w -match '(?im)^\s*(Signal|Sinyal)\s*:\s*(\d+)\s*%') { $r.wifiSignalPct = [int]$Matches[2] }
  if ($w -match '(?im)^\s*SSID\s*:\s*(.+?)\s*$') { $r.wifiSsid = $Matches[1] }
} catch {}

try {
  foreach ($d in @(Get-DnsClientServerAddress -AddressFamily IPv4 -ErrorAction Stop)) {
    foreach ($s in @($d.ServerAddresses)) { if ($s -and ($r.dnsServers -notcontains $s)) { $r.dnsServers += [string]$s } }
  }
} catch {}

$r | ConvertTo-Json -Compress -Depth 4
