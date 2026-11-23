//! F439's View page: what the shipyard's right-hand button shows for a hull.
//!
//! The page is the single-row ship list (F437's row, drawn by the station screen)
//! with the hull spinning in a window at the left (F559/F562, the status page's
//! spin) and four text blocks to the right, all drawn by F1399 through F1397's
//! expansion: the hull's masses, mountings and crew (`0x9913`, at 135,25), its
//! thrusters, drive and registration code (`0x9914`, at 135,75) and the
//! hyperspace range for each of classes 1 to 7 (`0x9915` at 135,115, `0x9916` at
//! 215,135, and class 7 at the end of `0x9914`). See `docs/notes/station.md`.
use ffe_core::equipment::{self, Category};
use ffe_core::names::{self, Names};
use ffe_core::system_text;

/// One expanded string and where F1399 starts it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub x: i32,
    pub y: i32,
    pub bytes: Vec<u8>,
}

/// The View page of one hull: the model, the word that places and spins it and
/// the text blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    pub model: u32,
    /// The statistics' `+0xc` word: F559 places the model by it.
    pub shield: i16,
    pub texts: Vec<Text>,
}

/// `DATA_005929`: mass, hold, gun mountings, missile pylons and crew.
const STATS: &[u8] = b"Mass (fully laden)\xfd{\xff0\x1f\rInternal Capacity (no drive)\xfd{\xff0 \rGun Mountings\xfd{\xff0\x03\rMissile Pylons\xfd{\xff0\x1d\rCrew\xfd{\xff0\x1e";
/// `DATA_005930`: thrusters, drive and registration (`0xff0\x17`, spliced), then
/// class 7's range at (292,135).
const MOTION: &[u8] = b"Main Thruster Acceleration\xfd{\xff0\x04 Earth g\rRetro Thruster Acceleration\xfd{\xff0\x05 Earth g\rDrive Fitted\xfd{\xff0\x1b\rRegistration Code\xfd{\xff0\x17\xfe\x92\x87\xff0\x09";
/// `DATA_005931`: the ranges' heading, the class names and classes 1 to 3.
const RANGES_1: &[u8] = b"\x01\x02Hyperspace Ranges:\r\x01\x03Class 1  Class 2  Class 3  Class 4  Class 5  Class 6  Class 7\r \xff0\x07\xfdQ\xff0\x08\xfd^\xff0\x09";
/// `DATA_005932`: classes 4 to 6.
const RANGES_2: &[u8] = b"\xff0\x07\xfdx\xff0\x08\xfd\x85\xff0\x09";

/// `DATA_005301`: the drive classes' name codes (`ship+0xd0` indexes it).
/// The names are the equipment table's.
fn drive_name(class: i8) -> Vec<u8> {
    if class <= 0 {
        return b"None".to_vec();
    }
    equipment::equipment()
        .iter()
        .find(|item| item.category == Category::Drive && i16::from(item.code) == i16::from(class))
        .map_or_else(Vec::new, |item| item.name.as_bytes().to_vec())
}

/// F902's hyperspace ranges for classes 1 to 7: `DATA_007188`'s numerator for
/// drive `class + 1` over the hull mass (1 when it is zero), zero when the hold
/// statistic does not exceed that drive's mass or the result is under 300.
pub fn ranges(mass: i16, cargo: i16) -> [i32; 7] {
    const DRIVE_MASS: [i32; 7] = [6, 12, 24, 80, 150, 250, 400];
    const NUMERATOR: [i32; 7] = [
        30_000, 120_000, 270_000, 480_000, 750_000, 1_080_000, 1_470_000,
    ];
    let divisor = if mass == 0 { 1 } else { i32::from(mass) };
    std::array::from_fn(|k| {
        if i32::from(cargo) <= DRIVE_MASS[k] {
            return 0;
        }
        let range = NUMERATOR[k] / divisor;
        if range < 300 {
            0
        } else {
            range
        }
    })
}

/// A word of the statistics block at `0x48 * model`.
fn word(record: &[u8], offset: usize) -> i16 {
    i16::from_le_bytes([record[offset], record[offset + 1]])
}

/// F439's text blocks and the model's placement for `model`, with the
/// registration code F791 makes from the stock entry's `seed`. `None` without
/// the cache's statistics.
pub fn page(cache: &ffe_content::VerifiedCache, model: u32, seed: u16) -> Option<Page> {
    let stats = cache.asset("base:ship/stats")?;
    let at = 48 * model as usize;
    let record = stats.get(at..at + 48)?;
    let registration = Names::new(cache.asset("base:text/names")?)
        .ok()
        .and_then(|names| names.generate(names::kind::SHIP, u32::from(seed).wrapping_mul(0x4a0d)))
        .unwrap_or_default();
    let mass = word(record, 6);
    let cargo = word(record, 8);
    let ranges = ranges(mass, cargo);
    let class = record[0x14] as i8;
    let name = drive_name(class);
    let lookup = |code: u16| (code == 0x9000).then(|| name.clone());
    let expand = |raw: &[u8], vars: [i32; 5]| system_text::expand(raw, &vars, &lookup);

    let stats_text = expand(
        STATS,
        [
            i32::from(mass),
            i32::from(cargo),
            i32::from(record[4] & 0xf),
            i32::from(word(record, 0x12)),
            i32::from(word(record, 0x10)),
        ],
    );
    // F1343 has no function 0x17: the registration string is vars[4].
    let mut motion_raw = Vec::new();
    let mut at = 0;
    while at < MOTION.len() {
        if MOTION[at..].starts_with(b"\xff0\x17") {
            motion_raw.extend(&registration);
            at += 3;
        } else {
            motion_raw.push(MOTION[at]);
            at += 1;
        }
    }
    let motion_text = expand(
        &motion_raw,
        [
            i32::from(word(record, 0)) / 0x36,
            -i32::from(word(record, 2)) / 0x36,
            ranges[6],
            0x9000,
            0,
        ],
    );
    let ranges_1 = expand(RANGES_1, [ranges[0], ranges[1], ranges[2], 0, 0]);
    let ranges_2 = expand(RANGES_2, [ranges[3], ranges[4], ranges[5], 0, 0]);
    Some(Page {
        model,
        shield: word(record, 0xc),
        texts: vec![
            Text {
                x: 135,
                y: 25,
                bytes: stats_text,
            },
            Text {
                x: 135,
                y: 75,
                bytes: motion_text,
            },
            Text {
                x: 135,
                y: 115,
                bytes: ranges_1,
            },
            Text {
                x: 215,
                y: 135,
                bytes: ranges_2,
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f902_ranges_are_the_numerator_over_the_mass() {
        // A 25 t hull with a 20 t hold: class 1 (needs more than 6 t) is
        // 30,000 / 25 = 1,200 (12.00 ly, the Eagle's status line), class 2
        // (needs more than 12 t) 4,800, class 3 (more than 24 t) not fitted.
        let ranges = ranges(25, 20);
        assert_eq!(&ranges[..3], &[1200, 4800, 0]);
        assert_eq!(ranges[6], 0);
        // A zero mass divides by one; a range under 300 shows as 0.
        assert_eq!(super::ranges(0, 7)[0], 30_000);
        assert_eq!(super::ranges(1000, 500)[0], 0);
        assert_eq!(super::ranges(100, 500)[0], 300);
    }

    #[test]
    fn the_templates_expand_like_f1397() {
        let name = b"Class 1 Hyperdrive".to_vec();
        let lookup = |code: u16| (code == 0x9000).then(|| name.clone());
        let text = system_text::expand(STATS, &[28, 23, 2, 2, 1], &lookup);
        assert_eq!(
            text,
            b"Mass (fully laden)\xfd{28t\rInternal Capacity (no drive)\xfd{23t\rGun Mountings\xfd{2\rMissile Pylons\xfd{2\rCrew\xfd{1"
                .to_vec()
        );
        let motion = system_text::expand(
            b"\xfd{\xff0\x04 \xff0\x1b",
            &[252, 0, 0, 0x9000, 0],
            &lookup,
        );
        assert_eq!(motion, b"\xfd{25.2 Class 1 Hyperdrive".to_vec());
    }
}
