# RepairDoc — inventaris untuk Mode Performa. HANYA MEMBACA, tidak mengubah apa pun.
# (-DryRun diterima demi keseragaman, tapi skrip ini memang tidak pernah menulis.)
#   - identitas PC (MachineGuid), supaya catatan perubahan di flashdisk tidak tertukar antar-PC;
#   - program startup di kunci Run + status StartupApproved-nya (sama dengan tab Startup Task Manager);
#   - (tahap 4) shortcut di folder Startup pengguna & semua pengguna (+ status StartupApproved\StartupFolder);
#   - (tahap 4) tugas terjadwal yang punya pemicu "saat masuk Windows" (logon).
#   Sumber yang SENGAJA tidak dibaca: RunOnce (sekali jalan, dipakai installer),
#   Winlogon/Userinit/Shell (terlalu berbahaya), dan service (ditangani tweak lain).
#   - aplikasi Microsoft Store milik pengguna + izin jalan di latar (BackgroundAccessApplications).
# -MachineOnly : hanya identitas PC (cepat).
param([switch]$DryRun, [switch]$MachineOnly)

$r = [ordered]@{ machine = [ordered]@{ machineId = $null; computerName = [string]$env:COMPUTERNAME }; startup = @(); apps = @(); globalBgDisabled = $null; startupFiles = @(); logonTasks = @() }

try {
  $r.machine.machineId = [string](Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Cryptography' -Name MachineGuid -ErrorAction Stop).MachineGuid
} catch {}

function ConvertTo-Hex([byte[]]$b) { return (($b | ForEach-Object { $_.ToString('x2') }) -join '') }

if (-not $MachineOnly) {
  $sources = @(
    @{ id = 'HKCU';   hive = 'HKCU'; run = 'Software\Microsoft\Windows\CurrentVersion\Run';             approved = 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run' },
    @{ id = 'HKLM';   hive = 'HKLM'; run = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Run';             approved = 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run' },
    @{ id = 'HKLM32'; hive = 'HKLM'; run = 'SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Run'; approved = 'Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32' }
  )
  $list = @()
  foreach ($s in $sources) {
    try {
      $root = if ($s.hive -eq 'HKLM') { [Microsoft.Win32.Registry]::LocalMachine } else { [Microsoft.Win32.Registry]::CurrentUser }
      $rk = $root.OpenSubKey($s.run, $false)
      if ($null -eq $rk) { continue }
      $ak = $root.OpenSubKey($s.approved, $false)
      $approvedNames = @()
      if ($ak) { $approvedNames = $ak.GetValueNames() }
      foreach ($n in $rk.GetValueNames()) {
        if ([string]::IsNullOrEmpty($n)) { continue }
        $hex = $null
        if ($approvedNames -contains $n) {
          $v = $ak.GetValue($n)
          if ($v -is [byte[]]) { $hex = ConvertTo-Hex $v }
        }
        $list += [pscustomobject]@{ source = $s.id; name = $n; command = [string]$rk.GetValue($n, '', 'DoNotExpandEnvironmentNames'); approvedHex = $hex }
      }
      $rk.Close()
      if ($ak) { $ak.Close() }
    } catch {}
  }
  $r.startup = $list

  # --- Folder Startup (tahap 4). Jalurnya sama dengan yang dipakai RepairDoc saat memindah
  #     shortcut: %APPDATA%\...\Startup dan %ProgramData%\...\StartUp. Hanya file .lnk biasa;
  #     symlink/junction dilewati.
  $shell = $null
  try { $shell = New-Object -ComObject WScript.Shell } catch {}
  $folders = @(
    @{ scope = 'user';   dir = (Join-Path ([string]$env:APPDATA) 'Microsoft\Windows\Start Menu\Programs\Startup');    root = [Microsoft.Win32.Registry]::CurrentUser },
    @{ scope = 'common'; dir = (Join-Path ([string]$env:ProgramData) 'Microsoft\Windows\Start Menu\Programs\StartUp'); root = [Microsoft.Win32.Registry]::LocalMachine }
  )
  $files = @()
  foreach ($f in $folders) {
    try {
      if (-not (Test-Path -LiteralPath $f.dir -PathType Container)) { continue }
      $ak = $f.root.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\StartupFolder', $false)
      $approvedNames = @()
      if ($ak) { $approvedNames = $ak.GetValueNames() }
      foreach ($file in @(Get-ChildItem -LiteralPath $f.dir -File -Force -ErrorAction SilentlyContinue)) {
        if ($file.Extension -ne '.lnk') { continue }
        if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) { continue }
        $target = ''
        if ($shell) {
          try {
            $lnk = $shell.CreateShortcut($file.FullName)
            $target = (([string]$lnk.TargetPath) + ' ' + ([string]$lnk.Arguments)).Trim()
          } catch {}
        }
        $hex = $null
        if ($approvedNames -contains $file.Name) {
          $v = $ak.GetValue($file.Name)
          if ($v -is [byte[]]) { $hex = ConvertTo-Hex $v }
        }
        $files += [pscustomobject]@{ scope = $f.scope; name = [string]$file.Name; target = $target; approvedHex = $hex }
      }
      if ($ak) { $ak.Close() }
    } catch {}
  }
  $r.startupFiles = $files

  # --- Tugas terjadwal dengan pemicu logon (tahap 4). Hanya dibaca; penyaringan
  #     (hanya pihak ketiga di Program Files / AppData) dilakukan RepairDoc.
  $tasks = @()
  try {
    foreach ($t in @(Get-ScheduledTask -ErrorAction Stop)) {
      $logon = @($t.Triggers | Where-Object { $_ -and $_.CimClass.CimClassName -eq 'MSFT_TaskLogonTrigger' -and $_.Enabled -ne $false })
      if ($logon.Count -eq 0) { continue }
      $acts = @()
      foreach ($a in @($t.Actions)) {
        if ($a -and $a.CimClass.CimClassName -eq 'MSFT_TaskExecAction') {
          $exe = ([Environment]::ExpandEnvironmentVariables([string]$a.Execute)).Trim().Trim('"')
          $acts += [pscustomobject]@{ execute = $exe; arguments = [string]$a.Arguments }
        } else {
          # Aksi selain "jalankan program" (COM handler, dsb.) → kosong = bukan pihak ketiga.
          $acts += [pscustomobject]@{ execute = ''; arguments = '' }
        }
      }
      $tasks += [pscustomobject]@{ path = [string]$t.TaskPath; name = [string]$t.TaskName; state = [string]$t.State; author = [string]$t.Author; actions = $acts }
    }
  } catch {}
  $r.logonTasks = $tasks

  $bgPath = 'Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications'
  try {
    $g = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($bgPath, $false)
    if ($g) { $gv = $g.GetValue('GlobalUserDisabled'); if ($null -ne $gv) { $r.globalBgDisabled = [int]$gv }; $g.Close() }
  } catch {}

  $apps = @()
  try {
    foreach ($p in @(Get-AppxPackage -ErrorAction Stop)) {
      if ($p.IsFramework) { continue }
      $d = $null; $du = $null
      $k = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("$bgPath\$($p.PackageFamilyName)", $false)
      if ($k) {
        $v = $k.GetValue('Disabled'); if ($null -ne $v) { $d = [int]$v }
        $v = $k.GetValue('DisabledByUser'); if ($null -ne $v) { $du = [int]$v }
        $k.Close()
      }
      $display = $null
      $kind = [string]$p.SignatureKind
      # Nama tampilan hanya dicari untuk aplikasi pihak lain (lebih cepat).
      if ($kind -eq 'Store' -and -not $p.NonRemovable -and -not ([string]$p.Name).StartsWith('Microsoft')) {
        try {
          $dn = [string](Get-AppxPackageManifest -Package $p.PackageFullName -ErrorAction Stop).Package.Properties.DisplayName
          if ($dn -and -not $dn.StartsWith('ms-resource:')) { $display = $dn }
        } catch {}
      }
      $apps += [pscustomobject]@{
        name = [string]$p.Name; familyName = [string]$p.PackageFamilyName; displayName = $display
        signatureKind = $kind; nonRemovable = [bool]$p.NonRemovable; disabled = $d; disabledByUser = $du
      }
    }
  } catch {}
  $r.apps = $apps
}

$r | ConvertTo-Json -Depth 6 -Compress
