//! Decoders for raw `DEVPROP_TYPE_*` property buffers.
//!
//! Pure functions over byte slices, so they compile and are tested on every
//! platform even though only the Windows back end produces such buffers.

use chrono::{DateTime, NaiveDate};
use uuid::Uuid;

/// Decodes a NUL-terminated UTF-16LE string.
pub fn utf16_string(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16_lossy(&units)
}

/// Decodes a `REG_MULTI_SZ`-style list: UTF-16LE strings each ending in NUL,
/// the whole list ending in an extra NUL.
pub fn utf16_multi_sz(bytes: &[u8]) -> Vec<String> {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    units
        .split(|&u| u == 0)
        .filter(|s| !s.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

/// Decodes a little-endian `u32`.
pub fn u32_le(bytes: &[u8]) -> Option<u32> {
    bytes
        .get(..4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Decodes a `DEVPROP_BOOLEAN` (one byte, `0xFF` true, `0x00` false).
pub fn boolean(bytes: &[u8]) -> Option<bool> {
    bytes.first().map(|&b| b != 0)
}

/// Decodes a 16-byte `GUID` (mixed-endian, as laid out in memory on Windows).
pub fn guid(bytes: &[u8]) -> Option<Uuid> {
    let b: [u8; 16] = bytes.get(..16)?.try_into().ok()?;
    let d1 = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
    let d2 = u16::from_le_bytes([b[4], b[5]]);
    let d3 = u16::from_le_bytes([b[6], b[7]]);
    let d4: [u8; 8] = b[8..16].try_into().ok()?;
    Some(Uuid::from_fields(d1, d2, d3, &d4))
}

/// Converts a `FILETIME` (100 ns ticks since 1601-01-01 UTC) to a calendar date.
pub fn filetime_date(bytes: &[u8]) -> Option<NaiveDate> {
    let b = bytes.get(..8)?;
    let ticks = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    const UNIX_EPOCH_TICKS: u64 = 116_444_736_000_000_000;
    let secs = i64::try_from(ticks / 10_000_000).ok()?
        - i64::try_from(UNIX_EPOCH_TICKS / 10_000_000).ok()?;
    DateTime::from_timestamp(secs, 0).map(|dt| dt.date_naive())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wide(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn decodes_strings_and_lists() {
        let mut buf = wide("NVIDIA");
        buf.extend([0, 0]);
        assert_eq!(utf16_string(&buf), "NVIDIA");

        let mut list = wide("A\\B");
        list.extend([0, 0]);
        list.extend(wide("Второй"));
        list.extend([0, 0, 0, 0]);
        assert_eq!(
            utf16_multi_sz(&list),
            vec!["A\\B".to_owned(), "Второй".to_owned()]
        );
        assert!(utf16_multi_sz(&[0, 0]).is_empty());
    }

    #[test]
    fn decodes_guid_with_windows_layout() {
        // GUID_DEVCLASS_DISPLAY {4d36e968-e325-11ce-bfc1-08002be10318}
        let raw = [
            0x68, 0xe9, 0x36, 0x4d, 0x25, 0xe3, 0xce, 0x11, 0xbf, 0xc1, 0x08, 0x00, 0x2b, 0xe1,
            0x03, 0x18,
        ];
        assert_eq!(
            guid(&raw).unwrap().to_string(),
            "4d36e968-e325-11ce-bfc1-08002be10318"
        );
        assert!(guid(&raw[..15]).is_none());
    }

    #[test]
    fn decodes_filetime() {
        // 2006-06-21, the date stamped on every inbox driver: build the
        // FILETIME from the date instead of trusting a hand-computed constant.
        let expected = NaiveDate::from_ymd_opt(2006, 6, 21).unwrap();
        let unix_secs = expected.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        let ticks = (u64::try_from(unix_secs).unwrap() + 11_644_473_600) * 10_000_000;
        assert_eq!(filetime_date(&ticks.to_le_bytes()), Some(expected));
        // Known reference value: 2006-06-21 00:00:00 UTC.
        assert_eq!(
            filetime_date(&127_953_216_000_000_000u64.to_le_bytes()),
            Some(expected)
        );
        assert!(filetime_date(&[1, 2, 3]).is_none());
    }

    #[test]
    fn decodes_scalars() {
        assert_eq!(u32_le(&[28, 0, 0, 0]), Some(28));
        assert_eq!(u32_le(&[1, 2]), None);
        assert_eq!(boolean(&[0xFF]), Some(true));
        assert_eq!(boolean(&[0]), Some(false));
        assert_eq!(boolean(&[]), None);
    }
}
