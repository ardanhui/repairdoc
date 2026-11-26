# RepairDoc — Dokter PC portable

Alat diagnosa PC Windows yang dijalankan langsung dari flashdisk, tanpa instalasi.
Ditulis untuk pengguna awam: hasilnya berupa kesimpulan dan saran dalam bahasa
sehari-hari. Angka teknis disimpan di bagian **Info untuk teknisi** yang tertutup.

Stack: Tauri 2 (Rust) + Vue 3 + TypeScript. Tema: Frutiger Aero.

---

## Status: Tahap 3 dari 4 — selesai

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

**Belum (tahap berikutnya)**

- Tahap 4: simpan laporan ke flashdisk (HTML/PDF).

## Mode Performa: cara pakai & cara mengembalikan

Buka dari tombol **Mode Performa** di layar awal, atau **Buka Mode Performa** di layar hasil scan.
Scan tetap hanya membaca; yang mengubah pengaturan hanya layar ini, dan hanya setelah kamu menekan
**Terapkan** dua kali (di kartu mode, lalu di pratinjau).

| Mode | Isinya |
|---|---|
| **Kentang** (8 pengaturan) | Efek visual "Adjust for best performance", efek transparan mati, animasi jendela mati, Game Bar/Game DVR mati, aplikasi Store pihak lain tidak jalan di latar, program startup pihak lain tidak ikut menyala, 8 service tidak penting jadi Manual, power plan High performance |
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

**Catatan perubahan** disimpan di `RepairDoc-data\preset-log.json` di samping exe (di flashdisk),
dipisah per PC (dikenali dari MachineGuid Windows), jadi satu flashdisk bisa dipakai di banyak PC
tanpa tertukar. Isinya: waktu, mode, nama restore point, dan tiap perubahan
(`tweakId`, target, `nilaiLama`, `nilaiBaru`, status `berhasil`/`gagal`/`dilewati`).
**Jangan hapus file ini** selama masih ingin memakai Kembalikan Semua. Kalau flashdisk tidak bisa
ditulisi, catatan disimpan di `%TEMP%\RepairDoc\` PC tersebut.

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
│   ├── App.vue                   ← alur layar: awal → scan → hasil, dan Mode Performa
│   ├── components/               ← gelembung scan, kartu temuan, latar Aero, dll.
│   │   └── PerformaView.vue      ← layar Mode Performa (kartu mode, pratinjau, hasil, riwayat)
│   ├── lib/api.ts                ← jembatan ke Rust (+ data contoh untuk mode pratinjau)
│   ├── lib/labels.ts             ← nama kategori/kelompok yang tampil
│   ├── lib/mock-report.json      ← data contoh, dihasilkan dari logika Rust yang asli
│   ├── lib/mock-preset.json      ← data contoh Mode Performa (dari PC tiruan di Rust)
│   └── styles/aero.css           ← warna & gaya Frutiger Aero
├── src-tauri/
│   ├── src/main.rs               ← jendela aplikasi, perintah scan, izin admin, Mode Performa
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
│           │                       exec.rs (menjalankan skrip), log.rs (preset-log.json),
│           │                       fake.rs (PC tiruan untuk tes & data contoh)
│           ├── scripts/*.ps1     ← skrip PowerShell yang dijalankan (bisa dibaca teknisi);
│           │                       preset.ps1, preset_inventory.ps1, restore_point.ps1 untuk
│           │                       Mode Performa, semuanya mendukung -DryRun
│           ├── platform/         ← PowerShell, cek admin, dsb.
│           ├── scan.rs           ← urutan scan & kabar progres
│           └── model.rs          ← struktur temuan, kategori, kelompok
└── .github/workflows/            ← build otomatis di GitHub (opsional)
```

Batas penilaian (misalnya "Drive C: dianggap hampir penuh kalau sisa < 5 GB", "8 program startup
= terlalu banyak") ada sebagai konstanta di awal tiap file `core/src/analyze/*.rs`, lengkap
dengan alasannya. Ubah di situ kalau mau lebih longgar/ketat.

Kalau teks di `core/src/analyze` atau `core/src/preset` diubah, perbarui data contoh untuk mode pratinjau:

```
cd src-tauri
cargo run -p repairdoc-core --example mock_report > ../src/lib/mock-report.json
cargo run -p repairdoc-core --example mock_preset > ../src/lib/mock-preset.json
```

Daftar pengaturan Mode Performa ada di `core/src/preset/tweaks.rs` (`TWEAKS`). Service yang dijadikan
Manual ada di `MANUAL_SERVICES`; service dan kata yang tidak boleh disentuh ada di `PROTECTED_SERVICES`
dan `PROTECTED_WORDS` — daftar yang sama juga tertulis di `preset.ps1`, dan tes memastikan keduanya cocok.
