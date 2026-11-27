# Musik latar RepairDoc

`musik-latar.ogg` adalah lagu **orisinal** yang dibuat khusus untuk RepairDoc pada
27 November 2026: komposisi (akor, bass, melodi, pola perkusi) dan suaranya dibuat dari nol
dengan sintesis di `tools/musik-latar/buat_musik.py`. Tidak memakai sampel, rekaman, atau
melodi lagu lain.

- Format: Ogg Opus 96 kbps, stereo, loop 35,56 detik (16 birama, 108 BPM, G mayor).
- Boleh dipakai, diubah, dan dibagikan bersama RepairDoc dengan lisensi yang sama dengan
  proyek ini.

Membuat ulang (butuh Python 3 + numpy + scipy, dan ffmpeg):

```
cd tools/musik-latar
python buat_musik.py
ffmpeg -i musik-latar.wav -c:a libopus -b:a 96k -vbr on -application audio ../../src/assets/musik-latar.ogg
```
