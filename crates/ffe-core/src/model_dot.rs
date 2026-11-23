//! The dots a point command draws (F1699, F1700, F1701 and the particle rasteriser F1610/F1611).
//!
//! A point (opcode 1) is a sphere seen from afar: its projected radius decides whether it is a
//! single pixel, a small two-tone dot (F1801: a halo particle and a core particle) or one larger
//! filled circle (F1800). Only the unlit materials (`0x2000`, which stars use) are ported here;
//! a lit point is drawn as a shaded sphere by polygons, which is not.

use crate::model_commands::material;
use crate::model_math::table_root;
use crate::model_thruster::{base_colour, special_divide};

/// A horizontal run of pixels of one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub x: i32,
    pub y: i32,
    pub width: i32,
}

/// The view's size: the particle code clips to 320 by 158 (`0x9e`).
const WIDTH: i32 = 320;
const HEIGHT: i32 = 0x9e;

/// F1600/F1601: a run at row `y`, clipped to the width (F1600) or not (F1601).
fn run(spans: &mut Vec<Span>, x: i32, y: i32, width: i32, clip: bool) {
    let (mut x, mut width) = (x, width);
    if clip {
        if x < 0 {
            if width + x <= 0 {
                return;
            }
            width += x;
            x = 0;
        }
        if x + width > WIDTH {
            width = WIDTH - x;
            if width <= 0 {
                return;
            }
        }
    }
    if width > 0 {
        spans.push(Span { x, y, width });
    }
}

/// F1611: a roughly circular particle of `size` (a diameter) centred on (x, y), as the runs it
/// draws in order. Sizes of 948 or more, and centres outside x 2..=317, y 2..=155 for the six
/// hand-drawn sizes, draw nothing.
pub fn particle(x: i32, y: i32, size: i32) -> Vec<Span> {
    let mut spans = Vec::new();
    if !(0..0x3b4).contains(&size) {
        return spans;
    }
    if size < 6 {
        if !(2..=0x13d).contains(&x) || !(2..=0x9b).contains(&y) {
            return spans;
        }
        let rows: &[(i32, i32, i32)] = match size {
            0 => &[(0, 0, 1)],
            1 => &[(-1, -1, 2), (-1, 0, 2)],
            2 => &[(0, -1, 1), (-1, 0, 3), (0, 1, 1)],
            3 => &[(-1, -1, 3), (-1, 0, 3), (-1, 1, 3)],
            4 => &[(-1, -2, 2), (-2, -1, 4), (-2, 0, 4), (-1, 1, 2)],
            _ => &[(0, -2, 1), (-1, -1, 3), (-2, 0, 5), (-1, 1, 3), (0, 2, 1)],
        };
        for (dx, dy, width) in rows {
            run(&mut spans, x + dx, y + dy, *width, false);
        }
        return spans;
    }
    let r = size / 2;
    let mut rows = vec![(0, 0); 2 * r as usize + 2];
    let mut d = size - 2;
    if d == 5 {
        d += 3;
    }
    let mut d = -d + 6;
    let mut b = r;
    let (mut top, mut p18, mut p1c, mut p14) = (r, r, 2 * r, 0);
    let mut i = 1;
    if b > i {
        loop {
            top -= 1;
            rows[top as usize] = (x - b, 2 * b);
            rows[p18 as usize] = (x - b, 2 * b);
            p18 += 1;
            if d < 0 {
                d += i * 4 + 6;
            } else {
                p1c -= 1;
                rows[p1c as usize] = (x - i, 2 * i);
                rows[p14 as usize] = (x - i, 2 * i);
                p14 += 1;
                d += (i - b) * 4 + 10;
                b -= 1;
            }
            i += 1;
            if b <= i {
                break;
            }
        }
    }
    if b == i {
        top -= 1;
        rows[top as usize] = (x - i, 2 * i);
        rows[p18 as usize] = (x - i, 2 * i);
    }
    let mut at = 0usize;
    let mut row = y - r;
    if row < 0 {
        at = (-row) as usize;
        row = 0;
    }
    let end = (y + r).min(HEIGHT);
    let clip = x - r < 0 || x + r >= WIDTH;
    while row < end {
        let (left, width) = rows.get(at).copied().unwrap_or((0, 0));
        run(&mut spans, left, row, width, clip);
        at += 1;
        row += 1;
    }
    spans
}

/// F1610: [`particle`] unless the whole particle is off the 320 by 158 view (a half-size
/// margin, rounded towards zero, is allowed).
pub fn particle_clipped(x: i16, y: i16, size: i32) -> Vec<Span> {
    let half = size / 2;
    let outside = |value: i16, limit: i32| {
        let value = i32::from(value);
        if (0..limit).contains(&value) {
            false
        } else if value < 0 {
            value + half <= 0
        } else {
            value - half >= limit
        }
    };
    if outside(x, WIDTH) || outside(y, HEIGHT) {
        return Vec::new();
    }
    particle(i32::from(x), i32::from(y), size)
}

/// The size tables F1801 reads: `DATA_007835` (the core's size) and `DATA_007836` (the halo's),
/// by the dot's size minus one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DotTables {
    pub core: [u8; 9],
    pub halo: [u8; 9],
}

pub const TABLE_BYTES: usize = 18;

impl DotTables {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() != TABLE_BYTES {
            return Err(format!(
                "The dot tables must be {TABLE_BYTES} bytes, not {}.",
                bytes.len()
            ));
        }
        let mut core = [0; 9];
        let mut halo = [0; 9];
        core.copy_from_slice(&bytes[..9]);
        halo.copy_from_slice(&bytes[9..]);
        if core.iter().chain(&halo).any(|size| *size > 16) {
            return Err("A dot size is out of range.".to_owned());
        }
        Ok(Self { core, halo })
    }
}

/// The first sprite record index: F1700 maps a sub-pixel object to a size from -7 to 0, and
/// F1609 reads the record at `DATA_007784 + 18 * size` (so seven records lie before the label).
pub const SPRITE_FIRST: i32 = -7;
/// Records from [`SPRITE_FIRST`] to 13.
pub const SPRITE_RECORDS: usize = 21;
/// Bytes per record: the halo's nine (`DATA_007784` side), then the core's nine (`DATA_007785`).
const SPRITE_RECORD: usize = 18;
/// `DATA_007825`: F1700's map from a sub-pixel size (`size >> 3`) to a sprite record.
pub const SIZE_MAP_BYTES: usize = 33;
/// Bytes of the sprites and the size map, as the importer stores them in that order.
pub const OBJECT_TABLE_BYTES: usize = SPRITE_RECORDS * SPRITE_RECORD + SIZE_MAP_BYTES;
/// Bytes per model in the descriptor records that follow: far class, dot class, then the
/// descriptor's three colour bytes (`+0x24`..`+0x26`), padded.
pub const OBJECT_MODEL_BYTES: usize = 8;

/// What F1680 reads from a model descriptor to draw an object as a dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectModel {
    /// `+0x20`: the distance class at which the object is no longer drawn.
    pub far: i32,
    /// `+0x28`: the distance class from which it is drawn as a dot.
    pub dot: i32,
    /// `+0x24`..`+0x26` masked to nibbles: the dot's colour.
    pub colour: [u8; 3],
    /// Bit 7 of `+0x24`: the object is its own light (a star), so no shade is added.
    pub unlit: bool,
}

/// The tables the dots of far objects use (F1700 with F1680's object flag): the sprite records
/// F1609 draws, the size map, and every model's [`ObjectModel`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDotTables {
    sprites: [[i8; SPRITE_RECORD]; SPRITE_RECORDS],
    size_map: [i8; SIZE_MAP_BYTES],
    models: Vec<[u8; OBJECT_MODEL_BYTES]>,
}

impl ObjectDotTables {
    pub fn new(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < OBJECT_TABLE_BYTES
            || (bytes.len() - OBJECT_TABLE_BYTES) % OBJECT_MODEL_BYTES != 0
        {
            return Err(format!(
                "The object dot tables are {} bytes, not {OBJECT_TABLE_BYTES} plus whole model records.",
                bytes.len()
            ));
        }
        let mut sprites = [[0i8; SPRITE_RECORD]; SPRITE_RECORDS];
        for (record, chunk) in sprites.iter_mut().zip(bytes.chunks_exact(SPRITE_RECORD)) {
            for (out, byte) in record.iter_mut().zip(chunk) {
                *out = *byte as i8;
            }
            // Each half is a centre half-width and then half-widths until a negative byte.
            for half in record.chunks_exact(SPRITE_RECORD / 2) {
                if !half[1..].iter().any(|byte| *byte < 0) {
                    return Err("A dot sprite has no end marker.".to_owned());
                }
            }
        }
        let mut size_map = [0i8; SIZE_MAP_BYTES];
        let map = &bytes[SPRITE_RECORDS * SPRITE_RECORD..OBJECT_TABLE_BYTES];
        for (out, byte) in size_map.iter_mut().zip(map) {
            *out = *byte as i8;
            if !(SPRITE_FIRST..=0).contains(&i32::from(*out)) {
                return Err("A dot size map entry is out of range.".to_owned());
            }
        }
        let models = bytes[OBJECT_TABLE_BYTES..]
            .chunks_exact(OBJECT_MODEL_BYTES)
            .map(|record| record.try_into().unwrap())
            .collect();
        Ok(Self {
            sprites,
            size_map,
            models,
        })
    }

    /// The model's descriptor fields; `None` when it has no descriptor (far class 0).
    pub fn model(&self, index: u32) -> Option<ObjectModel> {
        let record = self.models.get(index as usize)?;
        (record[0] != 0).then(|| ObjectModel {
            far: i32::from(record[0]),
            dot: i32::from(record[1]),
            colour: [record[2] & 0xf, record[3] & 0xf, record[4] & 0xf],
            unlit: record[2] & 0x80 != 0,
        })
    }

    /// One half of the sprite record `size`: the halo's (`core` false) or the core's.
    fn sprite(&self, size: i32, core: bool) -> Option<&[i8]> {
        let record = self
            .sprites
            .get(usize::try_from(size - SPRITE_FIRST).ok()?)?;
        let half = SPRITE_RECORD / 2;
        Some(if core {
            &record[half..]
        } else {
            &record[..half]
        })
    }
}

/// F1700's dot for an object F1680 sends it (its object flag set).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectDot {
    /// One particle (F1701), for a dot under a pixel across or over nine.
    Particle(Dot),
    /// F1802: a halo sprite, then a core sprite over it, of record `size`.
    Sprites {
        x: i16,
        y: i16,
        size: i32,
        halo: [u8; 3],
        core: [u8; 3],
    },
}

impl ObjectDot {
    /// The runs it draws, in order, each with its colour.
    pub fn spans(&self, tables: &ObjectDotTables) -> Vec<(Span, [u8; 3])> {
        match *self {
            ObjectDot::Particle(dot) => dot.spans(),
            ObjectDot::Sprites {
                x,
                y,
                size,
                halo,
                core,
            } => [(false, halo), (true, core)]
                .into_iter()
                .flat_map(|(is_core, colour)| {
                    tables
                        .sprite(size, is_core)
                        .map(|data| sprite(i32::from(x), i32::from(y), data))
                        .unwrap_or_default()
                        .into_iter()
                        .map(move |span| (span, colour))
                })
                .collect(),
        }
    }
}

/// F1700 for an object F1680 draws as a dot. `screen` is its projected centre, `size` F1680's
/// `SpecialDiv(radius, depth)` (the projected radius in pixels), `radius` and `depth` the
/// operands of that division, `colour` the descriptor's nibbles and `shade` the instance's
/// first shade (`+0x84`), which a lit object adds. `Ok(None)` when nothing reaches the view.
/// A lit object nine pixels or more across is a shaded sphere in the original, which is not
/// ported (`Err`).
#[allow(clippy::too_many_arguments)]
pub fn object_dot(
    screen: [i16; 2],
    size: i32,
    radius: i32,
    depth: i32,
    colour: [u8; 3],
    unlit: bool,
    shade: [u8; 3],
    tables: &ObjectDotTables,
    sky: [u8; 3],
) -> Result<Option<ObjectDot>, String> {
    // F1700's clip: the dot must reach the view.
    let half = size / 2;
    let reaches = |value: i32, limit: i32| {
        if (0..limit).contains(&value) {
            true
        } else if value < 0 {
            (0..limit).contains(&(value + half))
        } else {
            (0..limit).contains(&(value - half))
        }
    };
    let [x, y] = screen.map(i32::from);
    if !reaches(x, WIDTH) || !reaches(y, HEIGHT) {
        return Ok(None);
    }
    let mut colour = colour;
    if !unlit {
        if size >= 9 {
            return Err(
                "A lit object nine pixels across is a shaded sphere, which is not ported."
                    .to_owned(),
            );
        }
        colour = std::array::from_fn(|k| colour[k].wrapping_add(shade[k]));
    }
    let mut size = size;
    if size == 0 {
        // The object flag recomputes a finer size, in 1/256 of a pixel.
        let fine = crate::model_thruster::special_divide(radius, depth >> 8)?;
        if fine == 0 {
            return Ok(Some(ObjectDot::Particle(Dot {
                x: screen[0],
                y: screen[1],
                shape: DotShape::One { size: 0, colour },
            })));
        }
        let index = usize::try_from(fine >> 3).map_err(|_| "A negative dot size.".to_owned())?;
        size = tables
            .size_map
            .get(index)
            .map(|entry| i32::from(*entry))
            .ok_or_else(|| "A dot size is past the size map.".to_owned())?;
    } else if size > 9 {
        return Ok(Some(ObjectDot::Particle(Dot {
            x: screen[0],
            y: screen[1],
            shape: DotShape::One { size, colour },
        })));
    }
    if !(0..WIDTH).contains(&x) || !(0..HEIGHT).contains(&y) {
        return Ok(None);
    }
    // F1802's halo is the colour mixed a third towards the sky.
    let halo =
        std::array::from_fn(|k| ((u32::from(colour[k]) * 2 + u32::from(sky[k]) + 1) / 3) as u8);
    Ok(Some(ObjectDot::Sprites {
        x: screen[0] - 1,
        y: screen[1] - 1,
        size,
        halo,
        core: colour,
    }))
}

/// One dot as F1700 queues it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dot {
    /// The screen centre.
    pub x: i16,
    pub y: i16,
    pub shape: DotShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotShape {
    /// F1800: one particle of `size`.
    One { size: i32, colour: [u8; 3] },
    /// F1801: a halo particle, then a core particle over it.
    Two {
        halo_size: i32,
        halo: [u8; 3],
        core_size: i32,
        core: [u8; 3],
    },
}

impl Dot {
    /// The runs the dot draws, in order, each with the colour it takes.
    pub fn spans(&self) -> Vec<(Span, [u8; 3])> {
        let (x, y) = (self.x, self.y);
        match self.shape {
            DotShape::One { size, colour } => particle_clipped(x, y, size)
                .into_iter()
                .map(|span| (span, colour))
                .collect(),
            DotShape::Two {
                halo_size,
                halo,
                core_size,
                core,
            } => particle_clipped(x, y, halo_size)
                .into_iter()
                .map(|span| (span, halo))
                .chain(
                    particle_clipped(x, y, core_size)
                        .into_iter()
                        .map(|span| (span, core)),
                )
                .collect(),
        }
    }
}

/// What F1700 needs about a point to draw it as a dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointView {
    /// The projected centre (the slot's screen words).
    pub screen: [i16; 2],
    /// The point's view-space x and y (`+4` and `+8` of the slot).
    pub view: [i32; 2],
    /// The point's view-space depth (`+0xc` of the slot).
    pub depth: i32,
    /// The point's radius in view units (F1474's word shifted by the scale less 8).
    pub radius: i32,
    /// F1691's colour.
    pub colour: [u8; 3],
}

/// F1699's far path then F1700 for an unlit point: the dot, or `None` when it is off the view or
/// too near to draw (a depth under 0x40). A point whose radius is more than an eighth of its
/// depth is drawn by F1699's near path (an ellipse), which is not ported: `Err`.
/// `sky` is the ambient colour the halo is mixed with (`DATA_008815`).
pub fn unlit_dot(
    point: PointView,
    tables: &DotTables,
    sky: [u8; 3],
) -> Result<Option<Dot>, String> {
    let PointView {
        mut screen,
        view,
        depth,
        radius,
        colour,
    } = point;
    if depth < 0x40 {
        return Ok(None);
    }
    let size = if (radius as u32).wrapping_shl(3) > depth as u32 {
        // F1699's near path: the point's extent is a good part of its depth, so it is placed
        // and sized as an ellipse's centre from the exact projection. Nothing is drawn when the
        // radius reaches the depth.
        if radius as u32 >= depth as u32 {
            return Ok(None);
        }
        let mut bits = depth | radius | view[0].wrapping_abs() | view[1].wrapping_abs();
        let mut k = 0u32;
        for (limit, step) in [(0x400_0000, 16), (0x4_0000, 8), (0x4000, 4), (0x1000, 2)] {
            if bits >= limit {
                k += step;
                bits >>= step;
            }
        }
        if bits >= 0x800 {
            k += 1;
        }
        let reduced = (radius as u32 >> k) as i32;
        let depth_k = depth >> k;
        let gap = depth_k
            .wrapping_mul(depth_k)
            .wrapping_sub(reduced.wrapping_mul(reduced));
        if gap == 0 {
            return Ok(None);
        }
        let (x, y) = (view[0] >> k, view[1] >> k);
        let across = |value: i32| depth_k.wrapping_mul(value).wrapping_shl(8) / gap;
        screen = [
            (across(x) as i16).wrapping_add(0xa0),
            0x4f_i16.wrapping_sub(across(y) as i16),
        ];
        let root = table_root(
            x.wrapping_mul(x)
                .wrapping_add(y.wrapping_mul(y))
                .wrapping_add(gap) as u32,
        );
        let product = (root as i32).wrapping_mul(reduced.wrapping_shl(8)) as u32;
        let size = (product / gap as u32) as i32;
        if size < 0 {
            return Err("A negative dot size is not ported.".to_owned());
        }
        size
    } else {
        special_divide(radius, depth)?
    };
    // F1700's clip: the dot must reach the view.
    let [x, y] = screen.map(i32::from);
    let half = size / 2;
    let reaches = |value: i32, limit: i32| {
        if (0..limit).contains(&value) {
            true
        } else if value < 0 {
            let moved = value + half;
            (0..limit).contains(&moved)
        } else {
            let moved = value - half;
            (0..limit).contains(&moved)
        }
    };
    if !reaches(x, 0x140) || !reaches(y, HEIGHT) {
        return Ok(None);
    }
    let (x, y) = (screen[0], screen[1]);
    if size == 0 || size > 9 {
        // F1701: one particle, in the colour F1691 gave.
        return Ok(Some(Dot {
            x,
            y,
            shape: DotShape::One { size, colour },
        }));
    }
    // F1801: a halo and a core; the halo is the colour mixed a third towards the sky.
    if !(0..0x140).contains(&i32::from(x)) || !(0..HEIGHT).contains(&i32::from(y)) {
        return Ok(None);
    }
    let index = (size - 1) as usize;
    let halo =
        std::array::from_fn(|k| ((u32::from(colour[k]) * 2 + u32::from(sky[k]) + 1) / 3) as u8);
    Ok(Some(Dot {
        x,
        y,
        shape: DotShape::Two {
            halo_size: i32::from(tables.halo[index]),
            halo,
            core_size: i32::from(tables.core[index]),
            core: colour,
        },
    }))
}

/// F1699's radius: the point's angle word (F1474) shifted by the instance's scale less 8
/// (logically right for a negative shift, as the original's `shr`).
pub fn point_radius(angle: i32, scale: i32) -> i32 {
    let shift = scale.wrapping_sub(8);
    if shift < 0 {
        ((angle as u32) >> (shift.wrapping_neg() as u32 & 31)) as i32
    } else {
        angle.wrapping_shl(shift as u32 & 31)
    }
}

/// A point command as a dot: its material (an unlit one), the F1474 angle value, the instance's
/// scale and colour, and where the point projects. Lit points are shaded spheres drawn by
/// polygons, which are not ported (`Err`).
#[allow(clippy::too_many_arguments)]
pub fn point_dot(
    material_word: u16,
    angle: i32,
    scale: i32,
    instance: [u8; 3],
    screen: [i16; 2],
    view: [i32; 2],
    depth: i32,
    tables: &DotTables,
    sky: [u8; 3],
) -> Result<Option<Dot>, String> {
    if !material(material_word).unlit {
        return Err("A lit point is a shaded sphere, which is not ported.".to_owned());
    }
    unlit_dot(
        PointView {
            screen,
            view,
            depth,
            radius: point_radius(angle, scale),
            colour: base_colour(material_word, instance),
        },
        tables,
        sky,
    )
}

/// F1612 (`ffe.asm:96457`): the circle of diameter `size` about (`x`, `y`) as
/// the rows it writes. It halves `size` to a radius `r`, runs a midpoint-style
/// loop (the decision value starts at `8 - size`, or `-2` for a size of 7, and
/// steps by `4 * y + 6` or `4 * (y - x) + 10`) that fills a table of `2 * r`
/// `(left, width)` rows from the top, the bottom and the middle outward, then
/// draws rows `y - r` to `y + r - 1`, cut to the view's `0x9e` rows only. The
/// first row of the table and the last row drawn are whole runs (F1601); every
/// other row is its two end pixels. A size of 7 starts the decision value at
/// `-2` instead of `1`, and a size below 2 draws nothing.
pub fn circle(x: i32, y: i32, size: i32) -> Vec<Span> {
    let r = size / 2;
    let mut spans = Vec::new();
    if r <= 0 {
        return spans;
    }
    let r = r as usize;
    let mut decision = -(size - 2 + if size - 2 == 5 { 3 } else { 0 }) + 6;
    let mut table = vec![(0i32, 0i32); 2 * r];
    // The four pointers: the top rows grow down from the start, the bottom rows up
    // from the end, and the middle grows both ways from row `r`.
    let (mut top, mut below, mut upper, mut bottom) = (0usize, r, r, 2 * r);
    let (mut across, mut down) = (r as i32, 1i32);
    while across > down {
        upper -= 1;
        table[upper] = (x - across, 2 * across);
        table[below] = (x - across, 2 * across);
        below += 1;
        if decision < 0 {
            decision += 4 * down + 6;
        } else {
            bottom -= 1;
            table[bottom] = (x - down, 2 * down);
            table[top] = (x - down, 2 * down);
            top += 1;
            decision += 4 * (down - across) + 10;
            across -= 1;
        }
        down += 1;
    }
    if across == down {
        upper -= 1;
        table[upper] = (x - down, 2 * down);
        table[below] = (x - down, 2 * down);
    }
    let (mut row, mut index) = (y - r as i32, 0usize);
    if row < 0 {
        index = (-row) as usize;
        row = 0;
    }
    let end = (y + r as i32).min(HEIGHT);
    while end > row {
        let Some(&(left, width)) = table.get(index) else {
            break;
        };
        if end - row - 1 == 0 || index == 0 {
            run(&mut spans, left, row, width, true);
        } else {
            run(&mut spans, left, row, 1, true);
            run(&mut spans, left + width - 1, row, 1, true);
        }
        index += 1;
        row += 1;
    }
    spans
}

/// F1609 (`ffe.asm:95877`): a sprite from a record of signed half-widths,
/// vertically symmetric about `y`. The original picks the 18-byte record
/// `size` of `DATA_007784`, or of `DATA_007785` when its flag is set; this
/// takes the record's bytes. The x centre is one pixel right of `x` (the
/// routine's `inc`). `data[0]` is the centre row's half-width. Each later byte
/// is read first and then draws the pair of rows one further out above and
/// below; a byte of 0 moves out a row without drawing, and a negative byte ends
/// the sprite. Each row is a clipped run (F1600) of `2 * halfwidth - 1` pixels
/// from `x + 1 - halfwidth`, for rows inside the view (`y < 0x9e`, unsigned).
pub fn sprite(x: i32, y: i32, data: &[i8]) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut bytes = data.iter().map(|&byte| i32::from(byte));
    let Some(centre) = bytes.next() else {
        return spans;
    };
    let x = x + 1;
    let draw = |spans: &mut Vec<Span>, row: i32, half: i32| {
        if (row as u32) < HEIGHT as u32 {
            run(spans, x - half, row, 2 * half - 1, true);
        }
    };
    draw(&mut spans, y, centre);
    let (mut below, mut above) = (y, y);
    for half in bytes {
        if half < 0 {
            break;
        }
        below += 1;
        above -= 1;
        if half > 0 {
            draw(&mut spans, below, half);
            draw(&mut spans, above, half);
        }
    }
    spans
}
