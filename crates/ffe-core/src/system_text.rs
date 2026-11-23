//! The original's string expansion for the system and map pages (F830, F1343,
//! F1347/F1348 and the FF30 variable writers F1349-F1396).
//!
//! A system string is raw bytes in which `0xff hi lo` is an expansion code. The
//! codes these pages use are:
//!
//! * `0x3001..=0x300c`: a variable written as a decimal integer (`1..=3`, F1376,
//!   F1375, F1374), or as a fraction with one, two or three decimal places
//!   (`4..=12`, F1378-F1386), each family taking variable 1, 2 or 3;
//! * `0x3016`/`0x3017`: variable 4 or 5 as a string (F1364/F1365's recursive
//!   expansion is not needed by the system pages and writes nothing);
//! * `0x3018..=0x301c`: variable 1 to 5 as another expansion code (F1391-F1387);
//! * `0x301d`/`0x301e`: variable 4 or 5 as an integer (F1372, F1373);
//! * `0x301f..=0x3021`: variable 1 to 3 in tonnes (F1371-F1369);
//! * `0x8000..`: a string of the system table by its selector (F1343's `0x8000`
//!   branch for the handler that F830 uses), expanded in turn.
//!
//! The layout bytes (`0xfd`, `0xfe`, `0x0d`) are left in the text for the writer
//! (F1399/F1589 convert and draw them).

/// The recursion limit: F1343's buffer is 2 KiB and the original's strings nest
/// two levels at most.
const DEPTH: usize = 8;

/// F1545: `value` in decimal, zero-padded to at least `pad + 1` digits, with a
/// leading `-` for a negative value.
fn write_int(value: i32, pad: usize) -> Vec<u8> {
    let mut digits = Vec::new();
    let mut rest = i64::from(value).unsigned_abs();
    loop {
        digits.push(b'0' + (rest % 10) as u8);
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    let mut out = Vec::new();
    if value < 0 {
        out.push(b'-');
    }
    out.extend(std::iter::repeat_n(
        b'0',
        (pad + 1).saturating_sub(digits.len()),
    ));
    out.extend(digits.iter().rev());
    out
}

/// F1377: `value` with `places` decimal places. The original pads the whole
/// string (a sign included) with zeros to `places + 1` characters, then moves
/// the last `places` characters right and writes the point.
fn write_fraction(value: i32, places: usize) -> Vec<u8> {
    let mut text = write_int(value, 0);
    if text.len() <= places {
        let mut padded = vec![b'0'; places + 1 - text.len()];
        padded.extend(&text);
        text = padded;
    }
    let point = text.len() - places;
    let mut out = text[..point].to_vec();
    out.push(b'.');
    out.extend(&text[point..]);
    out
}

/// Expand `raw` with the five variables `vars` (the original's `StrVars`);
/// `lookup` returns the raw string for a system-table selector (`0x84xx` and
/// the like), or `None` when the table has no such string.
pub fn expand(raw: &[u8], vars: &[i32; 5], lookup: &dyn Fn(u16) -> Option<Vec<u8>>) -> Vec<u8> {
    expand_at(raw, vars, lookup, 0)
}

fn expand_at(
    raw: &[u8],
    vars: &[i32; 5],
    lookup: &dyn Fn(u16) -> Option<Vec<u8>>,
    depth: usize,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + 16);
    let mut at = 0;
    while at < raw.len() {
        let byte = raw[at];
        match byte {
            0 => break,
            0xff => {
                let (Some(high), Some(low)) = (raw.get(at + 1), raw.get(at + 2)) else {
                    break;
                };
                let code = u16::from(*high) << 8 | u16::from(*low);
                at += 3;
                out.extend(expand_code(code, vars, lookup, depth));
            }
            // F1348 keeps `0xfd XX` and `0xfe XX YY` with their operands.
            0xfd => {
                out.extend(raw[at..raw.len().min(at + 2)].iter());
                at += 2;
            }
            0xfe => {
                out.extend(raw[at..raw.len().min(at + 3)].iter());
                at += 3;
            }
            _ => {
                out.push(byte);
                at += 1;
            }
        }
    }
    out
}

/// F1343 for one code.
pub fn expand_code(
    code: u16,
    vars: &[i32; 5],
    lookup: &dyn Fn(u16) -> Option<Vec<u8>>,
    depth: usize,
) -> Vec<u8> {
    if depth >= DEPTH {
        return Vec::new();
    }
    let var = |index: usize| vars[index.min(4)];
    if code >= 0x8000 {
        return lookup(code).map_or_else(Vec::new, |raw| expand_at(&raw, vars, lookup, depth + 1));
    }
    if code < 0x3000 {
        return Vec::new();
    }
    match code - 0x3000 {
        function @ 1..=3 => write_int(var(usize::from(function) - 1), 0),
        function @ 4..=12 => {
            let family = usize::from(function - 4);
            write_fraction(var(family % 3), family / 3 + 1)
        }
        function @ 0x18..=0x1c => expand_code(
            var(usize::from(function) - 0x18) as u16,
            vars,
            lookup,
            depth + 1,
        ),
        0x1d => write_int(var(3), 0),
        0x1e => write_int(var(4), 0),
        function @ 0x1f..=0x21 => {
            let value = var(usize::from(function) - 0x1f);
            if value == 0 {
                b"-".to_vec()
            } else {
                let mut text = write_int(value, 0);
                text.push(b't');
                text
            }
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: u16) -> Option<Vec<u8>> {
        None
    }

    #[test]
    fn integers_and_fractions_follow_f1545_and_f1377() {
        assert_eq!(write_int(15, 0), b"15");
        assert_eq!(write_int(-99, 0), b"-99");
        assert_eq!(write_fraction(29902, 2), b"299.02");
        assert_eq!(write_fraction(5, 2), b"0.05");
        assert_eq!(write_fraction(1047, 1), b"104.7");
        assert_eq!(write_fraction(21_050, 3), b"21.050");
    }

    #[test]
    fn the_body_count_line_expands_ff3001() {
        // 0x84a0: "Stable system with \xff\x30\x01 major bodies."
        let raw = b"Stable system with \xff\x30\x01 major bodies.";
        let text = expand(raw, &[15, 0, 0, 0, 0], &none);
        assert_eq!(text, b"Stable system with 15 major bodies.");
    }

    #[test]
    fn a_variable_holding_a_code_expands_that_string() {
        // 0x84b5's "Government Type:\xfd\x41\xff\x30\x18": variable 1 is a code.
        let lookup = |code: u16| (code == 0x84d0).then(|| b"Federal Democracy".to_vec());
        let raw = b"Government Type:\xfd\x41\xff\x30\x18";
        let text = expand(raw, &[0x84d0, 0, 0, 0, 0], &lookup);
        assert_eq!(text, b"Government Type:\xfd\x41Federal Democracy");
    }
}
