# RepairDoc — program yang ikut jalan saat Windows menyala (startup).
# Hanya membaca, tidak mengubah apa pun.
# Status aktif/nonaktif diambil dari tempat yang sama dengan Task Manager → tab Startup.
$r = [ordered]@{ startup = @() }

$approved = @{}
foreach ($root in 'HKCU', 'HKLM') {
  foreach ($sub in 'Run', 'Run32', 'StartupFolder') {
    try {
      $k = Get-Item -Path "${root}:\Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\$sub" -ErrorAction Stop
      foreach ($name in $k.GetValueNames()) {
        $v = $k.GetValue($name)
        if ($v -is [byte[]] -and $v.Length -gt 0) { $approved[$name.ToLowerInvariant()] = ($v[0] -ne 3) }
      }
    } catch {}
  }
}

$list = @()
try {
  foreach ($c in @(Get-CimInstance -ClassName Win32_StartupCommand -ErrorAction Stop)) {
    $name = [string]$c.Name
    $key = $name.ToLowerInvariant()
    $enabled = $true
    if ($approved.ContainsKey($key)) { $enabled = $approved[$key] }
    else {
      # Entri folder Startup disimpan dengan nama file lengkap (mis. "Nama.lnk").
      $hit = @($approved.Keys | Where-Object { $_.StartsWith($key + '.') }) | Select-Object -First 1
      if ($hit) { $enabled = $approved[$hit] }
    }
    $list += [pscustomobject]@{ name = $name; command = [string]$c.Command; location = [string]$c.Location; enabled = [bool]$enabled }
  }
} catch {}

# Aplikasi Microsoft Store yang punya "startup task" (mis. Spotify, Teams). State 2 = aktif.
try {
  $base = 'HKCU:\Software\Classes\Local Settings\Software\Microsoft\Windows\CurrentVersion\AppModel\SystemAppData'
  if (Test-Path $base) {
    foreach ($pkg in @(Get-ChildItem -Path $base -ErrorAction SilentlyContinue)) {
      foreach ($task in @(Get-ChildItem -Path $pkg.PSPath -ErrorAction SilentlyContinue)) {
        $st = (Get-ItemProperty -Path $task.PSPath -ErrorAction SilentlyContinue).State
        if ($null -ne $st) {
          $pname = ([string]$pkg.PSChildName -split '_')[0]
          $list += [pscustomobject]@{ name = $pname; command = ''; location = 'StoreApp'; enabled = ([int]$st -eq 2) }
        }
      }
    }
  }
} catch {}

$r.startup = $list
$r | ConvertTo-Json -Compress -Depth 4
