# RepairDoc — Dokter PC portable

Alat diagnosa PC Windows yang dijalankan langsung dari flashdisk, tanpa instalasi.
Ditulis untuk pengguna awam: hasilnya berupa kesimpulan dan saran dalam bahasa
sehari-hari. Angka teknis disimpan di bagian **Info untuk teknisi** yang tertutup.

Stack: Tauri 2 (Rust) + Vue 3 + TypeScript. Tema: Frutiger Aero.

---

## Status: Tahap 2 dari 4

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

**Belum (tahap berikutnya)**

- Tahap 3: preset Mode Kentang / Seimbang / Normal, restore point, tombol Kembalikan Semua.
- Tahap 4: simpan laporan ke flashdisk (HTML/PDF).

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

## 6. Keterbatasan yang perlu diketahui

- **Belum dites di Windows sungguhan.** Kode dibuat di Linux. Semua bagian Windows lolos
  pemeriksaan compiler untuk target Windows, dan logika diagnosanya lolos 96 tes otomatis.
  Tapi skrip PowerShell-nya belum pernah dijalankan, jadi tes di PC kamu adalah tes yang
  sesungguhnya. Yang paling mungkin perlu dibetulkan setelah tes: skrip di
  `src-tauri/core/src/scripts/`.
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
│   ├── App.vue                   ← alur layar: awal → scan → hasil
│   ├── components/               ← gelembung scan, kartu temuan, latar Aero, dll.
│   ├── lib/api.ts                ← jembatan ke Rust (+ data contoh untuk mode pratinjau)
│   ├── lib/labels.ts             ← nama kategori/kelompok yang tampil
│   ├── lib/mock-report.json      ← data contoh, dihasilkan dari logika Rust yang asli
│   └── styles/aero.css           ← warna & gaya Frutiger Aero
├── src-tauri/
│   ├── src/main.rs               ← jendela aplikasi, perintah scan & izin admin
│   ├── tauri.conf.json
│   └── core/                     ← logika diagnosa (bisa dites tanpa tampilan)
│       └── src/
│           ├── probe/            ← membaca data PC (hanya baca): live (CPU+proses), memory,
│           │                       disk, gpu, battery, system, programs, cleanup, security,
│           │                       network, net (klien HTTP mini untuk tes internet)
│           ├── analyze/          ← menilai data → temuan bahasa awam + batas-batasnya
│           ├── scripts/*.ps1     ← skrip PowerShell yang dijalankan (bisa dibaca teknisi)
│           ├── platform/         ← PowerShell, cek admin, dsb.
│           ├── scan.rs           ← urutan scan & kabar progres
│           └── model.rs          ← struktur temuan, kategori, kelompok
└── .github/workflows/            ← build otomatis di GitHub (opsional)
```

Batas penilaian (misalnya "Drive C: dianggap hampir penuh kalau sisa < 5 GB", "8 program startup
= terlalu banyak") ada sebagai konstanta di awal tiap file `core/src/analyze/*.rs`, lengkap
dengan alasannya. Ubah di situ kalau mau lebih longgar/ketat.

Kalau teks di `core/src/analyze` diubah, perbarui data contoh untuk mode pratinjau:

```
cd src-tauri
cargo run -p repairdoc-core --example mock_report > ../src/lib/mock-report.json
```
