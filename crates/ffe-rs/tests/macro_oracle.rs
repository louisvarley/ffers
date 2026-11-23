//! F76's `[` macros against the executed original (`content-2`). Local only: it needs the
//! content cache and the original's expansions of the same strings from the reference harness,
//! one `FFE_EXPAND` run per case (`tools/reference/headless.c`), as a JSON object
//! `{"<text>": {"text": "<expansion>", "pair": [a, b]}}`:
//!
//! ```text
//! FFE_EXPAND=<hex of the text> FFE_PAIR=12345678,9abcdef0 jjffe-smoke.exe --flight idle 230 --start-key=0x39
//! FFE_WORLD_CACHE=<data.bin> FFE_MACRO_ORACLE=<oracle.json> \
//!   cargo test -p ffe-rs --test macro_oracle -- --ignored --nocapture
//! ```
//!
//! The harness starts the default Hope game, sets the shared random pair to the one the port is
//! given here, and prints the expansion with the pair left behind.
//!
//! One difference is deliberate (`DEVIATIONS.md` EXP-026): in the executed JJFFE the
//! `(argument)` forms of `Sy`, `Ci` and `Pl` always fall back to the current system, because the
//! argument buffer F76 compares starts at the `(`; the port follows the DOS game's intent (the
//! archive's `(F edge)`, `(C)`, `(I)` pick systems of that kind), so those cases are skipped here.
use ffe_rs::flight::{Flight, Mode};
use ffe_rs::navigation::{Navigator, DEFAULT_SHIP};

/// The shared random pair the harness was given (`FFE_PAIR`; `FFE_MACRO_PAIR` here, default below).
fn pair() -> (u32, u32) {
    let text = std::env::var("FFE_MACRO_PAIR").unwrap_or_else(|_| "12345678,9abcdef0".to_owned());
    let (a, b) = text.split_once(',').expect("a,b in hex");
    (
        u32::from_str_radix(a, 16).unwrap(),
        u32::from_str_radix(b, 16).unwrap(),
    )
}

#[test]
#[ignore = "Local-only: needs the content cache and an oracle file (FFE_MACRO_ORACLE)"]
fn the_ports_macros_match_the_executed_originals() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let oracle: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(std::env::var_os("FFE_MACRO_ORACLE").unwrap()).unwrap(),
    )
    .unwrap();
    let assets = ffe_rs::scripts::Assets::load(&cache).expect("cache v33");
    let messages =
        ffe_core::message_strings::Messages::new(cache.asset("base:text/messages").unwrap())
            .unwrap();
    let mut mismatches = Vec::new();
    let mut total = 0;
    for (case, want) in oracle.as_object().unwrap() {
        let Some(want_text) = want["text"].as_str() else {
            continue;
        };
        // The argument forms JJFFE cannot resolve (EXP-026); `(zz)` is the fallback both agree on.
        if case.contains('(') && !case.contains("(zz)") {
            continue;
        }
        let mut nav = Navigator::new_game(&cache, 0).unwrap();
        let station = nav.docked.expect("the new game starts docked");
        let mut state = Flight {
            mode: Mode::Docked,
            ship: DEFAULT_SHIP,
            ..Flight::default()
        };
        ffe_rs::scripts::with_all(&mut state, &mut nav, &cache, |scripts, hooks, _| {
            *scripts = Some(ffe_rs::scripts::new_game(&assets, hooks, true).unwrap());
        });
        nav.random = ffe_core::npc::Random::new(pair().0, pair().1);
        let bytes: Vec<u8> = case.chars().map(|c| c as u32 as u8).collect();
        let got = ffe_rs::scripts::expand_text(
            &mut state, &mut nav, &cache, station, &assets, &messages, "Jameson", &bytes,
        )
        .expect("scripts");
        let got: String = got.into_iter().map(char::from).collect();
        let pair = [u64::from(nav.random.a), u64::from(nav.random.b)];
        let want_pair: Vec<u64> = want["pair"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap())
            .collect();
        total += 1;
        if got != want_text || pair.to_vec() != want_pair {
            mismatches.push(format!(
                "{case:?}: port {got:?} pair {pair:x?}; original {want_text:?} pair {want_pair:x?}"
            ));
        }
    }
    eprintln!("{total} cases, {} differ", mismatches.len());
    for line in &mismatches {
        eprintln!("  {line}");
    }
    assert!(
        mismatches.is_empty(),
        "{} macro cases differ",
        mismatches.len()
    );
}
