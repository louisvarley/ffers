//! F77/F76: the `[` macros F1343 expands in every message string after its
//! FF codes (`docs/notes/journal.md` lists them). A macro is `[`, two
//! letters, then its arguments; F75 reads a seed (`G n` a global random word
//! F106, `L n` the article's slot word F95, `R` a fresh draw from the shared
//! pair), F74 a signed number.
//!
//! Ported: `Gr`, `Mi`, `Ca`, `Fm`/`Ff`/`Cr`/`Co`, `Dr`, `Nu`, `Sl`, `Oo`,
//! `Ri`, `Rf`, `Re`, `Sh`, `Si`, `Pn`, `Cs`, `Sy`, `Ci`, `Pl`, `Sc`, `Sr` and `Zz`: every code
//! of `DATA_102307`. `Pl` picks the object `Ci` would (F461/F462/F463) and names its parent
//! (the record's `+8` word; the table's second `Sy`/`Pl` entries at `+0x13d` and `+0x140`,
//! `ffe.asm:9627`), a planet where `Ci` gives a starport, as the executed original does. `Sc` writes the
//! picked system's sector coordinates, `Sr` a random ship model's name and `Si` the player's
//! ship registration. An unknown code drops its two letters and keeps the `[`, as F76 does.

/// What the macros read from the game and the scripts.
pub trait Macros {
    /// F75: seed `kind` (`b'G'`, `b'L'`, `b'R'`) with number `n`.
    fn seed(&mut self, kind: u8, n: i32) -> i32;
    /// F64: the scripts' text for query `name` with `word`.
    fn query(&mut self, word: i32, name: &str) -> Vec<u8>;
    /// `DATA_008919`.
    fn commander(&mut self) -> Vec<u8>;
    /// `DATA_008888`, tenths of a credit.
    fn cash(&mut self) -> i64;
    /// `DATA_008807`.
    fn today(&mut self) -> u32;
    /// F791: a generated name of `kind` from `seed`.
    fn name(&mut self, kind: u32, seed: i32) -> Vec<u8>;
    /// F1358's date text for absolute day `day`.
    fn date(&mut self, day: u32) -> Vec<u8>;
    /// F76's `Re`: the player's Elite rating word.
    fn elite_rating(&mut self) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Ri`/`Rf`: the player's Imperial/Federal rank word.
    fn military_rank(&mut self, _imperial: bool) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Sh`: the player's ship model name.
    fn ship_model_name(&mut self) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Si`: the seed of the player's ship object (`+0xa0`), from which F791 kind `0x12`
    /// made the registration copied from `+0x124`.
    fn ship_seed(&mut self) -> i32 {
        0
    }
    /// F76's `Sr`: ship model `model`'s display name (statistics `+0xe` through F1344).
    fn model_name(&mut self, _model: u32) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Cs`: the current system's F860 name.
    fn current_system_name(&mut self) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Sy`/`Ci`: the system the macro names. `mode` is `Some` for the
    /// F858 form (F74's number after F75's seed), `None` for the `(argument)`
    /// form, where `argument` is the parenthesized text. Must fall back to the
    /// current system when nothing is found, as F76 does.
    fn select_system(&mut self, _seed: i32, _mode: Option<i32>, _argument: &[u8]) -> u32 {
        0
    }
    /// F860: system `code`'s name.
    fn system_name(&mut self, _code: u32) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Ci`: an object's name in system `code` (F461/F462/F463).
    fn system_object_name(&mut self, _seed: i32, _code: u32) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Pl`: the name of the parent object of the object `Ci` would pick.
    fn system_place_name(&mut self, _seed: i32, _code: u32) -> Vec<u8> {
        Vec::new()
    }
    /// F76's `Zz`: clear one lazy random word in `DATA_008567`.
    fn reset_random_word(&mut self, _index: i32) {}
}

/// F921 on two seed words: a number below `range` (the words advance).
fn f921(range: u32, a: &mut u32, b: &mut u32) -> u32 {
    *a = a.wrapping_add(*b);
    *b = b.rotate_left(5);
    *a = a.wrapping_add(*b).rotate_left(16);
    (*a & 0xffff).wrapping_mul(range) >> 16
}

struct Reader<'a> {
    text: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> u8 {
        self.text.get(self.at).copied().unwrap_or(0)
    }

    fn take(&mut self) -> u8 {
        let byte = self.peek();
        if self.at < self.text.len() {
            self.at += 1;
        }
        byte
    }

    /// F74: an optional `-` and decimal digits.
    fn number(&mut self) -> i32 {
        let sign = if self.peek() == b'-' {
            self.at += 1;
            -1
        } else {
            1
        };
        let mut value: i32 = 0;
        while self.peek().is_ascii_digit() {
            value = value
                .wrapping_mul(10)
                .wrapping_add(i32::from(self.take() - b'0'));
        }
        value.wrapping_mul(sign)
    }

    /// F75.
    fn seed(&mut self, macros: &mut dyn Macros) -> i32 {
        match self.take() {
            kind @ (b'G' | b'L') => {
                let n = self.number();
                macros.seed(kind, n)
            }
            b'R' => macros.seed(b'R', 0),
            _ => 0,
        }
    }

    /// `(a,b)` after a seed: skip a byte, number, skip, number, skip.
    fn bounds(&mut self) -> (i32, i32) {
        self.take();
        let low = self.number();
        self.take();
        let high = self.number();
        self.take();
        (low, high)
    }

    /// The bytes in one parenthesized argument group, including nested groups.
    fn group(&mut self) -> &'_ [u8] {
        self.take(); // opening parenthesis
        let start = self.at;
        let mut depth = 0usize;
        let mut quoted = false;
        while self.peek() != 0 {
            let byte = self.take();
            if byte == b'"' {
                quoted = !quoted;
            } else if !quoted {
                match byte {
                    b'(' => depth += 1,
                    b')' if depth == 0 => return &self.text[start..self.at - 1],
                    b')' => depth -= 1,
                    _ => {}
                }
            }
        }
        &self.text[start..self.at]
    }
}

/// F76's argument lists use commas between alternatives except inside quotes
/// or nested groups.
fn alternatives(args: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut quoted = false;
    for (index, byte) in args.iter().enumerate() {
        if *byte == b'"' {
            quoted = !quoted;
        } else if !quoted {
            match *byte {
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                b',' if depth == 0 => {
                    out.push(&args[start..index]);
                    start = index + 1;
                }
                _ => {}
            }
        }
    }
    out.push(&args[start..]);
    out
}

fn choice(seed: i32, count: usize) -> usize {
    let mut a = seed as u32;
    let mut b = a;
    f921(count as u32, &mut a, &mut b) as usize
}

/// F77: expand every `[` macro in `text`; `0xd5` becomes `'`.
pub fn expand(text: &[u8], macros: &mut dyn Macros) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut reader = Reader { text, at: 0 };
    while reader.at < text.len() {
        let byte = reader.take();
        match byte {
            0xd5 => out.push(b'\''),
            b'[' => macro_(&mut reader, &mut out, macros),
            _ => out.push(byte),
        }
    }
    out
}

/// F76 at a `[` (already read).
fn macro_(reader: &mut Reader<'_>, out: &mut Vec<u8>, macros: &mut dyn Macros) {
    let start = reader.at;
    let code = [reader.take(), reader.take()];
    match &code {
        b"Gr" => {
            let n = reader.number();
            out.push(2);
            out.push(n as u8);
        }
        b"Mi" => {
            let word = reader.seed(macros);
            reader.take();
            let mut name = Vec::new();
            while !matches!(reader.peek(), 0 | b')') {
                name.push(reader.take());
            }
            reader.take();
            let name = String::from_utf8_lossy(&name).into_owned();
            out.extend(macros.query(word, &name));
        }
        b"Ca" => {
            let cash = macros.cash();
            out.extend(format!("${}.{}", cash / 10, (cash % 10).abs()).bytes());
        }
        b"Fm" | b"Ff" | b"Cr" | b"Co" => {
            let seed = reader.seed(macros);
            let kind = match &code {
                b"Fm" => 1,
                b"Ff" => 2,
                _ => 12,
            };
            out.extend(macros.name(kind, seed));
        }
        b"Dr" | b"Nu" => {
            let seed = reader.seed(macros);
            let (low, high) = reader.bounds();
            let (mut a, mut b) = (seed as u32, seed as u32);
            let draw = f921(high.wrapping_sub(low) as u32, &mut a, &mut b) as i32;
            if &code == b"Dr" {
                let day = (macros.today() as i32).wrapping_add(low).wrapping_add(draw);
                out.extend(macros.date(day as u32));
            } else {
                out.extend(low.wrapping_add(draw).to_string().bytes());
            }
        }
        b"Sl" => {
            let seed = reader.seed(macros);
            let args = reader.group();
            let mut choices = Vec::new();
            let mut choice_start = None;
            for (index, byte) in args.iter().enumerate() {
                match (*byte, choice_start) {
                    (b'"', None) => choice_start = Some(index + 1),
                    (b'"', Some(begin)) => {
                        choices.push(&args[begin..index]);
                        choice_start = None;
                    }
                    _ => {}
                }
            }
            if !choices.is_empty() {
                out.extend_from_slice(choices[choice(seed, choices.len())]);
            }
        }
        b"Oo" => {
            let seed = reader.seed(macros);
            let choices = alternatives(reader.group());
            if !choices.is_empty() {
                out.extend(expand(choices[choice(seed, choices.len())], macros));
            }
        }
        b"Ri" => out.extend(macros.military_rank(true)),
        b"Rf" => out.extend(macros.military_rank(false)),
        b"Re" => out.extend(macros.elite_rating()),
        b"Sh" => out.extend(macros.ship_model_name()),
        b"Pn" => out.extend(macros.commander()),
        b"Cs" => {
            let _ = reader.seed(macros);
            out.extend(macros.current_system_name());
        }
        b"Sy" | b"Ci" | b"Pl" => {
            // F75's seed, then F74's number (the F858 form) or the
            // parenthesized argument. The source branches on a digit or '-'
            // after the seed.
            let seed = reader.seed(macros);
            let (mode, argument) = if reader.peek().is_ascii_digit() || reader.peek() == b'-' {
                (Some(reader.number()), &[][..])
            } else {
                (None, reader.group())
            };
            let system = macros.select_system(seed, mode, argument);
            match &code {
                b"Sy" => out.extend(macros.system_name(system)),
                // The `(argument)` form names `Ci`'s object itself; the F858 form's shared
                // tail (`JUMP_000710`) and `Pl` name the object its `+8` word indexes.
                b"Ci" if mode.is_none() => out.extend(macros.system_object_name(seed, system)),
                _ => out.extend(macros.system_place_name(seed, system)),
            }
        }
        b"Si" => {
            // `+0x124` of the player's ship object: F791's registration (`XX-123`).
            let seed = macros.ship_seed();
            out.extend(macros.name(0x12, seed));
        }
        b"Sc" => {
            // F75, a skipped byte, F74's number, a skipped byte: F858's pick (F860 names it
            // into a buffer that is never copied), then `(x,y)` from the code's two 13-bit
            // halves less the galaxy's origin.
            let seed = reader.seed(macros);
            reader.take();
            let mode = reader.number();
            reader.take();
            let code = macros.select_system(seed, Some(mode), &[]);
            let x = i64::from(code & 0x1fff) - 0x1718;
            let y = i64::from((code >> 13) & 0x1fff) - 0x1524;
            out.extend(format!("({x},{y})").into_bytes());
        }
        b"Sr" => {
            // F75's seed twice into F921: a model from 0xf to 0x3e, named as `Sh` does.
            let seed = reader.seed(macros);
            let (mut a, mut b) = (seed as u32, seed as u32);
            let model = f921(0x30, &mut a, &mut b) + 0xf;
            out.extend(macros.model_name(model));
        }
        b"Zz" => {
            for argument in alternatives(reader.group()) {
                let mut number = Reader {
                    text: argument,
                    at: 0,
                };
                macros.reset_random_word(number.number());
            }
        }
        _ => {
            // Not ported yet: keep the macro as written.
            if is_known(&code) {
                out.push(b'[');
                reader.at = start;
            } else {
                // F76's unknown code: the `[` stays, the two letters go.
                out.push(b'[');
            }
        }
    }
}

/// `DATA_102307`'s two-letter codes.
fn is_known(code: &[u8; 2]) -> bool {
    const CODES: [&[u8; 2]; 26] = [
        b"Gr", b"Mi", b"Ca", b"Fm", b"Ff", b"Cr", b"Co", b"Dr", b"Nu", b"%i", b"Ri", b"Rf", b"Re",
        b"Sh", b"Si", b"Sl", b"Oo", b"Pn", b"Zz", b"Sy", b"Ci", b"Pl", b"Sc", b"Cs", b"Sr", b"di",
    ];
    CODES.contains(&code)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed {
        seed: i32,
        resets: Vec<i32>,
    }
    impl Macros for Fixed {
        fn seed(&mut self, kind: u8, n: i32) -> i32 {
            if self.seed == 0 {
                i32::from(kind) * 100 + n
            } else {
                self.seed
            }
        }
        fn query(&mut self, word: i32, name: &str) -> Vec<u8> {
            format!("<{name}:{word}>").into_bytes()
        }
        fn commander(&mut self) -> Vec<u8> {
            b"Jameson".to_vec()
        }
        fn cash(&mut self) -> i64 {
            1234
        }
        fn today(&mut self) -> u32 {
            1000
        }
        fn name(&mut self, kind: u32, seed: i32) -> Vec<u8> {
            format!("N{kind}/{seed}").into_bytes()
        }
        fn date(&mut self, day: u32) -> Vec<u8> {
            format!("D{day}").into_bytes()
        }
        fn elite_rating(&mut self) -> Vec<u8> {
            b"Harmless".to_vec()
        }
        fn military_rank(&mut self, imperial: bool) -> Vec<u8> {
            if imperial {
                b"Imperial".to_vec()
            } else {
                b"Federal".to_vec()
            }
        }
        fn ship_model_name(&mut self) -> Vec<u8> {
            b"Saker Mk III".to_vec()
        }
        fn current_system_name(&mut self) -> Vec<u8> {
            b"Hope".to_vec()
        }
        fn select_system(&mut self, _seed: i32, mode: Option<i32>, argument: &[u8]) -> u32 {
            match (mode, argument) {
                (Some(mode), _) => 0x7000 + mode as u32,
                (None, b"I") => 0x111,
                (None, b"C") => 0x222,
                _ => 0,
            }
        }
        fn system_name(&mut self, code: u32) -> Vec<u8> {
            if code == 0 {
                b"Hope".to_vec()
            } else {
                format!("S{code:x}").into_bytes()
            }
        }
        fn system_object_name(&mut self, _seed: i32, code: u32) -> Vec<u8> {
            format!("O{code:x}").into_bytes()
        }
        fn system_place_name(&mut self, _seed: i32, code: u32) -> Vec<u8> {
            format!("P{code:x}").into_bytes()
        }
        fn reset_random_word(&mut self, index: i32) {
            self.resets.push(index);
        }
        fn ship_seed(&mut self) -> i32 {
            0x1234
        }
        fn model_name(&mut self, model: u32) -> Vec<u8> {
            format!("M{model:x}").into_bytes()
        }
    }

    #[test]
    fn the_script_texts_macros_expand() {
        let mut fixed = Fixed {
            seed: 0,
            resets: Vec::new(),
        };
        let text = b"Commander [Pn. By [MiR(Finn date1), $[MiL0(cash) and [Ca.";
        let out = expand(text, &mut fixed);
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "Commander Jameson. By <Finn date1:8200>, $<cash:7600> and $123.4."
        );
        let numbers = String::from_utf8(expand(b"[NuG2(10,20) [FfG1", &mut fixed)).unwrap();
        let (low, rest) = numbers.split_once(' ').unwrap();
        let value: i32 = low.parse().unwrap();
        assert!((10..20).contains(&value));
        assert_eq!(rest, "N2/7101");
        assert_eq!(
            expand(b"[CrL1 Corporation", &mut fixed),
            b"N12/7601 Corporation"
        );
        assert_eq!(expand(b"[Cs and", &mut fixed), b"Hopeand");
        assert_eq!(expand(b"[SlL1(\"his\",\"her\")", &mut fixed), b"his");
        assert_eq!(expand(b"[SlL1(\"h,is\",\"her\")", &mut fixed), b"h,is");
        assert_eq!(
            expand(b"[OoL1([SlL1(\"his\",\"her\"),other)", &mut fixed),
            b"his"
        );
        fixed.seed = 0x4000_0000;
        assert_eq!(expand(b"[SlL1(\"his\",\"her\")", &mut fixed), b"her");
        assert_eq!(
            expand(b"[OoL1(one,[SlL1(\"his\",\"her\"))", &mut fixed),
            b"her"
        );
        assert_eq!(expand(b"[Re", &mut fixed), b"Harmless");
        assert_eq!(expand(b"[Ri [Rf", &mut fixed), b"Imperial Federal");
        assert_eq!(expand(b"[Sh", &mut fixed), b"Saker Mk III");
        assert_eq!(expand(b"[Zz(1,2,-3)", &mut fixed), b"");
        assert_eq!(fixed.resets, [1, 2, -3]);
        // F76's Sy/Ci: the argument filter picks a system (fixed lists and
        // classes) and Sy names it through F860; Ci names one of its objects.
        // The F858 form takes F74's number; an unknown argument keeps the
        // current system.
        assert_eq!(
            expand(b"[SyL1(I) meets [SyL1(C)", &mut fixed),
            b"S111 meets S222"
        );
        assert_eq!(expand(b"[SyR2", &mut fixed), b"S7002");
        assert_eq!(expand(b"[SyL1(zed)", &mut fixed), b"Hope");
        assert_eq!(expand(b"[CiL1(C) City", &mut fixed), b"O222 City");
        // The F858 form of `Ci` names the parent object, as `Pl` does.
        assert_eq!(expand(b"[CiR3 x", &mut fixed), b"P7003 x");
        // `Pl` shares `Ci`'s object-naming path.
        assert_eq!(expand(b"[PlL1(I) place", &mut fixed), b"P111 place");
        // `Sc`: seed, skipped byte, number, skipped byte, then the pick's coordinates.
        assert_eq!(
            expand(b"[ScL1,3, here", &mut fixed),
            b"(-1813,-5409) here".to_vec()
        );
        // `Si` is F791's registration of the player's ship seed; `Sr` a model name.
        assert_eq!(expand(b"[Si", &mut fixed), b"N18/4660".to_vec());
        assert!(expand(b"[SrL1", &mut fixed).starts_with(b"M"));
        // An unknown code loses its letters but keeps the bracket.
        assert_eq!(expand(b"[Qq!", &mut fixed), b"[!".to_vec());
        assert_eq!(expand(&[0xd5], &mut fixed), b"'".to_vec());
    }
}
