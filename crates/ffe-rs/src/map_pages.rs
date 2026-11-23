//! The galaxy map's information pages (F842 and F843), built from the selected
//! star: the system's body diagram and the texts drawn over F956's cabin.
//! `Screen::draw_galaxy` draws a [`Page`]. Docs: `docs/notes/galaxy-map.md`.

use crate::galaxy_map::Selected;
use crate::model_viewer::ModelViewer;
use ffe_content::VerifiedCache;
use ffe_core::map_diagram::{self, Placed};
use ffe_core::message_strings::Messages;
use ffe_core::names::Names;
use ffe_core::sector::Tables as SectorTables;
use ffe_core::sysgen::Generator;
use ffe_core::system_text::expand;

/// F842's yellow and green text colours (palette entries 18 and 17).
pub const YELLOW: u8 = 18;
pub const GREEN: u8 = 17;
/// F846's colour for the sky's names.
pub const RED: u8 = 16;

/// One string the page writes: F1399's position, colour and wrap width, and the
/// expanded bytes (with the `0x0d`, `0xfd` and `0xfe` layout codes still in).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageText {
    pub x: i32,
    pub y: i32,
    pub colour: u8,
    /// The wrap width in pixels, 0 for none.
    pub wrap: i32,
    pub bytes: Vec<u8>,
}

/// A map page: the diagram's sprites and the texts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Page {
    pub sprites: Vec<Placed>,
    pub texts: Vec<PageText>,
    /// Picture mode's sky: 320 x 158 palette indices drawn over the cabin.
    pub background: Option<Vec<u8>>,
}

/// The raw string for an expansion code, as F1343 finds it: handler 2 (`0x84xx`)
/// is F831's system strings; the others are the message tables (the system
/// descriptions are handler 0, `0x80xx`, F895's `DATA_007117`).
fn string(viewer: &ModelViewer, messages: Option<&Messages>, selector: u16) -> Option<Vec<u8>> {
    if (selector >> 9) & 0x3f == 2 {
        viewer.system_string(u32::from(selector))
    } else {
        messages?.get(selector).map(<[u8]>::to_vec)
    }
}

/// What every page of a selected star starts from: the strings, the F878 list's
/// diagram and the F869/F870 reports.
struct Base<'a> {
    viewer: &'a ModelViewer,
    messages: Option<Messages<'a>>,
    tables: ffe_core::galaxy::Tables<'a>,
    params: ffe_core::sysgen::Params,
    bodies: u8,
    sprites: Vec<Placed>,
    /// The generated 68-byte objects and their names.
    list: Vec<u8>,
    names: Vec<Vec<u8>>,
}

impl<'a> Base<'a> {
    fn new(
        cache: &'a VerifiedCache,
        viewer: &'a ModelViewer,
        selected: &Selected,
    ) -> Result<Self, String> {
        let entry =
            crate::navigation::map_entry(cache, selected.code, selected.position, selected.kind)?;
        let blob = cache
            .asset("base:world/system-tables")
            .ok_or("The cache lacks the system tables.")?;
        let names = Names::new(
            cache
                .asset("base:text/names")
                .ok_or("The cache lacks the names.")?,
        )?;
        let mut generator = Generator::new(ffe_core::galaxy::Tables::new(blob)?);
        generator.set_names(&names);
        generator.generate(&entry.params)?;
        Ok(Self {
            viewer,
            messages: messages_of(cache),
            tables: ffe_core::galaxy::Tables::new(blob)?,
            params: entry.params,
            bodies: generator.major_bodies(),
            sprites: map_diagram::layout(&generator.list),
            names: (0..generator.count as usize)
                .map(|index| generator.name_of(index))
                .collect(),
            list: generator.list.clone(),
        })
    }

    fn string(&self, selector: u16) -> Option<Vec<u8>> {
        string(self.viewer, self.messages.as_ref(), selector)
    }

    fn expand(&self, selector: u16, vars: &[i32; 5]) -> Result<Vec<u8>, String> {
        let raw = self
            .string(selector)
            .ok_or_else(|| format!("The system strings lack {selector:#x}."))?;
        Ok(expand(&raw, vars, &|code| self.string(code)))
    }

    /// F842's and F843's test of `DATA_008962`: a system with none has no pages.
    fn explored(&self) -> bool {
        self.params.population != 0
    }

    fn text(&self, x: i32, y: i32, colour: u8, bytes: Vec<u8>) -> PageText {
        PageText {
            x,
            y,
            colour,
            wrap: 0,
            bytes,
        }
    }
}

fn unexplored(viewer: &ModelViewer, messages: Option<&Messages>) -> Result<Page, String> {
    let raw = string(viewer, messages, 0x84b3).ok_or("The system strings lack 0x84b3.")?;
    let lookup = |code: u16| string(viewer, messages, code);
    Ok(Page {
        sprites: Vec::new(),
        background: None,
        texts: vec![PageText {
            x: 10,
            y: 0x5f,
            colour: YELLOW,
            wrap: 0x12c,
            bytes: expand(&raw, &[0; 5], &lookup),
        }],
    })
}

fn messages_of(cache: &VerifiedCache) -> Option<Messages<'_>> {
    Messages::new(cache.asset("base:text/messages")?).ok()
}

/// F842: the system's description. With a selected, explored star it is the body
/// diagram (F850/F851 over the objects F878 makes for the star), the line
/// "Stable system with N major bodies." at (10,95) and the description (the
/// second dword F869 reports, a handler-0 message) wrapped to 300 pixels at
/// (10,105); otherwise the "Unexplored System" text at (10,95) over the cabin.
pub fn description(
    cache: &VerifiedCache,
    viewer: &ModelViewer,
    selected: Option<&Selected>,
) -> Result<Page, String> {
    let Some(selected) = selected else {
        return unexplored(viewer, messages_of(cache).as_ref());
    };
    let base = Base::new(cache, viewer, selected)?;
    if !base.explored() {
        return unexplored(viewer, base.messages.as_ref());
    }
    let vars = [i32::from(base.bodies), 0, 0, 0, 0];
    let mut page = Page {
        background: None,
        sprites: base.sprites.clone(),
        texts: vec![base.text(10, 0x5f, YELLOW, base.expand(0x84a0, &vars)?)],
    };
    let code = base.tables.details(selected.code).words[1] as u16;
    if base.string(code).is_some() {
        let mut text = base.text(10, 0x69, GREEN, base.expand(code, &vars)?);
        text.wrap = 0x12c;
        page.texts.push(text);
    }
    Ok(page)
}

/// F843's F9 page (`0xf9`): the commodities the system trades in five columns.
/// F850 draws the diagram, F854 the headings (`0x84b4`) at (2,59) in white, then
/// each commodity of F870's stock string by its flags: bit 7 illegal (x 256);
/// supply class 5 or more a major good and 3 or 4 a minor one (flag bit 3 makes
/// it an export: major imports x 2, minor imports 65, major exports 128, minor
/// exports 193), nine pixels apart from y 69 in yellow. `names` gives the
/// commodity names (F613's `0x8e00 + index`). `None` for a system with no
/// information, where the original draws nothing.
pub fn economy(
    cache: &VerifiedCache,
    viewer: &ModelViewer,
    selected: &Selected,
    names: &dyn Fn(usize) -> Option<String>,
) -> Result<Option<Page>, String> {
    let base = Base::new(cache, viewer, selected)?;
    if !base.explored() {
        return Ok(None);
    }
    let mut page = Page {
        background: None,
        sprites: base.sprites.clone(),
        texts: vec![base.text(2, 0x3b, 15, base.expand(0x84b4, &[0; 5])?)],
    };
    let market = base.tables.market(selected.code);
    // Rows used so far in the five columns: major and minor imports, major and
    // minor exports, illegal goods.
    let mut rows = [0i32; 5];
    for (index, flags) in market.stock.iter().take(33).enumerate() {
        let (column, x) = if flags & 0x80 != 0 {
            (4, 0x100)
        } else if flags & 7 >= 5 {
            if flags & 8 != 0 {
                (2, 0x80)
            } else {
                (0, 2)
            }
        } else if flags & 7 >= 3 {
            if flags & 8 != 0 {
                (3, 0xc1)
            } else {
                (1, 0x41)
            }
        } else {
            continue;
        };
        let Some(name) = names(index) else { continue };
        let y = 0x45 + 9 * rows[column];
        rows[column] += 1;
        page.texts.push(base.text(x, y, YELLOW, name.into_bytes()));
    }
    Ok(Some(page))
}

/// F843's F10 page (`0xfa`): the people. F869/F870 give the allegiance word (the
/// low six bits the government, the rest the allegiance) and the population
/// byte; `0x84b5` writes the government (`0x84cb +`), economy (`0x84bb +`),
/// allegiance (`0x84b7 +`) and population (`0x84db +`) at (40,99), and `0x84b6`
/// the sector code and the star's number at (40,139), in yellow.
pub fn people(
    cache: &VerifiedCache,
    viewer: &ModelViewer,
    selected: &Selected,
) -> Result<Option<Page>, String> {
    let base = Base::new(cache, viewer, selected)?;
    if !base.explored() {
        return Ok(None);
    }
    let details = base.tables.details(selected.code);
    let market = base.tables.market(selected.code);
    let word = details.allegiance as i32;
    let vars = [
        (word & 0x3f) + 0x84cb,
        (word & 0x3f) + 0x84bb,
        (word >> 6) + 0x84b7,
        market.population as i32 + 0x84db,
        0,
    ];
    let code = selected.code;
    let sector = [
        (code & 0x1fff) as i32 - 0x1718,
        ((code >> 13) & 0x1fff) as i32 - 0x1524,
        ((code >> 26) & 0x3f) as i32,
        0,
        0,
    ];
    Ok(Some(Page {
        background: None,
        sprites: base.sprites.clone(),
        texts: vec![
            base.text(0x28, 0x63, YELLOW, base.expand(0x84b5, &vars)?),
            base.text(0x28, 0x8b, YELLOW, base.expand(0x84b6, &sector)?),
        ],
    }))
}

/// The body of the diagram under logical pixel (`x`, `y`): F851's hot squares
/// (`GuiAddHotRect` about the pen, half-extent the sprite's width plus one), the
/// first in the order they were made. The result is the object's index (the
/// click's action is `0x80 +` it).
pub fn body_at(page: &Page, x: i32, y: i32) -> Option<usize> {
    page.sprites.iter().find_map(|placed| {
        let (cx, cy, half) = placed.hot?;
        let inside =
            (cx - half).max(0) <= x && x <= cx + half && (cy - half).max(0) <= y && y <= cy + half;
        inside.then_some(placed.index)
    })
}

/// F843's click on a body (`0x80 + index`): the diagram again with the object's
/// facts in yellow. A rock or planet (type 1 to 31) gets its mass in Earth
/// masses, surface temperature and the starports it carries (`0x84a1`, with the
/// starports at (82,125), "None" otherwise); an object with an orbit gets its
/// period and radius (`0x84a4`-`0x84a6` and `0x84a2`/`0x84a3`) at (82,135) and
/// (10,135); and its name and type (`0x8476 +` the type) go at (10,95), or at
/// (10,125) for a star pair or a station.
pub fn body_info(
    cache: &VerifiedCache,
    viewer: &ModelViewer,
    selected: &Selected,
    index: usize,
) -> Result<Option<Page>, String> {
    use ffe_core::short_float::{multiply, scale, to_integer};
    let base = Base::new(cache, viewer, selected)?;
    if !base.explored() || index >= base.names.len() {
        return Ok(None);
    }
    let at = |i: usize| i * ffe_core::sysgen::OBJECT_BYTES;
    let word = |i: usize, offset: usize| {
        u16::from_le_bytes([base.list[at(i) + offset], base.list[at(i) + offset + 1]])
    };
    let dword = |i: usize, offset: usize| {
        u32::from_le_bytes(
            base.list[at(i) + offset..at(i) + offset + 4]
                .try_into()
                .unwrap(),
        )
    };
    let kind = |i: usize| word(i, 0xa);
    let mut page = Page {
        background: None,
        sprites: base.sprites.clone(),
        texts: Vec::new(),
    };
    let mut vars = [0i32; 5];
    let (k, mass) = (kind(index), dword(index, 0));
    if k != 0 && k < 0x20 {
        // The mass's exponent less 0x3f is the Earth-mass scale.
        let adjusted = mass.wrapping_sub(0x3f << 16);
        vars[0] = i32::from(word(index, 0x24)) - 0x111;
        vars[1] = (scale(to_integer(adjusted) as u32, 0xab5a) >> 2) as i32;
        page.texts
            .push(base.text(10, 0x69, YELLOW, base.expand(0x84a1, &vars)?));
        // The starports: the marker objects (type 0x23) that follow.
        let mut starports = Vec::new();
        let mut next = index + 1;
        if base.list.len() >= at(next + 1) && kind(next) == 0x23 {
            while base.list.len() >= at(next + 1) && kind(next) == 0x23 && dword(next, 0) != 0 {
                if !starports.is_empty() {
                    starports.push(b',');
                }
                starports.extend(&base.names[next]);
                next += 1;
            }
        } else {
            starports = base.expand(0x84a7, &vars)?;
        }
        page.texts.push(base.text(0x52, 0x7d, YELLOW, starports));
    }
    let period = dword(index, 0x14);
    if period != 0 {
        let exponent = word(index, 0x16) as i16;
        let (selector, factor) = if exponent > 0x1b {
            (0x84a6, 0xffeb_551fu32)
        } else if exponent > 0x13 {
            (0x84a5, 0xfff0_6117)
        } else {
            (0x84a4, 0xfff5_48d1)
        };
        vars[1] = to_integer(multiply(period, factor));
        page.texts
            .push(base.text(0x52, 0x87, YELLOW, base.expand(selector, &vars)?));
        vars[0] = to_integer(multiply(dword(index, 0xe), 0xffdb_759a));
        vars[1] = i32::from(word(index, 0x20));
        vars[2] = (i32::from(word(index, 0x22)) * 0xe10) >> 16;
        vars[3] = dword(index, 4) as i32;
        // A body that shares its parent's orbit with a star pair is co-orbital.
        let parent = usize::from(word(index, 8));
        let co_orbital = parent > 0 && kind(parent - 1) == 0 && (parent as i32) > index as i32 - 2;
        let selector = if co_orbital {
            vars[0] <<= 1;
            0x84a3
        } else {
            0x84a2
        };
        page.texts
            .push(base.text(10, 0x87, YELLOW, base.expand(selector, &vars)?));
    }
    let mut line = base.names[index].clone();
    line.extend(b": ");
    line.extend(base.expand(0x8476 + k, &vars)?);
    let y = if k == 0 || k >= 0x20 { 0x7d } else { 0x5f };
    page.texts.push(base.text(10, y, YELLOW, line));
    Ok(Some(page))
}

/// `DATA_006821`: the palette index F847-F849 draw for a density of 0 to 63 stars.
const GREYS: [u8; 64] = [
    0, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7,
    8, 8, 8, 9, 9, 9, 10, 10, 10, 11, 11, 11, 12, 12, 12, 13, 13, 13, 13, 14, 14, 14, 14, 14, 15,
    15, 15, 15, 15, 15, 15, 15,
];

/// `DATA_006812`: the names F846 writes over the sky: the zoom they stop showing
/// at, the sector offsets (x, y) and the system-string selector. The last record
/// (zoom 0) ends the list.
const LABELS: [(i32, i32, i32, u16); 14] = [
    (40, 3328, 7168, 0x84a8),
    (40, 6656, 4096, 0x84a9),
    (40, -512, 3328, 0x84aa),
    (40, 4096, 1280, 0x84ab),
    (16, 5912, 5412, 0x8401),
    (7, 5912, 5488, 0x8461),
    (6, 6004, 5418, 0x8459),
    (4, 5766, 5497, 0x8462),
    (4, 5944, 5444, 0x84ac),
    (3, 5873, 5378, 0x8457),
    (1, 5840, 5450, 0x84ad),
    (1, 5906, 5372, 0x8456),
    (1, 5971, 5426, 0x8458),
    (1, 5908, 5395, 0x8455),
];

/// The picture's size: the view's 320 x 158.
pub const PICTURE_WIDTH: usize = 320;
pub const PICTURE_HEIGHT: usize = 158;

/// F844's picture mode (F849/F848/F847 then F846): the galaxy's star density as
/// seen from the map's centre, `zoom` sectors to a pixel, in grey, with the names
/// of the nearby stars and features in red. The original draws it in three passes
/// over three frames (4x4, 2x2, then single pixels); this is the last. `centre`
/// is the map's centre in 16.16 sectors.
pub fn picture(
    cache: &VerifiedCache,
    viewer: &ModelViewer,
    centre: [i32; 2],
    zoom: i32,
) -> Result<Page, String> {
    let tables = SectorTables::new(
        cache
            .asset("base:world/galaxy-tables")
            .ok_or("The cache lacks the galaxy tables.")?,
    )?;
    let zoom = zoom.max(1);
    let (cx, cy) = (centre[0] >> 16, centre[1] >> 16);
    // F847: the pen starts at the bottom right and moves a sector (times the
    // zoom) per pixel; the rows run from y 157 up.
    let x_start = cx - (zoom >> 1) + zoom * 160;
    let y_start = cy + (zoom >> 1) - zoom * 0x4f;
    let mut pixels = vec![0u8; PICTURE_WIDTH * PICTURE_HEIGHT];
    for row in 0..PICTURE_HEIGHT {
        let y = y_start + zoom * (PICTURE_HEIGHT - 1 - row) as i32;
        for column in 0..PICTURE_WIDTH {
            let x = x_start - zoom * (PICTURE_WIDTH - 1 - column) as i32;
            let density = tables.star_count(x as u16 as u32, y as u16 as u32, zoom);
            let index = ((density >> 10).max(0) as usize).min(GREYS.len() - 1);
            pixels[row * PICTURE_WIDTH + column] = GREYS[index];
        }
    }
    // F846: each name whose zoom limit has not passed, if it lands inside the view.
    let messages = messages_of(cache);
    let lookup = |code: u16| string(viewer, messages.as_ref(), code);
    let mut texts = Vec::new();
    for (limit, x, y, code) in LABELS {
        if limit < zoom {
            continue;
        }
        let screen_x = (x - cx) / zoom + 0xa2;
        let screen_y = (-y + cy) / zoom + 0x4c;
        if (1..=0x95).contains(&screen_y) && (1..=0x136).contains(&screen_x) {
            if let Some(raw) = string(viewer, messages.as_ref(), code) {
                texts.push(PageText {
                    x: screen_x,
                    y: screen_y,
                    colour: RED,
                    wrap: 0,
                    bytes: expand(&raw, &[0; 5], &lookup),
                });
            }
        }
    }
    Ok(Page {
        sprites: Vec::new(),
        texts,
        background: Some(pixels),
    })
}

/// Used by the tests: the sector tables the selection comes from.
pub fn sector_tables(cache: &VerifiedCache) -> Option<SectorTables<'_>> {
    SectorTables::new(cache.asset("base:world/galaxy-tables")?).ok()
}
