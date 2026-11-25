//! Tanggal sederhana tanpa dependensi: "YYYY-MM-DD" ↔ hari sejak 1970-01-01.
//! Cukup untuk menghitung "sudah berapa hari" (umur update, tanggal tes memori, dsb.).

use std::time::{SystemTime, UNIX_EPOCH};

/// Hari sejak 1970-01-01 untuk tanggal sipil (algoritma Howard Hinnant).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Kebalikan `days_from_civil`.
pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// "2026-03-14" (boleh diikuti jam) → hari sejak 1970.
pub fn parse_ymd(s: &str) -> Option<i64> {
    let head = s.trim().get(..10)?;
    let mut it = head.split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// "20240314" (format InstallDate di registry) → hari sejak 1970.
pub fn parse_compact(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    parse_ymd(&format!("{}-{}-{}", &s[..4], &s[4..6], &s[6..8]))
}

pub fn today() -> i64 {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    (secs / 86_400) as i64
}

/// Berapa hari yang lalu tanggal `ymd` dihitung dari `today` (negatif bila di masa depan).
pub fn days_ago(ymd: &str, today: i64) -> Option<i64> {
    parse_ymd(ymd).map(|d| today - d)
}

/// "sekitar 3 bulan lalu", "2 hari lalu", "hari ini", "lebih dari setahun lalu".
pub fn ago_text(days: i64) -> String {
    match days {
        i64::MIN..=-1 => "di masa depan (jam PC mungkin salah)".to_string(),
        0 => "hari ini".to_string(),
        1 => "kemarin".to_string(),
        2..=13 => format!("{days} hari lalu"),
        14..=59 => format!("{} minggu lalu", days / 7),
        60..=364 => format!("{} bulan lalu", days / 30),
        365..=729 => "lebih dari setahun lalu".to_string(),
        _ => format!("{} tahun lalu", days / 365),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_roundtrip() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
        for z in [-1000, 0, 1, 10_000, 20_000, 30_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(civil_from_days(days_from_civil(2026, 11, 25)), (2026, 11, 25));
    }

    #[test]
    fn parsing() {
        assert_eq!(parse_ymd("2026-11-25"), Some(days_from_civil(2026, 11, 25)));
        assert_eq!(parse_ymd("2026-11-25 08:12"), Some(days_from_civil(2026, 11, 25)));
        assert_eq!(parse_compact("20240314"), Some(days_from_civil(2024, 3, 14)));
        assert_eq!(parse_ymd("x"), None);
        assert_eq!(parse_ymd("2026-13-01"), None);
    }

    #[test]
    fn ago() {
        let t = days_from_civil(2026, 11, 25);
        assert_eq!(days_ago("2026-11-25", t), Some(0));
        assert_eq!(days_ago("2026-08-25", t), Some(92));
        assert_eq!(ago_text(0), "hari ini");
        assert_eq!(ago_text(5), "5 hari lalu");
        assert_eq!(ago_text(21), "3 minggu lalu");
        assert_eq!(ago_text(92), "3 bulan lalu");
        assert_eq!(ago_text(400), "lebih dari setahun lalu");
        assert_eq!(ago_text(800), "2 tahun lalu");
    }
}
