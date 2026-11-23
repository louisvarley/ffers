//! F76's `Sy`/`Ci` system picks (`docs/notes/journal.md`): the generated-system
//! search F858, the argument filters (`DATA_007107`'s allegiance types and the
//! fixed lists `DATA_002289`/`DATA_002291`/`DATA_002290`) and the object names
//! `Ci` takes from F461/F462/F463 over F878's generated list.
//!
//! F76's two forms after F75's seed are:
//!
//! * a digit or `-`: F74's number selects the F858 mode (0 keeps the sector,
//!   1 and 2 step one or two sectors); F76 tries five times and falls back to
//!   the current system;
//! * otherwise the parenthesized argument: `Imp riot`, `F edge`, `Imp edge`
//!   pick from the fixed code lists, `A`, `F`, `C`, `I`, `Impmine`/`impmine`
//!   and `disput` pick a galaxy-table system by the record byte `+0xa & 0xf`,
//!   and an unknown argument keeps the current system.
//!
//! `Sy` names the system through F860. `Ci` is F461/F462/F463: generate the
//! system's objects with F878, count those whose `+0xa` kind is 0x20 or over,
//! and name one of them. The DOS original compares the fixed argument words
//! against the argument buffer; JJFFE 2.8a7's decompilation resolves those
//! three compares to the two-letter code instead, which can never match. The
//! DOS behaviour is ported here (`[SyL1(Imp riot)` and `[SyG1(F edge)` occur in
//! the imported archive).

use crate::galaxy;
use crate::names::Names;
use crate::sector;
use crate::sysgen;

/// `DATA_002289`: the systems F76's `Imp riot` argument picks from.
pub const IMPERIAL_RIOT: [u32; 6] = [
    0x23fe_e002,
    0x13fe_8002,
    0x33ff_7fff,
    0x2000_5ffa,
    0x1bff_5ffa,
    0x2fff_9ffd,
];

/// `DATA_002290`: the systems F76's `Imp edge` argument picks from.
pub const IMPERIAL_EDGE: [u32; 5] = [
    0x1fff_6000,
    0x03ff_6000,
    0x1bff_bff8,
    0x17ff_6001,
    0x13ff_7fff,
];

/// `DATA_002291`: the systems F76's `F edge` argument picks from.
pub const FEDERAL_EDGE: [u32; 8] = [
    0x1fff_5ff5,
    0x0400_5ff8,
    0x1ffe_fffa,
    0x17fe_dffe,
    0x2bfe_e009,
    0x2801_6002,
    0x0ffe_8002,
    0x23ff_c002,
];

/// F76's `Sy`/`Ci` system: `mode` is `Some(n)` for the F858 form (F74's
/// number after F75's seed, absent from the imported archive), `None` for the
/// `(argument)` form. Falls back to `current` as the source does.
pub fn resolve(
    sector: &sector::Tables<'_>,
    systems: &galaxy::Tables<'_>,
    current: u32,
    seed: i32,
    mode: Option<i32>,
    argument: &[u8],
) -> u32 {
    if let Some(mode) = mode {
        return sector.search_generated(current, mode, seed);
    }
    let (mut a, mut b) = (seed as u32, seed as u32);
    let fixed = match argument {
        b"Imp riot" => Some(&IMPERIAL_RIOT[..]),
        b"Imp edge" => Some(&IMPERIAL_EDGE[..]),
        b"F edge" => Some(&FEDERAL_EDGE[..]),
        _ => None,
    };
    if let Some(list) = fixed {
        return list[sector::f921(list.len() as u32, &mut a, &mut b) as usize];
    }
    let class = match argument {
        b"A" => 1,
        b"C" => 4,
        b"F" => 5,
        b"I" => 0xb,
        b"Impmine" | b"impmine" => 0xc,
        b"disput" => 0xe,
        _ => return current,
    };
    let choices: Vec<u32> = (0..systems.records())
        .filter(|index| (systems.allegiance(*index) as u8) & 0xf == class)
        .map(|index| systems.code(index))
        .collect();
    if choices.is_empty() {
        // F76 would read the table's first record here; a real table always has
        // systems of every class, so keep the current system instead.
        return current;
    }
    choices[sector::f921(choices.len() as u32, &mut a, &mut b) as usize]
}

/// F860: system `code`'s name from the system string table.
pub fn system_name(
    sector: &sector::Tables<'_>,
    strings: &sector::Strings<'_>,
    code: u32,
) -> Vec<u8> {
    sector.system_name(strings, code).unwrap_or_default()
}

/// F76's `Ci`: F461/F462/F463 over the F878 list of system `code`. The pick is
/// an object whose `+0xa` kind is 0x20 or over (the starports and stations);
/// its name is the object's `+0x26` text. Empty when the system cannot be
/// generated or has no such object, as the source emits nothing then.
pub fn object_name<'a>(
    sector: &'a sector::Tables<'a>,
    systems: &'a galaxy::Tables<'a>,
    strings: &'a sector::Strings<'a>,
    names: Option<&'a Names<'a>>,
    seed: i32,
    code: u32,
) -> Vec<u8> {
    picked_name(sector, systems, strings, names, seed, code, false)
}

/// F76's `Pl`: the same pick as [`object_name`], then the name of its parent object, the
/// list entry `+8` of the picked record numbers from 1 (a starport's planet). The executed original
/// names `Gupta's Rock` where `Ci` names its starport `Sarajevo`.
pub fn place_name<'a>(
    sector: &'a sector::Tables<'a>,
    systems: &'a galaxy::Tables<'a>,
    strings: &'a sector::Strings<'a>,
    names: Option<&'a Names<'a>>,
    seed: i32,
    code: u32,
) -> Vec<u8> {
    picked_name(sector, systems, strings, names, seed, code, true)
}

fn picked_name<'a>(
    sector: &'a sector::Tables<'a>,
    systems: &'a galaxy::Tables<'a>,
    strings: &'a sector::Strings<'a>,
    names: Option<&'a Names<'a>>,
    seed: i32,
    code: u32,
    parent: bool,
) -> Vec<u8> {
    let Some(params) = sysgen::params_for(code, sector, systems, strings) else {
        return Vec::new();
    };
    let mut generator = sysgen::Generator::new(*systems);
    if let Some(names) = names {
        generator.set_names(names);
    }
    if generator.generate(&params).is_err() {
        return Vec::new();
    }
    let objects = generator.count as usize;
    let choices: Vec<usize> = (0..objects)
        .filter(|object| {
            let at = object * sysgen::OBJECT_BYTES + 0xa;
            u16::from_le_bytes([generator.list[at], generator.list[at + 1]]) >= 0x20
        })
        .collect();
    if choices.is_empty() {
        return Vec::new();
    }
    let (mut a, mut b) = (seed as u32, seed as u32);
    let pick = choices[sector::f921(choices.len() as u32, &mut a, &mut b) as usize];
    let object = if parent {
        let at = pick * sysgen::OBJECT_BYTES + 8;
        // The word is 1-based: F76 adds it times 0x44 to the list's address less 0x44.
        usize::from(u16::from_le_bytes([
            generator.list[at],
            generator.list[at + 1],
        ]))
        .saturating_sub(1)
    } else {
        pick
    };
    generator.name_of(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A galaxy block with 329 records, record `index`'s code `index + 1` and
    /// its allegiance nibble `index % 16`.
    fn galaxy_block() -> Vec<u8> {
        let mut bytes = vec![0u8; galaxy::Tables::block_bytes(galaxy::RECORDS)];
        for index in 0..galaxy::RECORDS {
            let at = galaxy::GALAXY_OFFSET + index * galaxy::RECORD_BYTES;
            bytes[at..at + 4].copy_from_slice(&(index as u32 + 1).to_le_bytes());
            bytes[at + 10] = (index % 0x10) as u8;
        }
        bytes
    }

    /// A sector block: the special sector `(0x10, 0x20)` has three stars, so
    /// F853 reports three there and none anywhere else (the density grid is
    /// zero).
    fn sector_block() -> Vec<u8> {
        const SPECIAL: usize = 936;
        let mut bytes = vec![0u8; sector::TABLE_BYTES];
        for index in 0..sector::SPECIAL_SECTORS {
            let key = if index == 0 {
                0x10u32 | 0x20 << 16
            } else {
                (0x100 + index as u32) | ((0x200 + index as u32) << 16)
            };
            bytes[SPECIAL + 4 * index..SPECIAL + 4 * index + 4].copy_from_slice(&key.to_le_bytes());
        }
        // The index table follows the 49 keys (`sector::INDEX` is private).
        let index = SPECIAL + 196;
        let starts: [u32; 50] = std::array::from_fn(|k| match k {
            0 => 0,
            1 => 3,
            49 => 117,
            _ => 3,
        });
        for (k, value) in starts.iter().enumerate() {
            bytes[index + 4 * k..index + 4 * k + 4].copy_from_slice(&value.to_le_bytes());
        }
        for vector in 0..117 {
            bytes[6 * vector] = 9;
        }
        bytes
    }

    /// `0x4000_0000` makes the first draw `0x8000`, so `range` draws give the
    /// same fraction of `range`.
    const SEED: i32 = 0x4000_0000;

    #[test]
    fn f858_picks_another_star_of_the_sector() {
        let bytes = sector_block();
        let sector = sector::Tables::new(&bytes).unwrap();
        // Three stars; the seed takes star 1 of the sector.
        let current = 0x10 | 0x20 << 13;
        let (mut a, mut b) = (SEED as u32, SEED as u32);
        let value = sector.generated_system(current, 0, &mut a, &mut b);
        assert_eq!(value & 0x1fff, 0x10);
        assert_eq!((value >> 13) & 0x1fff, 0x20);
        assert_eq!((value >> 26) & 0x3f, 1);
        // F76's five tries give the same star, and an empty sector falls back
        // to the current system.
        assert_eq!(sector.search_generated(current, 0, SEED), value);
        let empty = 0x400 | 0x300 << 13;
        assert_eq!(sector.search_generated(empty, 0, SEED), empty);
        let (mut a, mut b) = (SEED as u32, SEED as u32);
        assert_eq!(sector.generated_system(empty, 0, &mut a, &mut b), 0);
        // Modes outside 0..2 are rejected.
        assert_eq!(sector.search_generated(current, 3, SEED), current);
    }

    #[test]
    fn the_arguments_pick_the_sources_systems() {
        let sector_bytes = sector_block();
        let sector = sector::Tables::new(&sector_bytes).unwrap();
        let bytes = galaxy_block();
        let systems = galaxy::Tables::new(&bytes).unwrap();
        let current = 0x1234;
        // 20 records have nibble 0xb; the seed chooses index 10 (record 171,
        // whose code is 172).
        let chosen = resolve(&sector, &systems, current, SEED, None, b"I");
        assert_eq!(chosen, 172);
        assert_eq!(systems.allegiance(chosen as usize - 1) as u8 & 0xf, 0xb);
        assert_eq!(
            resolve(&sector, &systems, current, SEED, None, b"F edge"),
            FEDERAL_EDGE[4]
        );
        assert_eq!(
            resolve(&sector, &systems, current, SEED, None, b"Imp riot"),
            IMPERIAL_RIOT[3]
        );
        // An argument the source does not know keeps the current system.
        assert_eq!(
            resolve(&sector, &systems, current, SEED, None, b"A edge"),
            current
        );
    }
}
