//! Format angka gaya Indonesia: koma untuk desimal ("3,2 GB").

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

pub fn bytes_to_gib(bytes: u64) -> f64 {
    bytes as f64 / GIB
}

/// Ukuran yang mudah dibaca: "3,2 GB" untuk < 10 GB, "118 GB" untuk yang lebih besar.
/// Memakai basis 1024 seperti yang ditampilkan Windows Explorer.
pub fn size(bytes: u64) -> String {
    let gib = bytes_to_gib(bytes);
    if gib >= 1024.0 {
        return format!("{} TB", decimal(gib / 1024.0, 1));
    }
    if gib < 1.0 {
        let mib = bytes as f64 / (1024.0 * 1024.0);
        return format!("{} MB", mib.round() as u64);
    }
    if gib < 10.0 {
        format!("{} GB", decimal(gib, 1))
    } else {
        format!("{} GB", gib.round() as u64)
    }
}

/// Angka dengan koma desimal, nol di belakang dibuang ("4,0" → "4").
pub fn decimal(value: f64, places: usize) -> String {
    let s = format!("{:.*}", places, value);
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    s.replace('.', ",")
}

pub fn percent(value: f64) -> String {
    format!("{}%", value.round() as i64)
}

/// Perkiraan ukuran RAM yang terpasang dari angka yang terbaca Windows.
/// Windows sering melaporkan sedikit di bawah (7,9 GB untuk 8 GB).
pub fn installed_gb_guess(total_bytes: u64) -> u64 {
    bytes_to_gib(total_bytes).round().max(1.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_are_indonesian() {
        assert_eq!(size(3_435_973_837), "3,2 GB");
        assert_eq!(size(126_701_535_232), "118 GB");
        assert_eq!(size(4 * 1024 * 1024 * 1024), "4 GB");
        assert_eq!(size(500 * 1024 * 1024), "500 MB");
        assert_eq!(size(2 * 1024 * 1024 * 1024 * 1024), "2 TB");
    }

    #[test]
    fn decimal_trims_zero() {
        assert_eq!(decimal(4.0, 1), "4");
        assert_eq!(decimal(7.88, 1), "7,9");
    }

    #[test]
    fn installed_guess() {
        assert_eq!(installed_gb_guess((7.88 * GIB) as u64), 8);
        assert_eq!(installed_gb_guess((3.86 * GIB) as u64), 4);
    }
}
