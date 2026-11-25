# RepairDoc — Windows: versi, update terakhir, driver bermasalah, waktu boot, power plan.
# Hanya membaca, tidak mengubah apa pun.
$r = [ordered]@{
  os = $null
  update = [ordered]@{ lastSuccess = $null; lastHotfix = $null; lastEvent = $null; wuStart = $null; rebootPending = $false }
  powerScheme = $null; fastStartup = $null; pageFileAuto = $null
  pcModel = $null; pnpErrors = @(); boot = $null
}
$inv = [Globalization.CultureInfo]::InvariantCulture

try {
  $os = Get-CimInstance -ClassName Win32_OperatingSystem -ErrorAction Stop
  $cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion' -ErrorAction SilentlyContinue
  $r.os = [pscustomobject]@{
    caption        = ([string]$os.Caption).Trim()
    version        = [string]$os.Version
    build          = [int]$os.BuildNumber
    ubr            = $(if ($cv.UBR) { [int]$cv.UBR } else { $null })
    displayVersion = $(if ($cv.DisplayVersion) { [string]$cv.DisplayVersion } elseif ($cv.ReleaseId) { [string]$cv.ReleaseId } else { $null })
    arch           = [string]$os.OSArchitecture
    installDate    = $(if ($os.InstallDate) { ([datetime]$os.InstallDate).ToString('yyyy-MM-dd') } else { $null })
    lastBoot       = $(if ($os.LastBootUpTime) { ([datetime]$os.LastBootUpTime).ToString('yyyy-MM-dd HH:mm') } else { $null })
  }
} catch {}

try {
  $cs = Get-CimInstance -ClassName Win32_ComputerSystem -ErrorAction Stop
  $r.pcModel = (([string]$cs.Manufacturer).Trim() + ' ' + ([string]$cs.Model).Trim()).Trim()
  if ($null -ne $cs.AutomaticManagedPagefile) { $r.pageFileAuto = [bool]$cs.AutomaticManagedPagefile }
} catch {}

# --- Windows Update: tiga sumber tanggal, nanti diambil yang paling baru ---
try {
  $k = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\Results\Install' -ErrorAction Stop
  if ($k.LastSuccessTime) {
    $t = [string]$k.LastSuccessTime
    try { $r.update.lastSuccess = [datetime]::ParseExact($t, 'yyyy-MM-dd HH:mm:ss', $inv).ToString('yyyy-MM-dd') }
    catch { try { $r.update.lastSuccess = ([datetime]::Parse($t, $inv)).ToString('yyyy-MM-dd') } catch {} }
  }
} catch {}
try {
  $hf = Get-HotFix -ErrorAction Stop | Where-Object { $_.InstalledOn } | Sort-Object InstalledOn -Descending | Select-Object -First 1
  if ($hf) { $r.update.lastHotfix = ([datetime]$hf.InstalledOn).ToString('yyyy-MM-dd') }
} catch {}
try {
  $ev = Get-WinEvent -FilterHashtable @{ LogName = 'System'; ProviderName = 'Microsoft-Windows-WindowsUpdateClient'; Id = 19 } -MaxEvents 1 -ErrorAction Stop
  if ($ev) { $r.update.lastEvent = $ev.TimeCreated.ToString('yyyy-MM-dd') }
} catch {}
try {
  $svc = Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Services\wuauserv' -ErrorAction Stop
  if ($null -ne $svc.Start) { $r.update.wuStart = [int]$svc.Start }   # 2 otomatis, 3 manual, 4 dimatikan
} catch {}
$r.update.rebootPending = (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired') -or
                          (Test-Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending')

# --- Power plan (dibaca dari GUID supaya tidak tergantung bahasa Windows) ---
try {
  $txt = (& "$env:SystemRoot\System32\powercfg.exe" /getactivescheme 2>$null) | Out-String
  if ($txt -match '([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})') { $r.powerScheme = $Matches[1].ToLower() }
} catch {}
try {
  $p = Get-ItemProperty 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Power' -ErrorAction Stop
  if ($null -ne $p.HiberbootEnabled) { $r.fastStartup = ([int]$p.HiberbootEnabled -eq 1) }
} catch {}

# --- Perangkat bertanda seru di Device Manager (kode 22 = sengaja dimatikan, dilewati) ---
try {
  $bad = @(Get-CimInstance -ClassName Win32_PnPEntity -ErrorAction Stop |
    Where-Object { $null -ne $_.ConfigManagerErrorCode -and $_.ConfigManagerErrorCode -ne 0 -and $_.ConfigManagerErrorCode -ne 22 -and $_.ConfigManagerErrorCode -ne 45 })
  $list = @()
  foreach ($d in $bad) {
    $list += [pscustomobject]@{ name = [string]$d.Name; code = [int]$d.ConfigManagerErrorCode; class = [string]$d.PNPClass }
  }
  $r.pnpErrors = $list
} catch {}

# --- Waktu boot terakhir (event 100). Di sebagian PC log ini hanya terbaca admin. ---
try {
  $b = Get-WinEvent -FilterHashtable @{ LogName = 'Microsoft-Windows-Diagnostics-Performance/Operational'; Id = 100 } -MaxEvents 1 -ErrorAction Stop
  if ($b) {
    $x = [xml]$b.ToXml()
    $d = @{}
    foreach ($n in $x.Event.EventData.Data) { $d[[string]$n.Name] = [string]$n.'#text' }
    $r.boot = [pscustomobject]@{
      date       = $b.TimeCreated.ToString('yyyy-MM-dd')
      bootTimeMs = [int64]$d['BootTime']
      mainPathMs = [int64]$d['MainPathBootTime']
      postBootMs = [int64]$d['BootPostBootTime']
    }
  }
} catch {}

$r | ConvertTo-Json -Compress -Depth 5
