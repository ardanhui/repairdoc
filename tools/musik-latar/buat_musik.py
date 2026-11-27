"""Musik latar orisinal RepairDoc: lagu santai-ceria, 16 birama, G mayor, 108 BPM.
Komposisi & sintesis asli (tidak memakai sampel atau melodi lagu lain)."""
import numpy as np
from scipy.signal import lfilter, butter
SR = 44100
BPM = 108
BEAT = 60 / BPM
E8 = BEAT / 2            # satu not seperdelapan
BAR = 4 * BEAT
BARS = 16
LOOP = BARS * BAR
N = int(round(LOOP * SR))
TOTAL = 3 * N            # render 3 putaran; ambil putaran tengah supaya sambungan loop mulus

def hz(m): return 440.0 * 2 ** ((m - 69) / 12)
def note(name):
    names = {'C':0,'C#':1,'D':2,'D#':3,'E':4,'F':5,'F#':6,'G':7,'G#':8,'A':9,'A#':10,'B':11}
    n, o = name[:-1], int(name[-1])
    return 12 * (o + 1) + names[n]

L = np.zeros(TOTAL); R = np.zeros(TOTAL)
wetL = np.zeros(TOTAL); wetR = np.zeros(TOTAL)

def add(sig, t0, pan=0.0, gain=1.0, wet=0.0):
    """Taruh sinyal di posisi waktu t0 (detik) pada ketiga putaran."""
    gl = gain * np.cos((pan + 1) * np.pi / 4); gr = gain * np.sin((pan + 1) * np.pi / 4)
    for k in range(3):
        i = int(round((t0 + k * LOOP) * SR))
        j = min(TOTAL, i + len(sig))
        if i >= TOTAL: continue
        s = sig[: j - i]
        L[i:j] += s * gl; R[i:j] += s * gr
        if wet:
            wetL[i:j] += s * gl * wet; wetR[i:j] += s * gr * wet

def env(n, a, tau, rel, dur):
    t = np.arange(n) / SR
    e = np.minimum(1, t / a) * np.exp(-t / tau)
    off = t > dur
    e[off] *= np.exp(-(t[off] - dur) / rel)
    return e

# ---------------------------------------------------------------- instrumen
def epiano(m, dur, vel):
    """Piano elektrik (FM): bunyi lembut dengan sedikit "tine" di awal."""
    n = int((dur + 1.2) * SR); t = np.arange(n) / SR; f = hz(m)
    idx = 1.1 * np.exp(-t / 0.35) + 0.15
    mod = np.sin(2 * np.pi * f * t) * idx
    tine = 0.25 * np.exp(-t / 0.03) * np.sin(2 * np.pi * f * 14 * t)
    s = np.sin(2 * np.pi * f * t + mod) + tine
    s *= env(n, 0.006, 1.4, 0.25, dur)
    s *= 1 + 0.12 * np.sin(2 * np.pi * 4.8 * t)
    return s * vel * 0.155

def vibes(m, dur, vel):
    """Vibrafon: nada dasar + parsial ke-4, dengan getar motor lembut."""
    n = int((dur + 2.0) * SR); t = np.arange(n) / SR; f = hz(m)
    s = (np.sin(2 * np.pi * f * t)
         + 0.36 * np.exp(-t / 0.4) * np.sin(2 * np.pi * f * 4.0 * t)
         + 0.06 * np.exp(-t / 0.05) * np.sin(2 * np.pi * f * 9.9 * t))
    s *= env(n, 0.003, 1.6, 0.35, dur)
    s *= 1 + 0.22 * np.sin(2 * np.pi * 5.6 * t)
    return s * vel * 0.30

def bass(m, dur, vel):
    n = int((dur + 0.3) * SR); t = np.arange(n) / SR; f = hz(m)
    # harmonik ke-2/3/4 cukup kuat supaya bass tetap terdengar di speaker laptop
    s = np.sin(2 * np.pi * f * t) + 0.55 * np.sin(4 * np.pi * f * t) + 0.3 * np.sin(6 * np.pi * f * t) + 0.12 * np.sin(8 * np.pi * f * t)
    s *= env(n, 0.008, 0.9, 0.07, dur)
    return s * vel * 0.24

rng = np.random.default_rng(7)
def shaker(vel):
    n = int(0.09 * SR); t = np.arange(n) / SR
    b, a = butter(2, [5500 / (SR / 2), 12000 / (SR / 2)], btype='band')
    s = lfilter(b, a, rng.standard_normal(n))
    s *= np.minimum(1, t / 0.006) * np.exp(-t / 0.028)
    return s * vel * 0.09

def rim(vel):
    n = int(0.08 * SR); t = np.arange(n) / SR
    b, a = butter(2, [1500 / (SR / 2), 4000 / (SR / 2)], btype='band')
    s = lfilter(b, a, rng.standard_normal(n)) * np.exp(-t / 0.006)
    s += 0.5 * np.sin(2 * np.pi * 1750 * t) * np.exp(-t / 0.018)
    return s * vel * 0.16

def kick(vel):
    n = int(0.3 * SR); t = np.arange(n) / SR
    f = 50 + 70 * np.exp(-t / 0.035)
    s = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.16)
    return s * vel * 0.18

# ---------------------------------------------------------------- akor
V = {
 'Gmaj7': ['B3','D4','F#4','A4'], 'Em7': ['G3','B3','D4','F#4'], 'Am7': ['C4','E4','G4','B4'],
 'D7': ['C4','E4','F#4','A4'], 'Bm7': ['A3','B3','D4','F#4'], 'E7': ['D4','F#4','G#4','B4'],
 'D7sus4': ['C4','D4','G4','A4'], 'Cmaj7': ['B3','D4','E4','G4'],
}
ROOT = {'Gmaj7':'G2','Em7':'E2','Am7':'A2','D7':'D2','Bm7':'B2','E7':'E2','D7sus4':'D2','Cmaj7':'C3'}
FIFTH = {'Gmaj7':'D3','Em7':'B2','Am7':'E3','D7':'A2','Bm7':'F#3','E7':'B2','D7sus4':'A2','Cmaj7':'G3'}
PROG = [['Gmaj7'],['Em7'],['Am7'],['D7'],['Gmaj7'],['Bm7','E7'],['Am7'],['D7sus4','D7'],
        ['Cmaj7'],['Bm7'],['Am7'],['D7'],['Gmaj7'],['Em7'],['Am7'],['D7']]

for b, chords in enumerate(PROG):
    t0 = b * BAR
    if len(chords) == 1:
        c = chords[0]
        hits = [(0, 2, 0.85, c), (3, 2, 0.6, c), (6, 2, 0.7, c)]
    else:
        hits = [(0, 3, 0.85, chords[0]), (4, 2, 0.75, chords[1]), (6, 2, 0.6, chords[1])]
    for slot, d, vel, c in hits:
        for i, nm in enumerate(V[c]):
            add(epiano(note(nm), d * E8 * 0.95, vel), t0 + slot * E8 + i * 0.006, pan=-0.25, gain=1, wet=0.35)
    # bass: akar (3/8) – kuint – kuint – akar/pendekat
    c1 = chords[0]; c2 = chords[-1]
    for slot, d, nm, vel in [(0, 3, ROOT[c1], 0.95), (3, 1, FIFTH[c1], 0.6), (4, 3, ROOT[c2] if len(chords) > 1 else FIFTH[c1], 0.8), (7, 1, ROOT[c2], 0.55)]:
        add(bass(note(nm), d * E8 * 0.9, vel), t0 + slot * E8, pan=0, gain=1, wet=0.05)
    # perkusi
    for slot in range(8):
        add(shaker(0.9 if slot % 2 else 0.55), t0 + slot * E8, pan=0.45, wet=0.15)
    for slot in (0, 4):
        add(kick(0.8 if slot == 0 else 0.55), t0 + slot * E8)
    clave = (0, 3, 6) if b % 2 == 0 else (2, 4)
    for slot in clave:
        add(rim(0.8), t0 + slot * E8, pan=-0.35, wet=0.2)

# ---------------------------------------------------------------- melodi (orisinal)
# (nada, panjang dalam not 1/8). 'r' = diam. Tiap birama = 8.
MEL = [
 [('D5',2),('B4',1),('D5',1),('F#5',3),('E5',1)],
 [('D5',4),('B4',2),('G4',2)],
 [('A4',1),('B4',1),('C5',2),('E5',2),('G5',1),('F#5',1)],
 [('E5',3),('D5',1),('C5',2),('A4',2)],
 [('B4',2),('D5',1),('G5',1),('F#5',2),('D5',2)],
 [('F#5',2),('D5',2),('B4',1),('G#4',1),('B4',2)],
 [('C5',2),('E5',2),('A5',3),('G5',1)],
 [('F#5',4),('E5',1),('D5',1),('C5',2)],
 [('B4',2),('E5',1),('G5',1),('B5',2),('A5',2)],
 [('F#5',3),('D5',1),('B4',4)],
 [('C5',1),('D5',1),('E5',2),('G5',1),('E5',1),('C5',2)],
 [('D5',2),('F#5',2),('A5',2),('C6',2)],
 [('B5',3),('A5',1),('G5',2),('F#5',2)],
 [('E5',2),('D5',1),('B4',1),('G4',4)],
 [('A4',1),('C5',1),('E5',2),('G5',2),('E5',2)],
 [('F#5',2),('E5',2),('D5',1),('C5',1),('A4',2)],
]
for b, bar in enumerate(MEL):
    assert sum(d for _, d in bar) == 8, (b, bar)
    pos = 0
    for nm, d in bar:
        if nm != 'r':
            vel = 0.9 if pos % 4 == 0 else 0.75
            add(vibes(note(nm), d * E8 * 0.92, vel), b * BAR + pos * E8, pan=0.25, wet=0.45)
        pos += d

# ---------------------------------------------------------------- gema (reverb Schroeder sederhana)
def reverb(x, combs, aps, g=0.8, damp=0.35):
    out = np.zeros_like(x)
    for d in combs:
        # comb dengan peredam frekuensi tinggi: y[n] = x[n] + g*((1-damp)*y[n-d] + damp*y[n-d-1])
        a = np.zeros(d + 2); a[0] = 1; a[d] = -g * (1 - damp); a[d + 1] = -g * damp
        out += lfilter([1], a, x)
    out /= len(combs)
    for d in aps:
        b = np.zeros(d + 1); b[0] = -0.5; b[d] = 1
        a = np.zeros(d + 1); a[0] = 1; a[d] = -0.5
        out = lfilter(b, a, out)
    return out
revL = reverb(wetL, [1557, 1617, 1491, 1422, 1277, 1356], [225, 556, 441])
revR = reverb(wetR, [1580, 1640, 1514, 1445, 1300, 1379], [248, 579, 464])
L += revL * 1.2; R += revR * 1.2

# filter akhir dijalankan pada ketiga putaran SEBELUM diambil putaran tengahnya,
# supaya keadaan filter di awal & akhir sama → sambungan loop mulus.
full = np.stack([L, R], axis=1)
b, a = butter(2, 45 / (SR / 2), btype='high'); full = lfilter(b, a, full, axis=0)
b, a = butter(2, 11500 / (SR / 2), btype='low'); full = lfilter(b, a, full, axis=0)
# sedikit kilau: tambah komponen di atas 2 kHz (shelf sederhana)
b, a = butter(1, 2000 / (SR / 2), btype='high'); full = full + 0.35 * lfilter(b, a, full, axis=0)
# ambil putaran tengah (sudah berisi ekor gema putaran sebelumnya → loop tanpa sambungan)
out = full[N:2 * N]
# kompresi lembut lalu normalisasi ke puncak -3 dB
out = np.tanh(out * 1.6) / 1.6
out *= (10 ** (-3 / 20)) / np.max(np.abs(out))
import wave
pcm = (out * 32767).astype('<i2')
with wave.open('musik-latar.wav', 'wb') as w:
    w.setnchannels(2); w.setsampwidth(2); w.setframerate(SR); w.writeframes(pcm.tobytes())
print("panjang loop %.3f detik, %d sampel" % (LOOP, N))
