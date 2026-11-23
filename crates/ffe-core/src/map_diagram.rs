//! The galaxy map's body diagram (F850/F851): the system's objects laid out as
//! small pictures across the top of the description, economy and people pages.
//!
//! F851 walks the 68-byte system objects from the list's start. Each object
//! draws the sprite its type selects from `DATA_006822` (width, height, the
//! offsets taken off the pen, and a primary bitmap id) at the pen, leaves a
//! square hot area `0x80 + index` for a click, then lays out the objects that
//! orbit it (a `parent` word of its index plus one): across for a star's
//! planets, down for a planet's moons. See `docs/notes/galaxy-map.md`.

use crate::sysgen::OBJECT_BYTES;

/// One `DATA_006822` record: width, height, x and y offsets, bitmap id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sprite {
    pub width: i32,
    pub height: i32,
    pub x_offset: i32,
    pub y_offset: i32,
    pub bitmap: u16,
}

const fn sprite(width: i32, height: i32, x_offset: i32, y_offset: i32, bitmap: u16) -> Sprite {
    Sprite {
        width,
        height,
        x_offset,
        y_offset,
        bitmap,
    }
}

/// `DATA_006822` (`ffedat.asm:14500`), 35 records of five dwords, by object type.
pub const SPRITES: [Sprite; 35] = [
    sprite(46, 46, 24, 11, 11),
    sprite(1, 1, 1, 0, 12),
    sprite(1, 1, 1, 1, 13),
    sprite(2, 2, 2, 2, 14),
    sprite(2, 2, 2, 2, 15),
    sprite(2, 2, 2, 2, 16),
    sprite(3, 3, 4, 3, 22),
    sprite(3, 3, 3, 3, 23),
    sprite(3, 3, 4, 3, 20),
    sprite(2, 3, 3, 2, 17),
    sprite(3, 3, 4, 3, 18),
    sprite(3, 3, 4, 3, 19),
    sprite(3, 3, 4, 3, 21),
    sprite(8, 8, 8, 7, 24),
    sprite(14, 14, 14, 13, 25),
    sprite(11, 11, 11, 11, 26),
    sprite(12, 12, 13, 12, 27),
    sprite(14, 14, 13, 13, 28),
    sprite(21, 21, 18, 15, 29),
    sprite(21, 21, 18, 15, 29),
    sprite(21, 21, 18, 15, 29),
    sprite(21, 21, 18, 16, 30),
    sprite(21, 21, 22, 20, 31),
    sprite(22, 22, 24, 20, 32),
    sprite(22, 22, 24, 20, 33),
    sprite(21, 21, 12, 12, 35),
    sprite(31, 31, 24, 35, 37),
    sprite(31, 31, 24, 35, 36),
    sprite(46, 46, 25, 35, 34),
    sprite(31, 31, 25, 35, 38),
    sprite(31, 31, 25, 35, 39),
    sprite(21, 21, 12, 12, 35),
    sprite(1, 1, 1, 1, 40),
    sprite(2, 2, 2, 2, 41),
    sprite(3, 3, 3, 3, 42),
];

/// The type F851 skips after a body (a marker object, `0x23`).
const MARKER: u16 = 0x23;
/// The view's height and width the hot areas must lie inside (F851's `0x9e`,
/// `0x140`).
const VIEW: (i32, i32) = (0x140, 0x9e);

/// One sprite F851 blits, and the hot area it leaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    /// The object's index in the list.
    pub index: usize,
    /// The sprite's bitmap id and the position `BlitIndex` is given (the pen
    /// less the record's offsets).
    pub bitmap: u16,
    pub x: i32,
    pub y: i32,
    /// The pen, which is the hot area's corner (`GuiAddHotSquare`'s x and y);
    /// `None` when the pen is outside the view.
    pub hot: Option<(i32, i32, i32)>,
}

/// One 68-byte object's fields F851 reads.
struct Reader<'a> {
    list: &'a [u8],
}

impl Reader<'_> {
    fn mass(&self, index: usize) -> u32 {
        let at = index * OBJECT_BYTES;
        self.list
            .get(at..at + 4)
            .map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn word(&self, index: usize, offset: usize) -> u16 {
        let at = index * OBJECT_BYTES + offset;
        self.list
            .get(at..at + 2)
            .map_or(0, |b| u16::from_le_bytes([b[0], b[1]]))
    }

    fn parent(&self, index: usize) -> u16 {
        self.word(index, 8)
    }

    fn kind(&self, index: usize) -> u16 {
        self.word(index, 0xa)
    }
}

/// F850: lay out the whole diagram of a generated system list.
pub fn layout(list: &[u8]) -> Vec<Placed> {
    let reader = Reader { list };
    let mut out = Vec::new();
    let mut index = 0;
    place(&reader, &mut index, 1, 0, 0x18, 4, true, &mut out);
    out
}

/// F851. `across` and `down` are the original's second and third arguments:
/// a child body's pen moves by `down` widths and `across` heights before it
/// draws (when `step` is set), and the pen moves on by the returned width
/// times `across` in x and `down` in y.
#[allow(clippy::too_many_arguments)]
fn place(
    reader: &Reader,
    index: &mut usize,
    across: i32,
    down: i32,
    mut x: i32,
    mut y: i32,
    step: bool,
    out: &mut Vec<Placed>,
) -> i32 {
    let first = *index;
    let Some(sprite) = SPRITES.get(usize::from(reader.kind(first))).copied() else {
        // A type past the table would read beyond it in the original; stop.
        *index = usize::MAX / 2;
        return 3;
    };
    if step {
        x += down * sprite.width;
        y += across * sprite.height;
    }
    let hot = (y < VIEW.1 && x < VIEW.0).then_some((x, y, sprite.width + 1));
    out.push(Placed {
        index: first,
        bitmap: sprite.bitmap,
        x: x - sprite.x_offset,
        y: y - sprite.y_offset,
        hot,
    });
    *index += 1;
    while reader.kind(*index) == MARKER && reader.mass(*index) != 0 {
        *index += 1;
    }
    // A star (type 0, the last object taken) is followed by the companions the
    // generator makes: one above and one below, not stepped.
    if reader.kind(*index - 1) == 0 {
        place(reader, index, 1, 0, x, y - 0x1d, false, out);
        place(reader, index, 1, 0, x, y + 0x1c, false, out);
    }
    x += (sprite.width + 3) * across;
    y += (sprite.height + 3) * down;
    // The bodies in orbit: planets across, then each planet's moons down.
    while reader.mass(*index) > 0 && usize::from(reader.parent(*index)) == first + 1 {
        let taken = place(reader, index, down, across, x, y, true, out);
        x += across * taken;
        y += down * taken;
    }
    sprite.width * 2 + 3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object(mass: u32, parent: u16, kind: u16) -> [u8; OBJECT_BYTES] {
        let mut bytes = [0u8; OBJECT_BYTES];
        bytes[..4].copy_from_slice(&mass.to_le_bytes());
        bytes[8..10].copy_from_slice(&parent.to_le_bytes());
        bytes[10..12].copy_from_slice(&kind.to_le_bytes());
        bytes
    }

    #[test]
    fn the_table_has_the_executables_records() {
        assert_eq!(SPRITES.len(), 35);
        assert_eq!(SPRITES[0], sprite(46, 46, 24, 11, 11));
        assert_eq!(SPRITES[34], sprite(3, 3, 3, 3, 42));
    }

    #[test]
    fn a_star_then_its_planets_run_across_the_top() {
        // Gateway's start of the list: a giant (type 26), two planets (13, 14)
        // whose parent word is 1, a moon (2) of the second (parent word 3).
        let mut list = Vec::new();
        list.extend(object(5, 0, 26));
        list.extend(object(5, 1, 13));
        list.extend(object(5, 1, 14));
        list.extend(object(5, 3, 2));
        list.extend(object(0, 0, 0));
        let placed = layout(&list);
        // The first call steps y by the star's height: pen (24, 4 + 31).
        assert_eq!(placed[0].hot, Some((24, 35, 32)));
        assert_eq!((placed[0].x, placed[0].y), (0, 0));
        // Planets follow to the right of the star, 3 pixels clear, each a width on.
        assert_eq!(placed[1].hot, Some((24 + 34 + 8, 35, 9)));
        assert_eq!(placed[2].hot.map(|h| h.1), Some(35));
        assert!(placed[2].hot.unwrap().0 > placed[1].hot.unwrap().0);
        // The moon stands below its planet: same x as the planet, lower.
        assert_eq!(placed[3].index, 3);
        assert!(placed[3].hot.unwrap().1 > 35);
    }
}
