//! The original's mission scripts (cache v32): every mission struct, state
//! and condition in the player's FIRSTENC.EXE decodes to its end exactly as
//! F31/F41 read it. Local-only: needs `FFE_WORLD_CACHE`.
use ffe_core::script::Image;

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn every_script_decodes_as_the_interpreter_reads_it() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    assert_eq!(image.missions(), ffe_core::script::MISSIONS);
    assert_eq!(image.static_words(), 10);
    let (mut states, mut conditions, mut quirky) = (0, 0, Vec::new());
    for mission in 0..image.missions() {
        let record = image.mission_struct(mission).unwrap();
        image.take_quirks();
        let reachable = image.reachable_states(&record).unwrap();
        if image.take_quirks() > 0 {
            quirky.push(mission);
        }
        states += reachable.len();
        for state in reachable {
            conditions += image
                .conditions(image.state(&record, state).unwrap())
                .unwrap();
        }
    }
    assert_eq!((states, conditions), (334, 667));
    // Mission 83's state 4 compares token 0x2b, which the shipping F31 reads
    // with two operands although the token record and the script give it one.
    // The second read used to swallow its `goto 7`; the correction keeps the
    // goto, making state 7 reachable (EXP-034).
    assert!(quirky.is_empty(), "{quirky:?}");
    // Mission 0, the one F55 starts on a new game: no parameters, four
    // locals and a single state.
    let first = image.mission_struct(0).unwrap();
    assert_eq!((first.parameter_count, first.locals), (0, 4));
    assert_eq!(image.reachable_states(&first).unwrap(), vec![0]);
}

/// The corrections listed in `docs/notes/broken-missions.md` are applied to
/// the decoded image; the imported asset itself keeps the player's original
/// (broken) bytes. See `docs/DEVIATIONS.md` EXP-030 to EXP-035.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_broken_mission_data_is_corrected_in_memory() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let bytes = cache.asset("base:script/missions").unwrap();
    // `Image::encode`: magic, table, missions, static words, pointer count,
    // the pointer offsets, the region length, then the region.
    let pointers = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]) as usize;
    let start = 20 + 4 * pointers;
    let blob = &bytes[start + 4..];
    // What the player's FIRSTENC.EXE holds at each corrected site.
    assert_eq!(&blob[0x054d..0x0553], b"\x17\x02\x65\xa9\x00\x00"); // 55: no 0xa963
    assert_eq!(&blob[0x5f08..0x5f0c], &0x0a_i32.to_le_bytes()); // 20: 10 * L4
    assert_eq!(&blob[0x5f6e..0x5f72], &0x78_i32.to_le_bytes()); // 20: 120 * L4
    assert_eq!(&blob[0x6046..0x604a], &0x3e8_i32.to_le_bytes()); // 20: 1000 * L4
    assert_eq!(&blob[0x7efe..0x7f02], &0x217_i32.to_le_bytes()); // 78: wrong article
    assert_eq!(
        &blob[0xcfd8..0xcfe3],
        b"\x15\x04\x00\x00\x00\x00\x02\x02\x00\x00\x00"
    ); // 81: G0 == 2
    assert_eq!(&blob[0x4245..0x424a], b"\x09\x07\x00\x00\x00"); // 83: swallowed goto

    let image = Image::new(bytes).unwrap();

    // 78: state 16 waits for the article its entry queued into G17.
    let record = image.mission_struct(78).unwrap();
    let state = image.state(&record, 16).unwrap();
    let (text, body) = image
        .expression_text(image.condition(state, 0).unwrap())
        .unwrap();
    assert!(text.contains("G17 0x197"), "{text}");
    assert!(!text.contains("0x217"), "{text}");
    assert!(image
        .listing(body)
        .unwrap()
        .iter()
        .any(|line| line.contains("9 20")));

    // 55: all four questions are on the screen, in message order.
    let listing = image.mission_listing(55).unwrap().join("\n");
    let order: Vec<usize> = ["23 0xa95f", "23 0xa961", "23 0xa963", "23 0xa965"]
        .iter()
        .map(|entry| {
            listing
                .find(entry)
                .unwrap_or_else(|| panic!("{entry} missing from the offer"))
        })
        .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    // Starting the offer shows all four questions in the same order.
    {
        use ffe_core::script::engine::Host;
        use ffe_core::script::run::{Game, Scripts};
        struct Quiet;
        impl Host for Quiet {}
        impl Game for Quiet {}
        let mut scripts = Scripts::new(&image);
        let mut game = Quiet;
        scripts
            .start_mission(&image, &mut game, 0x12_1cf7, 55, &[0])
            .unwrap();
        let codes: Vec<i32> = scripts.engine.menu.entries.iter().map(|e| e.code).collect();
        for code in [0xa95f, 0xa961, 0xa963, 0xa965, 0xa900] {
            assert!(codes.contains(&code), "{code:#x} missing from {codes:x?}");
        }
    }

    // 20: each debit is the displayed price.
    let listing = image.mission_listing(20).unwrap().join("\n");
    for debit in [
        "(0xe P2 (0xa L4 0x1))",
        "(0xe P2 (0xa L4 0xc))",
        "(0xe P2 (0xa L4 0x16))",
        "(0xe P2 (0xa L4 0x64))",
    ] {
        assert!(listing.contains(debit), "{debit} missing");
    }
    for overcharge in [
        "(0xe P2 (0xa L4 0xa))",
        "(0xe P2 (0xa L4 0x78))",
        "(0xe P2 (0xa L4 0x3e8))",
    ] {
        assert!(!listing.contains(overcharge), "{overcharge} remains");
    }

    // 81: only the 0xa841 response pays the reward.
    let record = image.mission_struct(81).unwrap();
    let state = image.state(&record, 7).unwrap();
    let (text, _) = image
        .expression_text(image.condition(state, 0).unwrap())
        .unwrap();
    assert_eq!(text, "(0x1b (0x15 0x2 0x3) (0x15 G0 0xa841))");

    // 83: 0x2b reads one operand, so the goto is a statement and state 7 is
    // reachable.
    let record = image.mission_struct(83).unwrap();
    assert!(image.reachable_states(&record).unwrap().contains(&7));
    let state = image.state(&record, 4).unwrap();
    let (text, body) = image
        .expression_text(image.condition(state, 1).unwrap())
        .unwrap();
    assert_eq!(text, "(0x15 (0x2b P4) 0x4)");
    assert!(image
        .listing(body)
        .unwrap()
        .iter()
        .any(|line| line.contains(": 9 7")));
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn a_new_game_starts_the_scripts() {
    use ffe_core::script::engine::{Engine, Host};
    #[derive(Default)]
    struct Recorder {
        log: Vec<String>,
    }
    impl Host for Recorder {
        fn query(&mut self, token: u8, args: &[i32]) -> i32 {
            self.log.push(format!("query {token:#x} {args:?}"));
            0
        }
        fn statement(&mut self, opcode: u8, args: &[i32], text: &[u8]) -> i32 {
            self.log.push(format!(
                "statement {opcode} {args:?} {}",
                String::from_utf8_lossy(text)
            ));
            0
        }
        fn assign(&mut self, token: u8, args: &[i32], value: i32) -> Option<(i32, i32, i32)> {
            self.log
                .push(format!("assign {token:#x} {args:?} = {value}"));
            None
        }
    }
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let mut engine = Engine::new(&image);
    let mut host = Recorder::default();
    engine.new_game(&image, &mut host).unwrap();
    let live: Vec<_> = engine.live().collect();
    println!("live {live:?}");
    println!("subscriptions {}", engine.subscription_count());
    for line in &host.log {
        println!("{line}");
    }
    assert_eq!((live.len(), engine.subscription_count()), (20, 94));
    assert!(host.log[0].ends_with("Imperial Herald"));
}

/// The scripts create the journals and F68/F90 publish their issues: a new
/// game, then 120 days of daily messages, with the shared pair standing in
/// for the game's draws.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_scripts_publish_the_journals() {
    use ffe_core::journal::{decode_stories, Journal};
    use ffe_core::script::engine::Host;
    use ffe_core::script::run::{Game, Scripts};
    struct Draws {
        random: ffe_core::npc::Random,
        notices: Vec<String>,
    }
    impl Host for Draws {}
    impl Game for Draws {
        fn bounded(&mut self, bound: u32) -> u32 {
            self.random.bounded(bound)
        }
        fn advance(&mut self) -> i32 {
            self.random.advance() as i32
        }
        fn post(&mut self, text: &str) {
            self.notices.push(text.to_owned());
        }
    }
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let stories = decode_stories(cache.asset("base:text/journal-stories").unwrap()).unwrap();
    assert_eq!(stories.len(), 170);
    let archive = Journal::new(cache.asset("base:text/journal").unwrap()).unwrap();
    let mut game = Draws {
        random: ffe_core::npc::Random::start(),
        notices: Vec::new(),
    };
    let mut scripts = Scripts::new(&image);
    let start = 0x121cf7 + 1200;
    scripts
        .new_game(&image, &stories, &mut game, start, true)
        .unwrap();
    let names: Vec<String> = scripts
        .library
        .journals
        .iter()
        .map(|journal| String::from_utf8_lossy(&journal.name).into_owned())
        .collect();
    assert_eq!(
        names,
        [
            "Imperial Herald",
            "Federal Times",
            "RIG",
            "Frontier News",
            "Universal Scientist"
        ]
    );
    for day in start + 1..start + 120 {
        scripts
            .daily(&image, &stories, &mut game, day, true)
            .unwrap();
    }
    for (slot, journal) in scripts.library.journals.iter().enumerate() {
        let issue = scripts.library.shown(slot);
        println!(
            "{}: next {} shown {issue:?}",
            names[slot],
            journal.next_issue - start
        );
        for article in issue.iter().take(2) {
            let headline = archive.article(*article as usize).unwrap().headline;
            println!("   {}", String::from_utf8_lossy(headline));
        }
    }
    println!(
        "{} notices, first: {:?}",
        game.notices.len(),
        game.notices.first()
    );
    assert_eq!(game.notices.len(), 20);
    assert!(scripts
        .library
        .journals
        .iter()
        .all(|j| j.next_issue == start + 140));
}

#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn census_of_statements_and_tokens() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let (statements, tokens, users) = image.census_by_mission().unwrap();
    let list = |counts: &[u32; 256]| {
        counts
            .iter()
            .enumerate()
            .filter(|(_, n)| **n > 0)
            .map(|(k, n)| format!("{k:#x}:{n}"))
            .collect::<Vec<_>>()
            .join(" ")
    };
    println!("statements {}", list(&statements));
    println!("tokens {}", list(&tokens));
    for (opcode, missions) in users.iter().enumerate() {
        if opcode >= 19 && !missions.is_empty() {
            println!("statement {opcode} in missions {missions:?}");
        }
    }
}

/// The script and journal state survive a save as JSON.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_script_state_round_trips_through_a_save() {
    use ffe_core::journal::decode_stories;
    use ffe_core::script::engine::Host;
    use ffe_core::script::run::{Game, Scripts};
    struct Draws(ffe_core::npc::Random);
    impl Host for Draws {}
    impl Game for Draws {
        fn bounded(&mut self, bound: u32) -> u32 {
            self.0.bounded(bound)
        }
        fn advance(&mut self) -> i32 {
            self.0.advance() as i32
        }
    }
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let stories = decode_stories(cache.asset("base:text/journal-stories").unwrap()).unwrap();
    let mut game = Draws(ffe_core::npc::Random::start());
    let mut scripts = Scripts::new(&image);
    scripts
        .new_game(&image, &stories, &mut game, 0x121cf7, true)
        .unwrap();
    let json = serde_json::to_string(&scripts).unwrap();
    let pretty = serde_json::to_vec_pretty(&scripts).unwrap().len();
    println!("pretty save size {pretty}");
    assert!(pretty < 400_000, "{pretty}");
    let back: Scripts = serde_json::from_str(&json).unwrap();
    assert_eq!(back, scripts);
    assert_eq!(
        back.engine.subscription_count(),
        scripts.engine.subscription_count()
    );
}

/// Opening a bulletin board after a new game: the scripts add their own
/// adverts to the station's (here none) on F453's signal.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_scripts_advertise_on_the_bulletin_board() {
    use ffe_core::journal::decode_stories;
    use ffe_core::message_strings::Messages;
    use ffe_core::script::engine::Host;
    use ffe_core::script::run::{Game, Scripts};
    struct Station {
        random: ffe_core::npc::Random,
        system: i32,
        clock: i32,
    }
    impl Host for Station {
        fn player(&mut self, case: i32) -> i32 {
            match case {
                2 => 100_000,
                4 => self.system,
                5..=11 => self.clock,
                _ => 0,
            }
        }
    }
    impl Game for Station {
        fn bounded(&mut self, bound: u32) -> u32 {
            self.random.bounded(bound)
        }
        fn advance(&mut self) -> i32 {
            self.random.advance() as i32
        }
    }
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let stories = decode_stories(cache.asset("base:text/journal-stories").unwrap()).unwrap();
    let messages = Messages::new(cache.asset("base:text/messages").unwrap()).unwrap();
    let mut game = Station {
        random: ffe_core::npc::Random::start(),
        system: 0,
        clock: 0,
    };
    let mut scripts = Scripts::new(&image);
    let start = 0x121cf7;
    scripts
        .new_game(&image, &stories, &mut game, start, true)
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for day in 0..400 {
        game.clock = day * 86_400;
        scripts
            .daily(&image, &stories, &mut game, start + day as u32, true)
            .unwrap();
        scripts
            .open_screen(&image, &mut game, start + day as u32, 2)
            .unwrap();
        for entry in &scripts.engine.menu.entries {
            if seen.insert(entry.code) {
                let text = messages.text(entry.code as u16).unwrap_or_default();
                println!(
                    "day {day}: {:#x} {:?} {}",
                    entry.code,
                    entry.words,
                    &text[..text.len().min(90)]
                );
            }
        }
    }
    println!("header {:#x}", scripts.engine.menu.header.code);
    assert!(!seen.is_empty(), "no script adverts");
    // Choose the RIG subscription offer and follow the conversation.
    let today = start + 400;
    scripts.open_screen(&image, &mut game, today, 2).unwrap();
    let show = |scripts: &Scripts, what: &str| {
        let menu = &scripts.engine.menu;
        println!(
            "-- {what}: screen {} title {:#x} header {:#x} active {}",
            menu.screen, menu.title, menu.header.code, menu.active
        );
        for entry in &menu.entries {
            let text = messages.text(entry.code as u16).unwrap_or_default();
            println!(
                "   {:#x} {:?} {}",
                entry.code,
                entry.words,
                &text[..text.len().min(80)]
            );
        }
        for line in &menu.lines {
            let text = messages.text(line.entry.code as u16).unwrap_or_default();
            println!(
                "   line {:#x} {:?}",
                line.entry.code,
                &text[..text.len().min(80)]
            );
        }
    };
    let offer = *scripts
        .engine
        .menu
        .entries
        .iter()
        .find(|entry| entry.code == 0xa8ca)
        .unwrap();
    scripts.select(&image, &mut game, today, offer).unwrap();
    show(&scripts, "after choosing the offer");
    if let Some(first) = scripts.engine.menu.entries.first().copied() {
        scripts.engine.menu.lines.clear();
        scripts.select(&image, &mut game, today, first).unwrap();
        show(&scripts, "after the first response");
        let thanks = scripts.engine.menu.lines.last().unwrap().entry.code;
        assert_eq!(
            messages.text(thanks as u16).as_deref(),
            Some("Thank you for taking a subscription to RIG.")
        );
    }
}

/// F326's dispatch over the real scripts: opening the bulletin board twice
/// rebuilds the same rows, and entering the upgrades page (F326's redraw,
/// but F384's entry clears it) takes the board's entries away; returning to
/// the board restores them.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn the_other_station_screens_signal_the_scripts() {
    use ffe_core::journal::decode_stories;
    use ffe_core::script::engine::Host;
    use ffe_core::script::run::{Game, Scripts};
    struct Draws(ffe_core::npc::Random);
    impl Host for Draws {}
    impl Game for Draws {
        fn bounded(&mut self, bound: u32) -> u32 {
            self.0.bounded(bound)
        }
        fn advance(&mut self) -> i32 {
            self.0.advance() as i32
        }
    }
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let stories = decode_stories(cache.asset("base:text/journal-stories").unwrap()).unwrap();
    let mut game = Draws(ffe_core::npc::Random::start());
    let mut scripts = Scripts::new(&image);
    let today = 0x121cf7;
    scripts
        .new_game(&image, &stories, &mut game, today, true)
        .unwrap();
    scripts.open_screen(&image, &mut game, today, 2).unwrap();
    let board: Vec<i32> = scripts.engine.menu.entries.iter().map(|e| e.code).collect();
    assert!(!board.is_empty(), "the scripts advertise on the board");
    // F453 again: the board is rebuilt and the rows repeat.
    scripts.open_screen(&image, &mut game, today, 2).unwrap();
    let again: Vec<i32> = scripts.engine.menu.entries.iter().map(|e| e.code).collect();
    assert_eq!(again, board);
    // F326(4) is F385 (redraw) but entering it is F384's F331/F342 clear:
    // the board's adverts are not the upgrades page's rows.
    scripts.open_screen(&image, &mut game, today, 4).unwrap();
    assert!(
        scripts
            .engine
            .menu
            .entries
            .iter()
            .all(|entry| !board.contains(&entry.code)),
        "the upgrades page keeps the board's adverts"
    );
    // The market (3), repairs (8) and the ship list (5) behave the same; the
    // services root (0) is F333's own clear.
    for screen in [3, 8, 5, 0] {
        scripts.open_screen(&image, &mut game, today, 2).unwrap();
        assert_eq!(scripts.engine.menu.entries.len(), board.len());
        scripts
            .open_screen(&image, &mut game, today, screen)
            .unwrap();
        assert!(
            scripts
                .engine
                .menu
                .entries
                .iter()
                .all(|entry| !board.contains(&entry.code)),
            "screen {screen} kept the board's adverts"
        );
    }
    // Coming back opens the board again.
    scripts.open_screen(&image, &mut game, today, 2).unwrap();
    let back: Vec<i32> = scripts.engine.menu.entries.iter().map(|e| e.code).collect();
    assert_eq!(back, board);
    // F493's military office: on a fresh start mission 23 is not running, so
    // signal 3 adds no rows (the port's own offers are F493's DATA_009004).
    scripts.open_screen(&image, &mut game, today, 7).unwrap();
    assert!(scripts.engine.menu.entries.is_empty());
    // Mission 23's state 0 answers signal 3 with F346 (`24 0xa9b9`), the
    // secret mission the military office offers. Its third condition is the
    // mission's deadline (P8 beyond L4's time), so pass a future one.
    let id = scripts
        .start_mission(&image, &mut game, today, 23, &[0, 0, i32::MAX, i32::MAX])
        .unwrap();
    assert!(id > 0);
    scripts.open_screen(&image, &mut game, today, 7).unwrap();
    assert_eq!(scripts.engine.menu.entries.len(), 1);
    assert_eq!(scripts.engine.menu.entries[0].code, 0xa9b9);
    // Selecting it (F360) starts mission 25, whose F331 opens the
    // conversation: F493's row leads to the mission's suggested responses.
    scripts
        .select(&image, &mut game, today, scripts.engine.menu.entries[0])
        .unwrap();
    assert_eq!(scripts.engine.menu.screen, 1);
    assert_eq!(scripts.engine.menu.title, 0x98d8);
    let codes: Vec<i32> = scripts.engine.menu.entries.iter().map(|e| e.code).collect();
    assert!(codes.contains(&0xa9bb), "{codes:x?}");
    assert!(codes.contains(&0x983f), "{codes:x?}");
}

/// `FFE_SCRIPT_MISSION=n` (or `all`) prints a mission's states as text.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn list_a_mission() {
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let which = std::env::var("FFE_SCRIPT_MISSION").unwrap_or_else(|_| "0".to_owned());
    let missions: Vec<usize> = if which == "all" {
        (0..image.missions()).collect()
    } else {
        vec![which.parse().unwrap()]
    };
    for mission in missions {
        for line in image.mission_listing(mission).unwrap() {
            println!("{line}");
        }
    }
}

/// The game around a scripted mission: the player's cash, the shared pair,
/// the contracts, the system and the time, and what the scripts asked for.
struct Player {
    credits: u32,
    random: ffe_core::npc::Random,
    posts: Vec<String>,
    hold: Vec<u16>,
    loadout: ffe_core::equipment::Loadout,
    combat: ffe_rs::combat::Combat,
    missions: ffe_core::missions::State,
    legal: ffe_core::npc::Legal,
    career: ffe_core::npc::Career,
    system: u32,
    day: u32,
    actions: Vec<ffe_rs::scripts::Action>,
    next_ship: u32,
}

impl Player {
    /// One call into the scripts with the port's hooks, the way
    /// `scripts::with_all` makes them in the game.
    fn run<R>(
        &mut self,
        scripts: &mut ffe_core::script::run::Scripts,
        f: impl FnOnce(&mut ffe_core::script::run::Scripts, &mut ffe_rs::scripts::Hooks<'_>, u32) -> R,
    ) -> R {
        use ffe_rs::scripts::{BodyView, Cargo, Hooks, ShipView, Snapshot, BODY_IDS, PLAYER_ID};
        let snapshot = Snapshot {
            system: self.system,
            clock: f64::from(self.day) * 86_400.0,
            legal: self.legal,
            career: self.career,
            next_ship: self.next_ship,
            next_tag: ffe_rs::scripts::TAG_IDS,
            ships: vec![ShipView {
                id: PLAYER_ID,
                model: 30,
                ai: 0,
                target: 0,
                seed: 0,
                distance: 0,
            }],
            // The target system's starport the mission names.
            bodies: vec![BodyView {
                id: BODY_IDS + 1,
                model: 20,
                seed: 0x9c80_553e,
                class: 0x10,
            }],
            values: [0; 21],
            allegiance: 2,
            capacity: 20,
        };
        let mut hooks = Hooks::new(
            snapshot,
            &mut self.credits,
            &mut self.random,
            &mut self.posts,
            None,
            Some(Cargo {
                hold: &mut self.hold,
                capacity: 0,
                loadout: &mut self.loadout,
                combat: &mut self.combat,
            }),
            Some(&mut self.missions),
        );
        let result = f(scripts, &mut hooks, self.day);
        self.next_ship = hooks.next_ship;
        self.legal = hooks.legal;
        self.career = hooks.career;
        self.actions.extend(hooks.finish_actions());
        result
    }
}

/// Mission 25, the military assassination the storyline's mission 23 offers
/// on the military board, played from the offer to the reward with the
/// port's hooks: the conversation, accepting it (a contract, two attackers),
/// arriving in its target system (0x6a45718) before the target leaves (its ship waits
/// in the starport's bay), killing it after it has left (F236's offence and
/// event 0x16) and going home for the $1500 reward.
#[test]
#[ignore = "Local-only: needs the content cache (FFE_WORLD_CACHE)"]
fn an_assassination_mission_runs_from_the_offer_to_the_reward() {
    use ffe_rs::scripts::{Action, BODY_IDS, PLAYER_ID};
    let cache =
        ffe_content::read(&std::fs::read(std::env::var_os("FFE_WORLD_CACHE").unwrap()).unwrap())
            .unwrap();
    let image = Image::new(cache.asset("base:script/missions").unwrap()).unwrap();
    let mut scripts = ffe_core::script::run::Scripts::new(&image);
    let origin = 0x0123_4567;
    let target = 0x06a4_5718;
    let start = 0x12_1cf7 + 100;
    let mut player = Player {
        credits: 1_000,
        random: ffe_core::npc::Random::start(),
        posts: Vec::new(),
        hold: vec![0; 0x26],
        loadout: Default::default(),
        combat: Default::default(),
        missions: Default::default(),
        legal: Default::default(),
        career: Default::default(),
        system: origin,
        day: start,
        actions: Vec::new(),
        next_ship: 1,
    };
    // The target leaves three days on (F21's seconds).
    let leaves = ((start + 3 - 0x12_1cf7) * 86_400) as i32;
    let id = player.run(&mut scripts, |scripts, hooks, today| {
        scripts
            .start_mission(&image, hooks, today, 25, &[0x10, 0x1234, leaves, 0])
            .unwrap()
    });
    assert_ne!(id, 0);
    let menu = &scripts.engine.menu;
    assert!(menu.active);
    let codes: Vec<i32> = menu.entries.iter().map(|entry| entry.code).collect();
    assert!(codes.contains(&0xa9c5), "{codes:x?}");

    // "OK - agreed." (0xa9c5): the contract and two F665 attackers.
    player.run(&mut scripts, |scripts, hooks, today| {
        let entry = ffe_core::script::menu::Entry {
            code: 0xa9c5,
            ..Default::default()
        };
        scripts.select(&image, hooks, today, entry).unwrap();
    });
    assert_eq!(player.missions.script_contracts.len(), 1);
    let attackers = player
        .actions
        .iter()
        .filter(|action| {
            matches!(
                action,
                Action::Attacker {
                    target: PLAYER_ID,
                    ..
                }
            )
        })
        .count();
    assert_eq!(attackers, 2);

    // F909's arrival in the target system: the target waits in the starport's bay.
    player.system = target;
    player.actions.clear();
    player.run(&mut scripts, |scripts, hooks, today| {
        scripts.player_changed(&image, hooks, today, 4).unwrap();
    });
    let ship = player
        .actions
        .iter()
        .find_map(|action| match action {
            Action::Departure {
                ship,
                starport,
                leaves: when,
                seed,
                ..
            } => {
                assert_eq!((*starport, *when, *seed), (BODY_IDS + 1, leaves, 0x1234));
                Some(*ship)
            }
            _ => None,
        })
        .expect("the target's ship");

    // A day after it left, the player destroys it: F924's removal.
    player.day = start + 4;
    player.run(&mut scripts, |scripts, hooks, today| {
        scripts
            .object_removed(
                &image,
                hooks,
                today,
                ship as i32 + ffe_rs::scripts::SHIP_IDS,
            )
            .unwrap();
    });
    assert!(
        player.legal.fines[3] > 0,
        "F236's offence in jurisdiction 3"
    );

    // Home again: F303-style reward and the contract gone.
    let before = player.credits;
    player.system = origin;
    player.run(&mut scripts, |scripts, hooks, today| {
        scripts.player_changed(&image, hooks, today, 4).unwrap();
    });
    assert_eq!(player.credits, before + 0x3a98);
    assert!(player.missions.script_contracts.is_empty());
    assert!(scripts.engine.live().all(|(_, mission, _)| mission != 25));
}
