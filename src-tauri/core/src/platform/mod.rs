//! Lapisan sistem operasi: menjalankan PowerShell, cek izin admin, dsb.
//! Kode khusus Windows ada di `windows.rs`; di sistem lain fungsi-fungsinya
//! mengembalikan "tidak tersedia" supaya logika diagnosa tetap bisa dites.

use std::io::Read;
use std::process::Child;
use std::time::{Duration, Instant};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use self::windows::*;

#[cfg(not(windows))]
mod other;
#[cfg(not(windows))]
pub use self::other::*;

/// Huruf drive tempat RepairDoc dijalankan (biasanya flashdisk), misalnya 'E'.
/// Drive ini dilewati saat diagnosa supaya flashdisk tidak ikut dinilai.
pub fn exe_drive_letter() -> Option<char> {
    let exe = std::env::current_exe().ok()?;
    drive_letter_of(&exe.to_string_lossy())
}

/// Huruf drive tempat Windows terpasang (biasanya 'C').
pub fn system_drive_letter() -> Option<char> {
    match std::env::var("SystemDrive") {
        Ok(v) => drive_letter_of(&v),
        Err(_) => {
            if cfg!(windows) {
                Some('C')
            } else {
                None
            }
        }
    }
}

/// Ambil huruf drive dari path seperti `E:\RepairDoc.exe` atau `\\?\E:\...`.
pub fn drive_letter_of(path: &str) -> Option<char> {
    let p = path.strip_prefix(r"\\?\").unwrap_or(path);
    let mut chars = p.chars();
    let letter = chars.next()?;
    if letter.is_ascii_alphabetic() && chars.next() == Some(':') {
        Some(letter.to_ascii_uppercase())
    } else {
        None
    }
}

/// Tunggu proses selesai paling lama `timeout`; kalau lewat, proses dihentikan.
/// Mengembalikan stdout (UTF-8) bila proses keluar dengan sukses.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn wait_with_timeout(mut child: Child, timeout: Duration, what: &str) -> Result<String, String> {
    let mut stdout = child.stdout.take().ok_or("stdout tidak tersedia")?;
    let mut stderr = child.stderr.take().ok_or("stderr tidak tersedia")?;
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{what} tidak selesai dalam {} detik", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(format!("{what} gagal dipantau: {e}")),
        }
    };

    let out = out_thread.join().unwrap_or_default();
    let err = err_thread.join().unwrap_or_default();
    let out = String::from_utf8_lossy(&out).trim_start_matches('\u{feff}').trim().to_string();
    if status.success() {
        Ok(out)
    } else {
        let err = String::from_utf8_lossy(&err).trim().to_string();
        Err(format!("{what} keluar dengan kode {:?}: {}", status.code(), first_line(&err)))
    }
}

#[cfg_attr(not(windows), allow(dead_code))]
fn first_line(s: &str) -> &str {
    s.lines().find(|l| !l.trim().is_empty() && !l.starts_with("#< CLIXML")).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_drive_letters() {
        assert_eq!(drive_letter_of(r"E:\RepairDoc.exe"), Some('E'));
        assert_eq!(drive_letter_of(r"\\?\f:\x\RepairDoc.exe"), Some('F'));
        assert_eq!(drive_letter_of("C:"), Some('C'));
        assert_eq!(drive_letter_of("/usr/bin"), None);
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_slow_process() {
        use std::process::{Command, Stdio};
        let child = Command::new("sleep").arg("5").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let start = Instant::now();
        let r = wait_with_timeout(child, Duration::from_millis(300), "tes");
        assert!(r.is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn captures_output() {
        use std::process::{Command, Stdio};
        let child = Command::new("echo").arg("halo").stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        assert_eq!(wait_with_timeout(child, Duration::from_secs(5), "tes").unwrap(), "halo");
    }
}
