# RepairDoc — membuat System Restore Point sebelum Mode Performa mengubah apa pun.
# Nama: "<awalan> <tanggal jam>", mis. "RepairDoc Mode Kentang 2026-11-26 13:50".
# -DryRun : hanya memeriksa apakah perintahnya tersedia dan mengembalikan nama yang
#           akan dipakai. Tidak membuat apa pun.
#
# Windows menolak membuat restore point kalau sudah ada yang dibuat 24 jam terakhir.
# Untuk restore point ini saja, batas itu dimatikan sementara (nilai
# SystemRestorePointCreationFrequency = 0, cara yang didokumentasikan Microsoft),
# lalu dikembalikan persis seperti semula (blok finally), berhasil ataupun gagal.
# Keberhasilan dicek ulang: restore point baru harus muncul di Get-ComputerRestorePoint.
param([switch]$DryRun, [string]$Prefix = $env:REPAIRDOC_RP_PREFIX)

if (-not $Prefix) { $Prefix = 'RepairDoc' }
$name = $Prefix + ' ' + (Get-Date -Format 'yyyy-MM-dd HH:mm')
$r = [ordered]@{ ok = $false; name = $name; dryRun = [bool]$DryRun; sequence = $null; error = $null }

if (-not (Get-Command Checkpoint-Computer -ErrorAction SilentlyContinue)) {
  $r.error = 'Perintah Checkpoint-Computer tidak tersedia di Windows ini.'
  $r | ConvertTo-Json -Compress
  return
}
if ($DryRun) {
  $r.ok = $true
  $r | ConvertTo-Json -Compress
  return
}

$srKey = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\SystemRestore'
$hadFreq = $false
$oldFreq = $null
try {
  $oldFreq = (Get-ItemProperty -Path $srKey -Name SystemRestorePointCreationFrequency -ErrorAction Stop).SystemRestorePointCreationFrequency
  $hadFreq = $true
} catch {}

$lastSeq = 0
try {
  $m = (@(Get-ComputerRestorePoint -ErrorAction Stop) | Measure-Object -Property SequenceNumber -Maximum).Maximum
  if ($m) { $lastSeq = [int64]$m }
} catch {}

try {
  if (-not (Test-Path $srKey)) { New-Item -Path $srKey -Force | Out-Null }
  New-ItemProperty -Path $srKey -Name SystemRestorePointCreationFrequency -Value 0 -PropertyType DWord -Force | Out-Null
  Checkpoint-Computer -Description $name -RestorePointType MODIFY_SETTINGS -ErrorAction Stop -WarningAction SilentlyContinue
  $made = @(Get-ComputerRestorePoint -ErrorAction Stop | Where-Object { $_.Description -eq $name -and [int64]$_.SequenceNumber -gt $lastSeq }) | Select-Object -Last 1
  if ($made) {
    $r.ok = $true
    $r.sequence = [int64]$made.SequenceNumber
  } else {
    $r.error = 'Windows tidak membuat restore point (tidak muncul di daftar setelah perintah dijalankan). System Restore mungkin mati.'
  }
} catch {
  $r.error = $_.Exception.Message
} finally {
  try {
    if ($hadFreq) { Set-ItemProperty -Path $srKey -Name SystemRestorePointCreationFrequency -Value $oldFreq -Type DWord }
    else { Remove-ItemProperty -Path $srKey -Name SystemRestorePointCreationFrequency -ErrorAction SilentlyContinue }
  } catch {}
}

$r | ConvertTo-Json -Compress
