# RepairDoc — menghapus sisa Windows lama (C:\Windows.old) dengan Disk Cleanup bawaan Windows.
#
# Hanya dijalankan kalau pengguna mencentang konfirmasi TERPISAH untuk Windows.old
# (Windows.old dibutuhkan untuk kembali ke Windows lama dalam 10 hari setelah upgrade;
# menghapusnya tidak bisa dibatalkan).
#
# Cara: penangan Disk Cleanup "Previous Installations" (sama dengan Settings → Storage →
# "Previous Windows installation(s)") diaktifkan untuk SATU kali jalan lewat nilai
# StateFlags4207 = 2, lalu `cleanmgr /sagerun:4207`. Nilai itu dihapus lagi sesudahnya
# (blok finally), berhasil ataupun gagal. Penangan lain tidak punya nilai ini, jadi tidak ikut.
# RepairDoc TIDAK mengubah izin file atau menghapus Windows.old sendiri — Disk Cleanup yang
# mengurusnya, karena isinya file sistem milik TrustedInstaller.
#
# Pengaman: hanya menerima jalur "<drive>:\Windows.old".
# -DryRun : hanya memeriksa (Windows.old ada? cleanmgr ada? penangannya ada?). Tidak menghapus apa pun.
# Keluaran: JSON { ok, dryRun, exists, cleanmgr, handler, stillExists, error }.
param([switch]$DryRun, [string]$Target = $env:REPAIRDOC_WINOLD)

$r = [ordered]@{ ok = $false; dryRun = [bool]$DryRun; exists = $false; cleanmgr = $false; handler = $false; stillExists = $null; error = $null }
$cm = Join-Path $env:SystemRoot 'System32\cleanmgr.exe'
$key = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\VolumeCaches\Previous Installations'
$flag = 'StateFlags4207'

if (-not ($Target -match '^[A-Za-z]:\\Windows\.old$')) {
  $r.error = "Jalur '$Target' bukan folder Windows.old di akar drive."
  $r | ConvertTo-Json -Compress
  return
}
$r.exists = [bool](Test-Path -LiteralPath $Target -PathType Container)
$r.cleanmgr = [bool](Test-Path -LiteralPath $cm)
$r.handler = [bool](Test-Path -LiteralPath $key)
$r.stillExists = $r.exists
if (-not $r.exists) {
  $r.error = 'Folder Windows.old tidak ada.'
  $r | ConvertTo-Json -Compress
  return
}
if (-not $r.cleanmgr -or -not $r.handler) {
  $r.error = 'Disk Cleanup (cleanmgr) atau penangan "Previous Installations" tidak ada di Windows ini. Hapus lewat Settings → System → Storage → Temporary files.'
  $r | ConvertTo-Json -Compress
  return
}
if ($DryRun) {
  $r.ok = $true
  $r | ConvertTo-Json -Compress
  return
}

try {
  New-ItemProperty -Path $key -Name $flag -Value 2 -PropertyType DWord -Force -ErrorAction Stop | Out-Null
  # -Wait menunggu cleanmgr beserta proses anaknya selesai.
  Start-Process -FilePath $cm -ArgumentList '/sagerun:4207' -Wait -ErrorAction Stop
} catch {
  $r.error = $_.Exception.Message
} finally {
  Remove-ItemProperty -Path $key -Name $flag -ErrorAction SilentlyContinue
}

$r.stillExists = [bool](Test-Path -LiteralPath $Target)
$r.ok = -not $r.stillExists
if ($r.stillExists -and -not $r.error) {
  $r.error = 'Disk Cleanup sudah selesai, tapi sebagian Windows.old masih ada.'
}
$r | ConvertTo-Json -Compress
