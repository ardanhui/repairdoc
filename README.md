# RepairDoc — Dokter PC portable

Alat diagnosa PC Windows yang dijalankan langsung dari flashdisk, tanpa instalasi.
Ditulis untuk pengguna awam: hasilnya berupa kesimpulan dan saran dalam bahasa
sehari-hari. Angka teknis disimpan di bagian **Info untuk teknisi** yang tertutup.

Stack: Tauri 2 (Rust) + Vue 3 + TypeScript. Tema: Frutiger Aero.

---

## Status: Tahap 4 dari 5 — selesai

**Sudah jalan**

- Layar utama bertema Frutiger Aero: tombol **Mulai Scan**, gelembung yang terisi air
  sebagai progres, dan status per kelompok (Hardware, Windows, Kebersihan, Keamanan, Jaringan).
- Scan **hanya membaca**, tidak ada pengaturan PC yang diubah. Flashdisk tempat RepairDoc
  dijalankan otomatis dilewati.
- **Hardware:** prosesor (pemakaian, suhu bila ada sensornya), RAM (ukuran, pemakaian, slot,
  hasil tes memori Windows), penyimpanan (ruang kosong, HDD/SSD, kesehatan disk), kartu grafis
  (driver, kode error, suhu untuk NVIDIA), baterai laptop (kapasitas sekarang vs saat baru).
- **Windows:** versi & status dukungan, update terakhir, Windows Update dimatikan atau tidak,
  menunggu restart, lama tidak restart, perangkat bertanda seru di Device Manager, power plan,
  waktu boot terakhir, program yang ikut jalan saat menyala, program yang sedang membebani.
- **Kebersihan:** file sementara, cache browser, Recycle Bin, sisa unduhan update, Windows.old.
- **Keamanan:** Windows Defender / antivirus lain (aktif? definisi terbaru? kapan terakhir scan?),
  dua antivirus bentrok, firewall, UAC, program yang sering tidak perlu/berisiko (aktivator bajakan,
  toolbar, "pembersih" palsu, dsb.).
- **Jaringan:** tersambung ke internet atau tidak (cara yang sama dengan Windows), sinyal Wi-Fi,
  DNS, tes unduh sederhana (maksimal 10 MB, berhenti setelah 8 detik).
- Hasil: satu kesimpulan besar, lalu tiga keranjang: **Yang perlu dibereskan**, **Hal kecil
  yang bisa dirapikan nanti**, dan **Yang sudah baik**. Tiap temuan punya penjelasan dan
  langkah. Angka teknis ada di **Info untuk teknisi** (tertutup).
- Deteksi izin administrator, daftar apa saja yang belum bisa diperiksa tanpa izin itu, dan tombol
  untuk memintanya (memunculkan jendela UAC).
- **Mode Performa** (tahap 3, v0.2): tiga mode seperti pengaturan grafis di game — **Kentang**,
  **Seimbang**, **Normal**. Selalu ada pratinjau dulu, restore point wajib sebelum mengubah apa pun,
  catatan perubahan per PC, dan tombol **Kembalikan Semua**. Penjelasan lengkap di bawah.
- **Bersihkan File Sampah** (tahap 4, v0.3): hapus file sementara, cache browser, sisa unduhan
  Windows Update, dan (kalau dicentang sendiri) Recycle Bin — dari lokasi yang sama dengan yang
  diukur scan. Windows.old hanya lewat konfirmasi terpisah. Ada pratinjau, konfirmasi, laporan
  ruang yang benar-benar dibebaskan, dan catatan.
- **Startup lebih ganas** (tahap 4, v0.3): Mode Kentang sekarang juga memeriksa folder Startup dan
  tugas terjadwal, dan aplikasi tambahan vendor hardware (Realtek, NVIDIA, Synaptics, tombol Fn, …)
  boleh ikut dimatikan. Yang tetap tidak pernah disentuh: Microsoft, Windows, keamanan, antivirus.

- **Tampilan dirapikan mengikuti audit UI** (setelah v0.3): kontras tombol utama ≥ 4,5:1,
  semua target klik ≥ 32 px, teks isi ≥ 13 px, tidak ada teks terpotong "…", hanya kartu
  temuan pertama yang langsung terbuka, label & istilah dalam bahasa Indonesia, dan status
  "tidak dipilih" tidak lagi ditandai dengan teks yang dipudarkan. Aturan mainnya ditulis di
  awal `src/styles/aero.css`.
- **Latar "bersih"** putih & biru muda dengan gelembung kaca dan ombak air (dipilih dari tiga
  alternatif), tetap statis tanpa animasi supaya ringan.
- **Musik latar orisinal** (lagu santai 35 detik yang berulang tanpa jeda). Mulai setelah klik
  pertama, volume pelan, bisa dimatikan lewat tombol **Musik** di kanan atas (pilihan diingat),
  dan berhenti sendiri saat jendela diminimize. Lagu ini dibuat khusus untuk RepairDoc dari nol
  — bukan lagu pihak lain; asal-usul dan cara membuat ulangnya ada di
  `src/assets/LISENSI-MUSIK.md`.

**Belum (tahap berikutnya)**

- Tahap 5: export laporan (simpan hasil scan ke flashdisk sebagai HTML/PDF).

## Mode Performa: cara pakai & cara mengembalikan

Buka dari tombol **Mode Performa** di layar awal, atau **Buka Mode Performa** di layar hasil scan.
Scan tetap hanya membaca; yang mengubah pengaturan hanya layar ini, dan hanya setelah kamu menekan
**Terapkan** dua kali (di kartu mode, lalu di pratinjau).

| Mode | Isinya |
|---|---|
| **Kentang** (8 pengaturan) | Efek visual "Adjust for best performance", efek transparan mati, animasi jendela mati, Game Bar/Game DVR mati, aplikasi Store pihak lain tidak jalan di latar, program startup pihak lain tidak ikut menyala (dari 4 sumber, lihat di bawah), 8 service tidak penting jadi Manual, power plan High performance |
| **Seimbang** (4 pengaturan) | Efek transparan mati, animasi jendela mati, Game Bar/Game DVR mati, power plan Balanced |
| **Normal** | Membatalkan semua perubahan RepairDoc di PC ini ke nilai aslinya (dari catatan). Kalau belum ada catatan: efek visual, transparansi, animasi, Game Bar, aplikasi latar, dan power plan dikembalikan ke bawaan Windows |

Langkahnya:

1. **Izin administrator.** Tanpa izin itu kartu mode bisa dilihat tapi tidak bisa diterapkan.
   Tekan **Minta izin administrator** → setujui UAC → RepairDoc terbuka ulang → buka Mode Performa lagi.
2. **Atur satu per satu** (opsional). Matikan saklar pengaturan yang tidak kamu mau. Pengaturan yang
   dimatikan dilewati dan tercatat "dilewati".
3. **Terapkan** di kartu mode → muncul **pratinjau**: daftar apa yang akan berubah, nilai sekarang →
   nilai baru. Belum ada yang diubah. Untuk program startup dan aplikasi latar, tiap program bisa
   dihapus centangnya. **Batal** = tidak ada yang berubah.
4. **Terapkan** di pratinjau → RepairDoc **membuat restore point dulu** (1–3 menit). Kalau restore
   point gagal dibuat, RepairDoc **berhenti tanpa mengubah apa pun** dan menampilkan cara menyalakan
   System Restore. Tidak ada tombol "lanjut tanpa restore point".
5. Pengaturan diterapkan satu per satu. Kalau satu gagal, yang lain tetap jalan; yang gagal disebut di
   layar hasil. Sebagian perubahan baru terasa setelah sign out atau restart.

Menerapkan mode yang sama dua kali aman: yang kedua tidak mengubah apa pun ("sudah sesuai").
Mode yang sedang berlaku tampil di atas kartu dan bertanda **Aktif**.

**Cara mengembalikan** (dari yang paling mudah):

1. **Kembalikan Semua** di kotak *Riwayat perubahan*: membatalkan penerapan terakhir, setiap
   pengaturan kembali ke nilai yang tercatat sebelum diubah. Tekan lagi untuk penerapan sebelumnya.
2. **Mode Normal**: membatalkan *semua* penerapan RepairDoc di PC ini sekaligus (dari yang terbaru).
3. **System Restore** (jaring pengaman terakhir): ketik `rstrui` di menu Start → pilih restore point
   bernama "RepairDoc Mode …" → Next → Finish. PC akan restart.

### Program startup (tahap 4: lebih ganas)

Pratinjau Mode Kentang menampilkan semua program startup yang akan dimatikan, dikelompokkan per
sumber, lengkap dengan perintah/jalurnya. Tiap program bisa dihapus centangnya.

| Sumber | Cara mematikan | Cara mengembalikan |
|---|---|---|
| **Registry** (kunci Run pengguna, Run semua pengguna, Run 32-bit) | Ditandai "Disabled" di `StartupApproved`, sama seperti tab Startup di Task Manager. Entri Run tidak dihapus. | Data `StartupApproved` lama ditulis ulang |
| **Folder Startup** (`%APPDATA%\…\Startup` dan `C:\ProgramData\…\StartUp`) | Shortcut (`.lnk`) **dipindah** ke `RepairDoc-data\startup-cadangan\<nama-pc>\<tanggal>\…` — tidak dihapus | Shortcut dipindah balik ke folder asalnya |
| **Tugas Terjadwal** (pemicu "saat masuk Windows", program di Program Files / AppData) | Dinonaktifkan (`Disable-ScheduledTask`) — tidak dihapus | Diaktifkan lagi (`Enable-ScheduledTask`) |

Yang **tidak pernah** jadi sumber: `RunOnce` (dipakai installer), `Winlogon`/`Userinit`/`Shell`,
dan service (sudah diurus pengaturan service tersendiri).

Yang **tidak pernah** dimatikan: apa pun yang nama atau jalurnya mengandung *Microsoft*, *Windows*,
*Security*, *Defender*, atau nama antivirus (Avast, AVG, Kaspersky, ESET, McAfee, Norton,
Bitdefender, Avira, Sophos, Malwarebytes, Trend Micro, Panda, Comodo, 360 Total Security, Smadav,
…). Program yang file-nya ada di folder `C:\Windows` juga ikut terlindung, karena jalurnya
mengandung "windows".

**Aplikasi tambahan vendor hardware** (suara Realtek/Waves/Dolby, touchpad Synaptics/Elan,
grafis NVIDIA/AMD/Intel, Bluetooth, tombol Fn) sekarang boleh dimatikan. Yang dimatikan hanya
aplikasi pembuka / ikon tray-nya, **bukan drivernya** — driver audio, touchpad, dan kartu grafis
tetap jalan, jadi suara, touchpad dasar, dan tampilan tetap berfungsi. Efek tiap item ditulis di
pratinjau (mis. "gestur lanjutan touchpad bisa mati", "tombol Fn bisa tidak berfungsi"); kalau
efeknya tidak diketahui, ditulis begitu. Pembaru otomatis (Google Update, dsb.) diberi catatan
bahwa programnya bisa telat mendapat perbaikan keamanan.

**Kembalikan Semua** dan **Mode Normal** mengembalikan ketiga sumber sekaligus. Folder
`startup-cadangan` ada di flashdisk: **jangan dihapus** selama shortcut di dalamnya masih ingin
dikembalikan, dan colok flashdisk yang sama saat menekan Kembalikan Semua.

**Catatan perubahan** disimpan di `RepairDoc-data\preset-log.json` di samping exe (di flashdisk),
dipisah per PC (dikenali dari MachineGuid Windows), jadi satu flashdisk bisa dipakai di banyak PC
tanpa tertukar. Isinya: waktu, mode, nama restore point, dan tiap perubahan
(`tweakId`, target, `nilaiLama`, `nilaiBaru`, status `berhasil`/`gagal`/`dilewati`).
**Jangan hapus file ini** selama masih ingin memakai Kembalikan Semua. Kalau flashdisk tidak bisa
ditulisi, catatan disimpan di `%TEMP%\RepairDoc\` PC tersebut.

## Bersihkan File Sampah: cara pakai & yang tidak bisa kembali

Buka dari tombol **Bersihkan File Sampah** di layar awal, tombol yang sama di layar hasil scan,
atau tombol **Bersihkan** di kartu temuan file sampah.

| Jenis | Lokasi (sama dengan yang diukur scan) | Tercentang otomatis? |
|---|---|---|
| File sementara | `%TEMP%` pengguna dan `C:\Windows\Temp` | Ya |
| Cache browser & internet | Cache Chrome, Edge, Brave, Opera, Vivaldi, Firefox (`cache2`), dan `INetCache` Windows | Ya |
| Sisa unduhan Windows Update | `C:\Windows\SoftwareDistribution\Download` | Ya |
| Recycle Bin | `$Recycle.Bin` di semua drive tetap (bukan flashdisk) | **Tidak** — harus dicentang sendiri |
| Sisa Windows lama | `C:\Windows.old` | **Tidak** — kotak terpisah dengan peringatan |

Langkahnya:

1. **Izin administrator** (C:\Windows\Temp dan sisa Windows Update butuh izin itu).
2. **Pratinjau:** RepairDoc mengukur ulang tiap jenis (ukuran, jumlah file, lokasi folder). Belum
   ada yang dihapus. Centang / hapus centang per jenis; ringkasan di kanan ikut berubah.
3. **Bersihkan** → muncul konfirmasi sekali lagi ("akan menghapus ±X GB dari N jenis…").
4. RepairDoc menghapus **isi** folder-folder itu satu jenis demi satu jenis. Jenis yang gagal
   dicatat, jenis lain tetap jalan.
5. **Hasil:** ruang yang benar-benar dibebaskan (ukuran nyata file yang terhapus, bukan perkiraan),
   jumlah file terhapus, dan jumlah yang dilewati beserta alasannya.

Yang **selalu dilewati** (bukan galat): file yang sedang dipakai program; file sementara yang
dibuat/diubah 24 jam terakhir (mungkin masih dipakai installer atau update yang menunggu restart);
cache browser yang sedang terbuka (tutup dulu browsernya); symlink/junction (tidak pernah diikuti).
Yang **tidak pernah dihapus:** `hiberfil.sys` dan `pagefile.sys` (hanya ditampilkan ukurannya —
itu file sistem, bukan sampah), dan apa pun di luar lokasi di tabel.

**Windows.old** dihapus memakai Disk Cleanup bawaan Windows (penangan "Previous Windows
installation(s)", sama dengan Settings → Storage), hanya kalau kotaknya sendiri dicentang.
Windows.old dibutuhkan untuk kembali ke Windows lama dalam 10 hari setelah upgrade.

**Yang tidak bisa kembali:** file yang sudah dihapus **tidak bisa dikembalikan** — tidak ada
Kembalikan Semua untuk fitur ini, dan RepairDoc sengaja tidak membuat restore point (System Restore
tidak menyimpan file sementara, cache, maupun isi Recycle Bin). Pengamannya adalah pratinjau,
centang per jenis, konfirmasi, dan lokasi yang dibatasi. Isi Recycle Bin dan Windows.old hilang
permanen.

**Catatan pembersihan** disimpan di `RepairDoc-data\cleanup-log.json` (waktu, PC, dan per jenis:
`jenis`, `lokasi`, `fileTerhapus`, `byteDibebaskan`, `dilewati`, `status`). Ini catatan riwayat,
bukan tombol undo.

## 1. Persiapan (sekali saja), di Windows 10/11 64-bit

1. **Node.js LTS** (versi 22 atau lebih baru): unduh dari https://nodejs.org
2. **Rust**: unduh `rustup-init.exe` dari https://rustup.rs lalu jalankan.
   Kalau penginstal menawarkan memasang **Visual Studio / C++ Build Tools**, setujui.
   Yang dibutuhkan adalah workload **"Desktop development with C++"**.
3. **WebView2**: sudah ada di Windows 11 dan di hampir semua Windows 10 yang pernah
   di-update. Tidak perlu dipasang terpisah.

Cek di Command Prompt / PowerShell:

```
node -v
cargo -V
```

Kalau keduanya menampilkan nomor versi, persiapan selesai.

## 2. Menjalankan saat pengembangan

Di folder proyek:

```
npm install
npm run tauri dev
```

Jendela RepairDoc akan terbuka. Perubahan di file `.vue` langsung terlihat.

Untuk mengerjakan tampilan saja tanpa Rust: `npm run dev`, lalu buka
http://localhost:1420 di browser. Di mode ini muncul label **"Mode pratinjau: data contoh"**
dan hasil scan memakai data contoh (PC kentang khas), bukan PC kamu.

## 3. Build exe portable

```
npm run tauri build
```

Hasilnya satu file: `src-tauri\target\release\repairdoc.exe`
(build pertama bisa 5–15 menit; berikutnya jauh lebih cepat).

**Alternatif tanpa memasang Build Tools:** upload proyek ke GitHub. Workflow
`.github/workflows/build-windows.yml` akan mem-build exe di server GitHub. Unduh hasilnya
di tab **Actions** → run terakhir → **Artifacts** → `RepairDoc-windows`.

## 4. Menyalin ke flashdisk

1. Buat folder di flashdisk, misalnya `E:\RepairDoc\`.
2. Salin `repairdoc.exe` ke folder itu. Boleh diganti nama jadi `RepairDoc.exe`.
3. Selesai. Colok ke PC lain, buka `RepairDoc.exe`.

Saat dijalankan, RepairDoc membuat folder `RepairDoc-data` di samping exe (data tampilan).
Ini supaya tidak meninggalkan jejak di PC yang diperiksa. Folder itu aman dihapus.
Kalau flashdisk tidak bisa ditulisi, data disimpan sementara di folder TEMP PC tersebut.

**Opsional, untuk Windows 10 lama yang belum punya WebView2:**
unduh **WebView2 Fixed Version Runtime (x64)** dari
https://developer.microsoft.com/microsoft-edge/webview2/ , ekstrak, lalu ganti nama
foldernya menjadi `WebView2` dan letakkan di samping exe (sekitar 200 MB):

```
E:\RepairDoc\
├── RepairDoc.exe
└── WebView2\
    ├── msedgewebview2.exe
    └── ...
```

RepairDoc otomatis memakai folder itu bila ada.

---

## 5. Cara mengetes

1. Jalankan `RepairDoc.exe` dari flashdisk di PC Windows, klik **Mulai Scan**.
   Lima kelompok harus selesai satu per satu, lalu hasil muncul (±30–60 detik,
   tergantung kecepatan PC dan internet).
2. Cocokkan angkanya:
   - RAM terpakai → Task Manager (Ctrl + Shift + Esc) → Performance → Memory.
   - Sisa ruang Drive C: → File Explorer → This PC.
   - HDD atau SSD → Task Manager → Performance → Disk 0.
   - Jumlah program startup → Task Manager → tab Startup (hitung yang "Enabled").
   - Versi Windows → Settings → System → About.
   - Antivirus & firewall → Windows Security.
   - Kecepatan unduh → bandingkan kasar dengan fast.com (hasil RepairDoc biasanya sedikit lebih rendah
     karena hanya 8 detik dan satu sambungan).
3. Pastikan flashdisk **tidak** muncul sebagai temuan.
4. Klik **Minta izin administrator** → setujui UAC → RepairDoc terbuka ulang. Scan lagi:
   catatan "Sebagian pemeriksaan butuh izin administrator" harus hilang.
5. Buka **Info untuk teknisi** dan cek isinya masuk akal.
6. Coba juga di PC desktop: kelompok Hardware harus tetap selesai, dan di Info untuk teknisi
   bagian Baterai tertulis "Dilewati: PC desktop".

Tes tanpa tampilan (berguna kalau ada yang aneh):

```
cd src-tauri
cargo test -p repairdoc-core                         # tes logika + semua skrip PowerShell sungguhan
cargo run -p repairdoc-core --example scan_cli       # scan penuh, hasil dicetak sebagai JSON
```

**Kalau ada yang salah,** kirimkan ke saya isi "Info untuk teknisi" (atau keluaran
`scan_cli`) beserta apa yang terlihat di layar.

### Mengetes Mode Performa (tahap 3)

Sebaiknya di PC/VM uji dulu, bukan PC kerja. Pastikan System Restore menyala di drive C:.

1. Jalankan sebagai administrator → **Mode Performa** → **Mode Kentang** → **Terapkan**. Periksa
   pratinjaunya masuk akal (nilai "sekarang" cocok dengan Windows), lalu **Batal**: tidak ada yang berubah.
2. **Terapkan** lagi → di pratinjau tekan **Terapkan**. Setelah selesai, cek di Windows:
   - `rstrui` → ada restore point "RepairDoc Mode Kentang …".
   - `SystemPropertiesPerformance` → terpilih "Adjust for best performance" (setelah sign out).
   - Settings → Personalization → Colors → Transparency effects: Off.
   - Settings → Gaming → Captures / Game Bar: perekaman mati.
   - Task Manager → Startup: program pihak lain "Disabled"; antivirus, Windows Security, driver audio/touchpad tetap "Enabled".
   - `services.msc` → Xbox*, Fax, Remote Registry, Downloaded Maps Manager, WMP Network Sharing: Manual
     (yang sebelumnya sudah Manual/Disabled dibiarkan).
   - `powercfg /getactivescheme` → High performance.
   - `RepairDoc-data\preset-log.json` ada di flashdisk.
3. **Terapkan** Mode Kentang sekali lagi → pratinjau harus bilang tidak ada yang perlu diubah.
4. **Kembalikan Semua** → semua yang di langkah 2 kembali seperti semula (restore point tetap ada).
5. Coba **Mode Seimbang**, lalu **Mode Normal** → semuanya kembali ke nilai sebelum RepairDoc.
6. Tes pengaman: matikan System Protection untuk drive C: → coba terapkan mode apa pun → harus berhenti
   dengan layar "Restore point tidak bisa dibuat" dan **tidak ada** yang berubah. Nyalakan lagi sesudahnya.

`cargo test -p repairdoc-core` di Windows juga menjalankan skrip Mode Performa sungguhan, tapi
**hanya** pada kunci uji `HKCU\Software\RepairDocTest\…` (dibuat dan dihapus oleh tesnya sendiri).
Service dan power plan di tes itu hanya dibaca, restore point hanya mode `-DryRun`.

### Mengetes tahap 4 (Bersihkan + startup)

**Bersihkan File Sampah** — di PC/VM uji:

1. Catat sisa ruang drive C: (File Explorer → This PC). Buka **Bersihkan File Sampah** (sebagai
   admin). Cocokkan ukuran tiap jenis kira-kira dengan Disk Cleanup (`cleanmgr`).
2. Pastikan Recycle Bin dan Windows.old **tidak** tercentang. Buka Chrome/Edge, lalu **Bersihkan**
   → konfirmasi → **Ya, bersihkan**.
3. Hasil: "X GB berhasil dibebaskan" harus kira-kira sama dengan bertambahnya sisa ruang drive C:.
   Cache browser yang terbuka harus tertulis "dilewati". Isi Recycle Bin harus masih ada.
4. Taruh file uji di Recycle Bin → centang Recycle Bin → bersihkan → Recycle Bin kosong.
5. `RepairDoc-data\cleanup-log.json` ada dan isinya sesuai.
6. Windows.old: hanya uji di VM yang baru di-upgrade — centang kotaknya sendiri → Disk Cleanup
   berjalan → folder `C:\Windows.old` hilang.

**Startup lebih ganas** — sebelum mulai, foto tab Startup Task Manager, isi kedua folder Startup
(`shell:startup` dan `shell:common startup` di kotak Run), dan Task Scheduler Library:

1. **Mode Kentang** → pratinjau: bagian "Program startup dikurangi" dikelompokkan **Registry /
   Folder Startup / Tugas Terjadwal**, tiap item ada perintah/jalurnya; item vendor (Realtek,
   NVIDIA, Synaptics, …) ada catatan efeknya; antivirus & Windows Security tidak muncul.
2. Terapkan → shortcut pindah ke `RepairDoc-data\startup-cadangan\…`; tugas pihak ketiga di Task
   Scheduler berstatus "Disabled" (tidak hilang).
3. **Kembalikan Semua** → shortcut kembali ke folder asal, tugas kembali "Ready", tab Startup sama
   dengan foto awal.

`cargo test -p repairdoc-core` di Windows juga menguji: penghapus di folder **uji** sementara
(file terkunci dilewati, junction tidak diikuti, simulasi tidak mengubah apa pun), skrip Windows.old
**hanya** mode `-DryRun`, dan menonaktifkan/mengaktifkan tugas terjadwal **uji** di folder
`\RepairDocTest\` (dibuat dan dihapus oleh tesnya sendiri; kalau tidak ada izin membuatnya,
tes itu dilewati).

## 6. Keterbatasan yang perlu diketahui

- **v0.1 dites di Windows sungguhan 26 Nov 2026 oleh pemilik — scan jalan.**
- **Mode Performa (v0.2) belum pernah dijalankan di Windows sungguhan.** Kode dibuat di Linux:
  logikanya (rencana, urutan, pengembalian, catatan, pengaman) lolos tes otomatis dengan PC tiruan,
  dan semua kode Windows lolos pemeriksaan compiler untuk target Windows. Yang belum pernah jalan:
  skrip `preset.ps1`, `preset_inventory.ps1`, `restore_point.ps1`, perubahan lewat `sc.exe` dan
  `powercfg`, serta efek nyata pengaturan aplikasi latar per aplikasi. Ikuti "Mengetes Mode Performa"
  di atas sebelum dipakai di PC orang lain.
- **Mode Performa memakai akun yang memberi izin administrator.** Kalau pengguna biasa memasukkan
  kata sandi akun admin *lain* di jendela UAC, pengaturan per pengguna (efek visual, transparansi,
  Game Bar, startup HKCU, aplikasi latar) berlaku untuk akun admin itu, bukan akun pengguna. Paling
  aman: jalankan dari akun yang memang akan dipakai, dan akun itu admin.
- **Restore point:** Windows normalnya hanya mengizinkan satu restore point per 24 jam. RepairDoc
  mematikan batas itu sementara (`SystemRestorePointCreationFrequency` = 0) khusus untuk restore
  point-nya, lalu mengembalikan nilai aslinya. **Kembalikan Semua** tidak membuat restore point baru.
- **Kembalikan Semua** menulis ulang nilai yang tercatat. Kalau setelah menerapkan mode kamu mengubah
  pengaturan yang sama secara manual, perubahan manual itu ikut tertimpa nilai lama.
- **Tahap 4 (v0.3) belum pernah dijalankan di Windows sungguhan.** Logika pembersih dan pemindah
  shortcut dites sungguhan di folder uji (di Linux, termasuk symlink); alur startup 4 sumber dites
  dengan PC tiruan; semua kode Windows lolos pemeriksaan compiler untuk target Windows. Yang belum
  pernah jalan di Windows: penghapusan di lokasi sungguhan (termasuk file terkunci & atribut
  read-only), Disk Cleanup untuk Windows.old, bagian baru `preset_inventory.ps1` (folder Startup &
  tugas logon), `Disable-/Enable-ScheduledTask` lewat `preset.ps1`, dan pemindahan shortcut dari
  drive C: ke flashdisk (beda drive). Ikuti "Mengetes tahap 4" di atas.
- **Cadangan shortcut startup ada di flashdisk.** Kalau flashdisk hilang atau diformat, shortcut
  yang dipindah tidak bisa dikembalikan otomatis (programnya tetap terpasang; shortcut bisa dibuat
  ulang). Kalau flashdisk tidak bisa ditulisi, cadangan disimpan di `%TEMP%\RepairDoc\` PC itu.
- **Bersihkan File Sampah tidak bisa dibatalkan.** File yang terhapus hilang permanen. Pratinjau
  mengukur ulang setiap kali dibuka (hasil scan tidak menyimpan angka per jenis), jadi angkanya bisa
  sedikit berbeda dari layar hasil scan.
- **Recycle Bin** yang dibersihkan adalah semua isi `$Recycle.Bin` yang terukur, termasuk milik
  akun lain di PC itu (dengan izin administrator).
- **Suhu prosesor:** Windows hanya bisa membaca sensor "thermal zone" motherboard, dan banyak PC
  tidak menyediakannya. Kalau tidak ada, RepairDoc tidak menampilkan apa-apa soal suhu. Suhu kartu
  grafis hanya terbaca untuk NVIDIA.
- **Data disk lanjutan** (umur SSD, suhu disk, error), **log waktu boot**, dan **sampah milik
  pengguna lain** hanya terbaca dengan izin administrator. RepairDoc menyebutkan ini di hasil.
- **Dampak tiap program startup** tidak diukur (angka "impact" di Task Manager tidak bisa dibaca
  dengan andal). RepairDoc menghitung jumlah dan menyebut namanya.
- **Program mencurigakan** dikenali dari daftar pola nama yang sengaja pendek dan konservatif
  (`core/src/analyze/security.rs`, konstanta `SUSPICIOUS`). Program yang tidak ada di daftar itu
  tidak akan ditandai; yang ada di daftar pun kalimatnya "sebaiknya dihapus", bukan "virus".
- **Tes internet** mengunduh paling banyak 10 MB dari alamat tes publik (daftar di
  `core/src/probe/network.rs`, konstanta `DOWNLOAD_URLS`). Kalau semua alamat itu gagal, RepairDoc
  cuma melewatkan tes kecepatannya, bukan menganggap internet mati. Cek "ada internet / tidak"
  memakai alamat yang sama dengan Windows sendiri (msftconnecttest.com).
- **Windows 10:** RepairDoc menyebut dukungan Windows 10 berakhir 14 Oktober 2025. Kalau PC
  memakai program Extended Security Updates, kalimat itu tetap muncul (tidak dicek).
- **Pemakaian prosesor dan proses berat** diukur 2 detik saat scan — potret sesaat.
- **Windows SmartScreen / antivirus** bisa memberi peringatan karena exe belum ditandatangani.
  Klik "More info" → "Run anyway". Tanda tangan digital bisa diurus saat rilis.

## 7. Struktur proyek

```
repairdoc/
├── src/                          ← tampilan (Vue 3)
│   ├── App.vue                   ← alur layar: awal → scan → hasil, Mode Performa, Bersihkan
│   ├── components/               ← gelembung scan, kartu temuan, latar Aero, dll.
│   │   ├── PerformaView.vue      ← layar Mode Performa (kartu mode, pratinjau, hasil, riwayat)
│   │   └── CleanupView.vue       ← layar Bersihkan File Sampah (tahap 4)
│   ├── lib/api.ts                ← jembatan ke Rust (+ data contoh untuk mode pratinjau)
│   ├── lib/labels.ts             ← nama kategori/kelompok yang tampil
│   ├── lib/mock-report.json      ← data contoh, dihasilkan dari logika Rust yang asli
│   ├── lib/mock-preset.json      ← data contoh Mode Performa + Bersihkan (dari logika Rust)
│   ├── lib/music.ts              ← pemutar musik latar (Web Audio, loop tanpa jeda)
│   ├── assets/musik-latar.ogg    ← musik latar orisinal (lihat assets/LISENSI-MUSIK.md)
│   └── styles/aero.css           ← warna & gaya Frutiger Aero
├── src-tauri/
│   ├── src/main.rs               ← jendela aplikasi, perintah scan, izin admin, Mode Performa,
│   │                               Bersihkan File Sampah
│   ├── tauri.conf.json
│   └── core/                     ← logika diagnosa (bisa dites tanpa tampilan)
│       └── src/
│           ├── probe/            ← membaca data PC (hanya baca): live (CPU+proses), memory,
│           │                       disk, gpu, battery, system, programs, cleanup, security,
│           │                       network, net (klien HTTP mini untuk tes internet)
│           ├── analyze/          ← menilai data → temuan bahasa awam + batas-batasnya
│           ├── preset/           ← Mode Performa: tweaks.rs (daftar pengaturan & daftar yang
│           │                       TIDAK BOLEH disentuh), plan.rs (rencana, murni), engine.rs
│           │                       (alur admin → pratinjau → restore point → terapkan → catat),
│           │                       exec.rs (menjalankan skrip + memindah shortcut startup),
│           │                       log.rs (preset-log.json), fake.rs (PC tiruan untuk tes)
│           ├── cleaner/          ← Bersihkan File Sampah (tahap 4): plan.rs (aturan murni),
│           │                       exec.rs (penghapus, tidak mengikuti symlink), engine.rs
│           │                       (alur), log.rs (cleanup-log.json). probe/cleanup.rs tetap
│           │                       hanya mengukur; daftar lokasinya dipakai bersama.
│           ├── scripts/*.ps1     ← skrip PowerShell yang dijalankan (bisa dibaca teknisi);
│           │                       preset.ps1, preset_inventory.ps1, restore_point.ps1 untuk
│           │                       Mode Performa, cleanup_windows_old.ps1 untuk Windows.old;
│           │                       yang mengubah sistem mendukung -DryRun
│           ├── platform/         ← PowerShell, cek admin, dsb.
│           ├── scan.rs           ← urutan scan & kabar progres
│           └── model.rs          ← struktur temuan, kategori, kelompok
├── tools/musik-latar/            ← skrip Python pembuat musik latar (tidak ikut ke aplikasi)
└── .github/workflows/            ← build otomatis di GitHub (opsional)
```

Batas penilaian (misalnya "Drive C: dianggap hampir penuh kalau sisa < 5 GB", "8 program startup
= terlalu banyak") ada sebagai konstanta di awal tiap file `core/src/analyze/*.rs`, lengkap
dengan alasannya. Ubah di situ kalau mau lebih longgar/ketat.

Kalau teks di `core/src/analyze`, `core/src/preset`, atau `core/src/cleaner` diubah, perbarui data contoh untuk mode pratinjau:

```
cd src-tauri
cargo run -p repairdoc-core --example mock_report > ../src/lib/mock-report.json
cargo run -p repairdoc-core --example mock_preset > ../src/lib/mock-preset.json
```

Daftar pengaturan Mode Performa ada di `core/src/preset/tweaks.rs` (`TWEAKS`). Service yang dijadikan
Manual ada di `MANUAL_SERVICES`; service yang tidak boleh disentuh ada di `PROTECTED_SERVICES` (daftar
yang sama tertulis di `preset.ps1`, dicek tes). Kata yang membuat program startup tidak pernah
dimatikan ada di `PROTECTED_WORDS`; aplikasi tambahan vendor hardware ada di `HARDWARE_HELPER_WORDS`
(boleh dimatikan sebagai startup sejak tahap 4, tetap terlindung untuk aplikasi latar).
Aturan lokasi yang boleh dibersihkan ada di `core/src/cleaner/plan.rs` (`root_is_sane`).
