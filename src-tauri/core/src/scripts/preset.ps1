# RepairDoc — menerapkan (atau mensimulasikan) perubahan Mode Performa.
#
# Daftar perubahan dikirim RepairDoc lewat variabel lingkungan REPAIRDOC_OPS
# (JSON dalam base64). Tiap perubahan: { key, target, value }.
#   target.kind = 'registry' (hive, path, name) | 'service' (name) | 'powerPlan'
#   value.type  = 'absent' | 'dword' | 'string' | 'binary' | 'service' | 'power'
#
# -DryRun  : HANYA membaca nilai sekarang dan melaporkan apa yang AKAN berubah.
#            Tidak menulis apa pun.
# tanpa -DryRun : tiap perubahan -> baca nilai lama -> tulis -> baca ulang untuk memastikan.
#
# Pengaman (selalu aktif, juga saat -DryRun):
#   - registry hanya boleh ditulis di bawah kunci di $Allowed;
#   - service di $Protected selalu ditolak;
#   - nilai registry berjenis selain DWORD/String/Binary tidak diubah (supaya bisa dikembalikan persis).
# Keluaran: JSON berisi daftar { key, before, after, status, error }.
#   status = changed | same | planned | notpresent | refused | failed
param([switch]$DryRun, [string]$OpsBase64 = $env:REPAIRDOC_OPS)

$Allowed = @(
  'HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\VisualEffects',
  'HKCU\Control Panel\Desktop',
  'HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced',
  'HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize',
  'HKCU\System\GameConfigStore',
  'HKCU\Software\Microsoft\Windows\CurrentVersion\GameDVR',
  'HKLM\SOFTWARE\Policies\Microsoft\Windows\GameDVR',
  'HKCU\Software\Microsoft\Windows\CurrentVersion\BackgroundAccessApplications',
  'HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run',
  'HKLM\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run',
  'HKLM\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run32',
  'HKCU\Software\RepairDocTest'
)
$Protected = @('wuauserv','WinDefend','MpsSvc','WlanSvc','Audiosrv','Spooler','WSearch','SysMain','BITS','Dhcp','Dnscache','EventLog','PlugPlay','Power','RpcSs','AudioEndpointBuilder','SecurityHealthService','wscsvc','Winmgmt','TrustedInstaller','UsoSvc','WaaSMedicSvc')
$Sc = Join-Path $env:SystemRoot 'System32\sc.exe'
$PowerCfg = Join-Path $env:SystemRoot 'System32\powercfg.exe'

function Get-Root([string]$hive) {
  if ($hive -eq 'HKLM') { return [Microsoft.Win32.Registry]::LocalMachine }
  return [Microsoft.Win32.Registry]::CurrentUser
}

function Test-Allowed($t) {
  $full = ([string]$t.hive + '\' + [string]$t.path).ToLowerInvariant()
  foreach ($a in $Allowed) {
    $a = $a.ToLowerInvariant()
    if ($full -eq $a -or $full.StartsWith($a + '\')) { return $true }
  }
  return $false
}

function ConvertTo-Hex([byte[]]$b) { return (($b | ForEach-Object { $_.ToString('x2') }) -join '') }

function ConvertFrom-Hex([string]$hex) {
  $b = New-Object byte[] ($hex.Length / 2)
  for ($i = 0; $i -lt $b.Length; $i++) { $b[$i] = [Convert]::ToByte($hex.Substring($i * 2, 2), 16) }
  return ,$b
}

function Read-Reg($t) {
  $k = (Get-Root $t.hive).OpenSubKey([string]$t.path, $false)
  if ($null -eq $k) { return @{ type = 'absent'; keyExisted = $false } }
  try {
    if (-not ($k.GetValueNames() -contains [string]$t.name)) { return @{ type = 'absent'; keyExisted = $true } }
    $kind = [string]$k.GetValueKind([string]$t.name)
    $v = $k.GetValue([string]$t.name, $null, 'DoNotExpandEnvironmentNames')
    switch ($kind) {
      'DWord'  { return @{ type = 'dword'; value = [BitConverter]::ToUInt32([BitConverter]::GetBytes([int32]$v), 0) } }
      'String' { return @{ type = 'string'; value = [string]$v } }
      'Binary' { return @{ type = 'binary'; hex = (ConvertTo-Hex $v) } }
      default  { return @{ type = 'unsupported'; kind = $kind } }
    }
  } finally { $k.Close() }
}

function Write-Reg($t, $val) {
  $root = Get-Root $t.hive
  $path = [string]$t.path
  $name = [string]$t.name
  if ($val.type -eq 'absent') {
    $k = $root.OpenSubKey($path, $true)
    if ($null -eq $k) { return }
    $empty = $false
    try {
      $k.DeleteValue($name, $false)
      $empty = ($k.ValueCount -eq 0 -and $k.SubKeyCount -eq 0)
    } finally { $k.Close() }
    # Kunci yang dulu tidak ada dan sekarang kosong ikut dihapus, supaya kembali persis.
    if ($empty -and $val.keyExisted -eq $false) { $root.DeleteSubKey($path, $false) }
    return
  }
  $k = $root.CreateSubKey($path)
  try {
    switch ([string]$val.type) {
      'dword'  { $k.SetValue($name, [BitConverter]::ToInt32([BitConverter]::GetBytes([uint32]$val.value), 0), [Microsoft.Win32.RegistryValueKind]::DWord) }
      'string' { $k.SetValue($name, [string]$val.value, [Microsoft.Win32.RegistryValueKind]::String) }
      'binary' { $k.SetValue($name, [byte[]](ConvertFrom-Hex ([string]$val.hex)), [Microsoft.Win32.RegistryValueKind]::Binary) }
      default  { throw "Jenis nilai '$($val.type)' tidak didukung." }
    }
  } finally { $k.Close() }
}

function Read-Service([string]$name) {
  $out = (& $Sc qc $name 2>&1) | Out-String
  if ($LASTEXITCODE -eq 1060) { return @{ type = 'missing' } }
  if ($LASTEXITCODE -ne 0) { throw "sc.exe qc $name gagal (kode $LASTEXITCODE)." }
  if ($out -notmatch 'START_TYPE\s*:\s*(\d+)') { throw "Jenis start $name tidak terbaca." }
  $n = [int]$Matches[1]
  $start = switch ($n) {
    0 { 'boot' }
    1 { 'system' }
    2 { if ($out -match 'DELAYED') { 'delayed-auto' } else { 'auto' } }
    3 { 'demand' }
    4 { 'disabled' }
    default { "kode$n" }
  }
  return @{ type = 'service'; start = $start }
}

function Write-Service([string]$name, [string]$start) {
  if ($Protected -contains $name) { throw "Service $name dilindungi." }
  if (@('auto', 'delayed-auto', 'demand', 'disabled') -notcontains $start) { throw "Jenis start '$start' tidak dikenal." }
  $out = (& $Sc config $name start= $start 2>&1) | Out-String
  if ($LASTEXITCODE -ne 0) { throw "sc.exe config $name gagal (kode $LASTEXITCODE): $($out.Trim())" }
}

function Read-Power {
  $t = (& $PowerCfg /getactivescheme 2>$null) | Out-String
  if ($t -match '([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})') {
    return @{ type = 'power'; guid = $Matches[1].ToLowerInvariant() }
  }
  throw 'Power plan aktif tidak terbaca.'
}

function Test-PowerExists([string]$guid) {
  $list = (& $PowerCfg /list 2>$null) | Out-String
  return ($list.ToLowerInvariant().Contains($guid.ToLowerInvariant()))
}

function Test-Same($a, $b) {
  if ($null -eq $a -or $null -eq $b) { return $false }
  if ([string]$a.type -ne [string]$b.type) { return $false }
  switch ([string]$a.type) {
    'absent'  { return $true }
    'missing' { return $true }
    'dword'   { return ([uint32]$a.value -eq [uint32]$b.value) }
    'string'  { return ([string]$a.value -ceq [string]$b.value) }
    'binary'  { return (([string]$a.hex).ToLowerInvariant() -eq ([string]$b.hex).ToLowerInvariant()) }
    'service' { return ([string]$a.start -eq [string]$b.start) }
    'power'   { return (([string]$a.guid).ToLowerInvariant() -eq ([string]$b.guid).ToLowerInvariant()) }
  }
  return $false
}

$ops = @()
if ($OpsBase64) {
  $json = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($OpsBase64))
  # -InputObject (bukan pipeline): di PowerShell 5.1, array JSON lewat pipeline
  # menjadi satu objek, sehingga foreach hanya berjalan sekali.
  $ops = ConvertFrom-Json -InputObject $json
}

$results = @()
foreach ($op in $ops) {
  $r = [ordered]@{ key = [string]$op.key; before = $null; after = $null; status = 'failed'; error = $null }
  try {
    $t = $op.target
    $want = $op.value
    switch ([string]$t.kind) {
      'registry' {
        if (-not (Test-Allowed $t)) { $r.status = 'refused'; $r.error = 'Lokasi registry ini tidak ada di daftar izin RepairDoc.'; break }
        $r.before = Read-Reg $t
        if ($r.before.type -eq 'unsupported') { $r.error = "Jenis nilai $($r.before.kind) tidak diubah supaya bisa dikembalikan persis."; break }
        if (Test-Same $r.before $want) { $r.status = 'same'; break }
        if ($DryRun) { $r.status = 'planned'; break }
        Write-Reg $t $want
        $r.after = Read-Reg $t
        if (Test-Same $r.after $want) { $r.status = 'changed' } else { $r.error = 'Nilai setelah ditulis tidak sesuai.' }
      }
      'service' {
        $name = [string]$t.name
        if ($Protected -contains $name) { $r.status = 'refused'; $r.error = "Service $name dilindungi dan tidak boleh diubah."; break }
        $r.before = Read-Service $name
        if ($r.before.type -eq 'missing') { $r.status = 'notpresent'; $r.error = 'Service ini tidak ada di PC ini.'; break }
        if (Test-Same $r.before $want) { $r.status = 'same'; break }
        if ($DryRun) { $r.status = 'planned'; break }
        Write-Service $name ([string]$want.start)
        $r.after = Read-Service $name
        if (Test-Same $r.after $want) { $r.status = 'changed' } else { $r.error = 'Jenis start setelah diubah tidak sesuai.' }
      }
      'powerPlan' {
        $r.before = Read-Power
        if (Test-Same $r.before $want) { $r.status = 'same'; break }
        if (-not (Test-PowerExists ([string]$want.guid))) { $r.status = 'notpresent'; $r.error = 'Power plan ini tidak tersedia di PC ini.'; break }
        if ($DryRun) { $r.status = 'planned'; break }
        & $PowerCfg /setactive ([string]$want.guid) | Out-Null
        $r.after = Read-Power
        if (Test-Same $r.after $want) { $r.status = 'changed' } else { $r.error = 'Power plan tidak berubah.' }
      }
      default { $r.error = "Jenis perubahan '$($t.kind)' tidak dikenal." }
    }
  } catch {
    $r.status = 'failed'
    $r.error = $_.Exception.Message
  }
  $results += [pscustomobject]$r
}

ConvertTo-Json -InputObject @($results) -Depth 6 -Compress
