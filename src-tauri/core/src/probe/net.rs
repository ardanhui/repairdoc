//! Klien HTTP mini (hanya `http://`, tanpa dependensi) untuk dua keperluan:
//! cek internet seperti yang dilakukan Windows sendiri, dan tes unduh sederhana.
//! Bukan untuk hal lain — sengaja dibuat sekecil mungkin.

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub struct HttpSample {
    pub status: u16,
    /// Isi (dipotong `max_bytes`).
    pub body: Vec<u8>,
    /// Lama dari mulai sambung sampai selesai membaca.
    pub elapsed: Duration,
    /// Berhenti karena kehabisan waktu (bukan karena isi habis).
    pub timed_out: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Url {
    pub host: String,
    pub port: u16,
    pub path: String,
}

pub fn parse_http_url(url: &str) -> Result<Url, String> {
    let rest = url.strip_prefix("http://").ok_or("hanya http:// yang didukung")?;
    let (hostport, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().map_err(|_| "port tidak valid")?),
        None => (hostport, 80),
    };
    if host.is_empty() {
        return Err("host kosong".into());
    }
    Ok(Url { host: host.to_string(), port, path: path.to_string() })
}

/// Selesaikan nama domain dengan batas waktu (resolver OS tidak punya timeout sendiri).
pub fn resolve(host: &str, port: u16, timeout: Duration) -> Result<(Vec<std::net::SocketAddr>, Duration), String> {
    let (tx, rx) = mpsc::channel();
    let target = format!("{host}:{port}");
    let started = Instant::now();
    std::thread::spawn(move || {
        let r = target.to_socket_addrs().map(|it| it.collect::<Vec<_>>()).map_err(|e| e.to_string());
        let _ = tx.send(r);
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(addrs)) if !addrs.is_empty() => Ok((addrs, started.elapsed())),
        Ok(Ok(_)) => Err("tidak ada alamat".into()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!("DNS tidak menjawab dalam {} detik", timeout.as_secs())),
    }
}

/// GET sederhana. Berhenti setelah `max_bytes` isi atau `deadline` terlewati.
pub fn http_get(url: &str, max_bytes: usize, deadline: Duration) -> Result<HttpSample, String> {
    let u = parse_http_url(url)?;
    let started = Instant::now();
    let (addrs, _) = resolve(&u.host, u.port, Duration::from_secs(5))?;

    let mut stream = None;
    for addr in addrs.iter().take(3) {
        if let Ok(s) = TcpStream::connect_timeout(addr, Duration::from_secs(5)) {
            stream = Some(s);
            break;
        }
    }
    let mut stream = stream.ok_or("tidak bisa tersambung")?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    stream.set_write_timeout(Some(Duration::from_secs(5))).map_err(|e| e.to_string())?;
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: RepairDoc/{}\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        u.path,
        u.host,
        env!("CARGO_PKG_VERSION")
    );
    stream.write_all(req.as_bytes()).map_err(|e| format!("gagal mengirim permintaan: {e}"))?;

    let mut buf = Vec::with_capacity(16 * 1024);
    let mut chunk = [0u8; 16 * 1024];
    let mut header_end: Option<usize> = None;
    let mut status = 0u16;
    let mut timed_out = false;

    loop {
        if started.elapsed() >= deadline {
            timed_out = true;
            break;
        }
        let n = match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                timed_out = true;
                break;
            }
            Err(e) => return Err(format!("gagal membaca jawaban: {e}")),
        };
        buf.extend_from_slice(&chunk[..n]);
        if header_end.is_none() {
            if let Some(i) = find(&buf, b"\r\n\r\n") {
                header_end = Some(i + 4);
                let line = String::from_utf8_lossy(&buf[..i]);
                status = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).ok_or("jawaban bukan HTTP")?;
            } else if buf.len() > 64 * 1024 {
                return Err("header terlalu panjang".into());
            }
        }
        if let Some(h) = header_end {
            if buf.len() - h >= max_bytes {
                break;
            }
        }
    }
    let h = header_end.ok_or("jawaban tidak lengkap")?;
    let mut body = buf.split_off(h);
    body.truncate(max_bytes);
    Ok(HttpSample { status, body, elapsed: started.elapsed(), timed_out })
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// Server HTTP palsu: menjawab satu permintaan dengan isi sepanjang `body_len`,
    /// dikirim dalam potongan dengan jeda `delay` supaya batas waktu bisa diuji.
    fn serve(body_len: usize, delay: Duration) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap();
        std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut req = [0u8; 2048];
                let _ = s.read(&mut req);
                let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n");
                let _ = s.write_all(head.as_bytes());
                let mut left = body_len;
                while left > 0 {
                    let n = left.min(4096);
                    if s.write_all(&vec![b'a'; n]).is_err() {
                        break;
                    }
                    left -= n;
                    if !delay.is_zero() {
                        std::thread::sleep(delay);
                    }
                }
            }
        });
        format!("http://127.0.0.1:{}/file.bin", addr.port())
    }

    #[test]
    fn parses_urls() {
        assert_eq!(parse_http_url("http://example.com").unwrap(), Url { host: "example.com".into(), port: 80, path: "/".into() });
        assert_eq!(parse_http_url("http://h:8080/a/b?c=1").unwrap().port, 8080);
        assert!(parse_http_url("https://x").is_err());
    }

    #[test]
    fn reads_small_body() {
        let url = serve(100, Duration::ZERO);
        let s = http_get(&url, 10_000, Duration::from_secs(5)).unwrap();
        assert_eq!((s.status, s.body.len(), s.timed_out), (200, 100, false));
    }

    #[test]
    fn caps_at_max_bytes() {
        let url = serve(50_000, Duration::ZERO);
        let s = http_get(&url, 8_000, Duration::from_secs(5)).unwrap();
        assert_eq!(s.body.len(), 8_000);
    }

    #[test]
    fn stops_at_deadline() {
        let url = serve(400_000, Duration::from_millis(40));
        let s = http_get(&url, 10_000_000, Duration::from_millis(300)).unwrap();
        assert!(s.timed_out);
        assert!(s.body.len() < 400_000);
        assert!(s.elapsed < Duration::from_secs(3));
    }

    #[test]
    fn resolves_localhost() {
        let (addrs, t) = resolve("localhost", 80, Duration::from_secs(5)).unwrap();
        assert!(!addrs.is_empty());
        assert!(t < Duration::from_secs(5));
    }

    #[test]
    fn connection_refused_is_error() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        assert!(http_get(&format!("http://127.0.0.1:{port}/"), 100, Duration::from_secs(2)).is_err());
    }
}
