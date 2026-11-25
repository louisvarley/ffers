//! The Escape preference pages: layout, hot areas, keys, the applied settings
//! and the space colour in the drawn frame.
use ffe_rs::display::{DisplaySettings, Filter, FitMode, PixelAspect};
use ffe_rs::flight::Flight;
use ffe_rs::indexed::IndexedImage;
use ffe_rs::prefs::{
    self, escape_action, Activate, Escape, Key, Prefs, DISPLAY, GRAPHIC, MAIN, MAPS, OTHERS, PORT,
    SOUND,
};
use ffe_rs::screen::Screen;

/// F224 at the default start: Escape 1 stops time (no page), Escape 2 opens
/// the pages, Escape 3 returns to the game with the clock still stopped. The
/// reference `--game-frame` captures `hope-esc1..3.png` show the same order.
#[test]
fn escape_pauses_then_opens_the_pages_then_returns() {
    assert_eq!(escape_action(false, false), Escape::Pause);
    assert_eq!(escape_action(false, true), Escape::Open);
    assert_eq!(escape_action(true, true), Escape::Close);
    // The pages cannot be open while time runs in this port, but the function
    // still closes them if they are.
    assert_eq!(escape_action(true, false), Escape::Close);
}

#[test]
fn main_page_hot_areas_are_the_originals_icon_squares() {
    let mut prefs = Prefs::default();
    prefs.escape();
    // `DATA_004898`: the left column's 8x8 icon squares at x 32, y 28..78,
    // and the right column's at x 112.
    for (x, y, page) in [
        (34, 30, GRAPHIC),
        (34, 40, SOUND),
        (34, 50, OTHERS),
        (34, 60, MAPS),
        (34, 70, PORT),
    ] {
        prefs.page = MAIN;
        assert_eq!(prefs.click(x, y), Activate::None);
        assert_eq!(prefs.page, page);
    }
    // The label itself is not a hot area, as in the original.
    prefs.page = MAIN;
    assert_eq!(prefs.click(70, 30), Activate::None);
    assert_eq!(prefs.page, MAIN);
    // Return to Game closes and Quit reports the port's exit. The other main
    // menu rows route into the native commander workflows.
    assert_eq!(prefs.click(114, 60), Activate::Close);
    assert!(!prefs.open);
    prefs.open = true;
    assert_eq!(prefs.click(34, 80), Activate::Quit);
    for (x, y, action) in [
        (114, 30, Activate::Load),
        (114, 40, Activate::Save),
        (114, 50, Activate::Delete),
        (114, 80, Activate::NewGame),
    ] {
        assert_eq!(prefs.click(x, y), action);
    }
    assert!(prefs
        .rows()
        .iter()
        .filter(|row| matches!(row.label, "Load" | "Save" | "Delete" | "New Game"))
        .all(|row| row.applied));
    for (index, action) in [
        Activate::Load,
        Activate::Save,
        Activate::Delete,
        Activate::Close,
        Activate::NewGame,
    ]
    .into_iter()
    .enumerate()
    {
        let mut prefs = Prefs::default();
        prefs.escape();
        prefs.keyboard = true;
        prefs.selected = 6 + index;
        assert_eq!(prefs.key(prefs::Key::Enter), action);
    }
}

#[test]
fn graphic_page_sets_the_space_colour_and_the_stored_shape_detail() {
    let mut prefs = Prefs::default();
    prefs.escape();
    prefs.page = GRAPHIC;
    // `DATA_004903`: shape detail at y 24..54, space colour at y 80..120.
    for (y, level) in [(24, 0), (34, 1), (44, 2), (54, 3)] {
        prefs.click(34, y);
        assert_eq!(prefs.shape_detail, level);
    }
    for (y, colour) in [(80, 0), (90, 1), (100, 2), (110, 3), (120, 4)] {
        prefs.click(34, y);
        assert_eq!(prefs.space_colour, colour);
        assert_eq!(
            prefs::space_colour_rgb(colour),
            [0x00_0000, 0x04_0414, 0x08_0828, 0x08_0838, 0x0c_0c4c][colour]
        );
    }
    // `0x9e`: Main Menu.
    assert_eq!(prefs.click(34, 136), Activate::None);
    assert_eq!(prefs.page, MAIN);
}

#[test]
fn others_page_applies_stars_and_reverse_and_keeps_the_rest_inert() {
    let mut prefs = Prefs::default();
    prefs.escape();
    prefs.page = OTHERS;
    // `DATA_004904`: rows at y 28..118. Background stars (index 1) and the two
    // reverse options (3, 4) are the port's applied settings.
    assert!(prefs.others[1]);
    prefs.click(34, 38);
    assert!(!prefs.others[1]);
    prefs.click(34, 38);
    assert!(prefs.others[1]);
    // Lock external view to ship (index 2) is applied through ViewState.
    assert!(prefs.others[2]);
    prefs.click(34, 48);
    assert!(!prefs.others[2]);
    prefs.click(34, 58);
    assert!(prefs.others[3]);
    prefs.click(34, 68);
    assert!(prefs.others[4]);
    // Space dust (index 0) now gates F1035's spawn; elite control (5) and
    // video (9) remain inert.
    assert!(prefs.others[0]);
    prefs.click(34, 28);
    assert!(!prefs.others[0]);
    prefs.click(34, 28);
    assert!(prefs.others[0]);
    for (y, index) in [(78, 5), (118, 9)] {
        let before = prefs.others[index];
        prefs.click(34, y);
        assert_eq!(prefs.others[index], before);
    }
}

#[test]
fn maps_stay_inert_and_the_sound_rows_apply_the_originals_switches() {
    let mut prefs = Prefs::default();
    prefs.escape();
    prefs.page = MAPS;
    for (index, label) in prefs::MAP_LABELS.iter().enumerate() {
        assert_eq!(prefs.rows()[index].label, *label);
        assert!(prefs.rows()[index].mark == prefs::Mark::Toggle(true));
        assert!(!prefs.rows()[index].applied);
    }
    prefs.click(34, 28);
    assert_eq!(prefs.maps, [true; 5]);
    prefs.page = SOUND;
    for (index, label) in prefs::SOUND_LABELS.iter().enumerate() {
        assert_eq!(prefs.rows()[index].label, *label);
        // The port now applies all ten Sound Effects switches.
        assert!(prefs.rows()[index].applied);
    }
    prefs.click(34, 28);
    assert_eq!(
        prefs.sound,
        [false, true, true, true, true, true, true, true, true, true]
    );
    assert_eq!(prefs.texts().iter().filter(|t| t.x == 160).count(), 16);
}

#[test]
fn port_page_switches_modern_and_classic_controls() {
    let mut prefs = Prefs::default();
    prefs.escape();
    prefs.page = PORT;
    assert!(!prefs.modern);
    assert_eq!(prefs.click(34, 48), Activate::None);
    assert!(!prefs.modern);
    assert_eq!(prefs.click(34, 38), Activate::None);
    assert!(prefs.modern);
}

#[test]
fn the_port_page_flips_the_preset_and_shows_it() {
    use ffe_rs::input::Preset;
    use ffe_rs::prefs::Mark;

    let mut prefs = Prefs::default();
    assert_eq!(Preset::from_modern(prefs.modern), Preset::Classic);
    prefs.toggle_modern();
    assert_eq!(Preset::from_modern(prefs.modern), Preset::Modern);
    prefs.toggle_modern();
    assert_eq!(Preset::from_modern(prefs.modern), Preset::Classic);
    prefs.escape();
    prefs.page = PORT;
    let rows = prefs.rows();
    let modern = rows.iter().find(|row| row.label == "Modern").unwrap();
    let classic = rows.iter().find(|row| row.label == "Classic").unwrap();
    assert_eq!(modern.mark, Mark::Radio(false));
    assert_eq!(classic.mark, Mark::Radio(true));
    // The page switches the same value back immediately.
    assert_eq!(prefs.click(34, 38), Activate::None);
    assert!(prefs.modern);
}

#[test]
fn display_page_sets_the_port_only_presentation_options() {
    let mut prefs = Prefs::default();
    prefs.escape();
    prefs.page = DISPLAY;
    assert_eq!(prefs.display, DisplaySettings::default());
    let labels: Vec<&str> = prefs.rows().iter().map(|row| row.label).collect();
    assert!(labels.contains(&"4x"));
    assert!(labels.contains(&"5x"));
    assert!(!labels.contains(&"6x"));
    assert!(labels.contains(&"Pixel-perfect"));
    assert!(labels.contains(&"Nearest"));
    assert!(labels.contains(&"xBRZ 2x"));
    assert!(!labels.contains(&"xBRZ 3x"));
    assert!(labels.contains(&"4:3 DOS"));
    assert!(labels.contains(&"Fullscreen"));
    let rows = prefs.rows();
    assert_eq!(
        rows.iter().find(|row| row.label == "Fullscreen").unwrap().y,
        86
    );
    assert_eq!(
        rows.iter().find(|row| row.label == "Scale2x").unwrap().y,
        96
    );
    assert_eq!(
        rows.iter().find(|row| row.label == "Widescreen").unwrap().y,
        126
    );
    prefs.page = MAIN;
    let main_rows = prefs.rows();
    assert_eq!(
        main_rows
            .iter()
            .find(|row| row.label == "Advanced Options")
            .unwrap()
            .colour,
        15
    );
    assert!(!prefs
        .texts()
        .iter()
        .any(|text| text.text.contains("Yellow:")));
    prefs.page = PORT;
    assert_eq!(
        prefs
            .texts()
            .iter()
            .find(|text| text.text == "Advanced options:")
            .unwrap()
            .colour,
        16
    );
    prefs.page = DISPLAY;
    // 3x, fill window, linear, square pixels.
    prefs.click(34, 56);
    prefs.click(178, 46);
    prefs.click(178, 80);
    prefs.click(178, 126);
    assert_eq!(
        prefs.display,
        DisplaySettings {
            scale: 3,
            fit: FitMode::FillWindow,
            filter: Filter::Linear,
            aspect: PixelAspect::Wide,
            scanlines: false,
            scanline_intensity: 0.3,
        }
    );
    // The remaining upscalers are selectable from their radio rows.
    prefs.click(178, 86);
    assert_eq!(prefs.display.filter, ffe_rs::display::Filter::XbrzX2);
    prefs.click(178, 96);
    assert_eq!(prefs.display.filter, ffe_rs::display::Filter::Scale2x);
    // The optional scanline row reveals its intensity control and cycles it.
    prefs.click(34, 96);
    assert!(prefs.display.scanlines);
    assert!(prefs.rows().iter().any(|row| row.label == "Intensity"));
    prefs.click(34, 106);
    assert_eq!(prefs.display.scanline_intensity, 0.5);
    // Out-of-range scale rows do nothing.
    assert_eq!(prefs.apply(prefs::Action::WindowScale(0)), Activate::None);
    assert_eq!(prefs.display.scale, 3);
    // The Main Menu row returns to the original page list.
    assert_eq!(prefs.click(34, 136), Activate::None);
    assert_eq!(prefs.page, MAIN);
    // Port Options opens the display page from its port-only row at y 104.
    prefs.page = PORT;
    assert_eq!(prefs.click(34, 104), Activate::None);
    assert_eq!(prefs.page, DISPLAY);
}

#[test]
fn keyboard_navigation_activates_rows_and_escape_closes() {
    let mut prefs = Prefs::default();
    assert_eq!(prefs.escape(), Activate::None);
    assert!(prefs.open);
    // Six rows down from Graphic Detail reaches Quit; Enter reports the exit.
    for _ in 0..5 {
        assert_eq!(prefs.key(Key::Down), Activate::None);
    }
    assert_eq!(prefs.rows()[prefs.selected].label, "Quit");
    assert_eq!(prefs.key(Key::Enter), Activate::Quit);
    // Backspace returns to the main page; Escape closes the pages.
    prefs.page = GRAPHIC;
    prefs.key(Key::Backspace);
    assert_eq!(prefs.page, MAIN);
    assert_eq!(prefs.key(Key::Escape), Activate::Close);
    assert!(!prefs.open);
}

fn screen() -> Screen {
    let mut font = vec![0xff; 1160];
    for width in font.iter_mut().skip(9).step_by(10) {
        *width = 5;
    }
    let panel = IndexedImage::new(320, 43, [0, 0], false, &vec![2; 320 * 43]).unwrap();
    let palette = std::array::from_fn(|index| index as u32);
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 320, 158);
        encoder.set_color(png::ColorType::Rgb);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&vec![100; 320 * 158 * 3]).unwrap();
    }
    Screen::from_assets(&bytes, &font, panel, palette).unwrap()
}

#[test]
fn the_preference_frame_uses_the_originals_grey_and_keeps_the_console() {
    let mut screen = screen();
    let prefs = Prefs::default();
    screen.draw_prefs(&prefs);
    let pixels = screen.logical_pixels();
    // The menu background is palette 3, the original's (3,3,3) clear colour.
    assert_eq!(pixels[0], 3);
    assert_eq!(pixels[100 * 320 + 300], 3);
    // The console panel below the 158-row view stays as the flight screen has it.
    assert_eq!(pixels[158 * 320], 2);
    // The title and a label are drawn in the original colours.
    let view = &pixels[..320 * 158];
    assert!(view.contains(&16), "the red title");
    assert!(view.contains(&15), "the white labels");
    assert!(view.contains(&17), "the green commander line");
    assert!(!view.contains(&18), "the menu has no yellow port-only rows");
}

#[test]
fn the_save_and_load_lists_match_the_preference_pages() {
    let mut menu = screen();
    menu.draw_prefs(&Prefs::default());
    let menu_pixels = menu.logical_pixels().to_vec();

    let mut list = screen();
    let rows = vec![
        ("quick.json".to_owned(), true),
        ("save-0001.json".to_owned(), false),
    ];
    list.draw_pref_list(
        "Load Commander",
        "Saved commanders:",
        &rows,
        "Esc to go back",
    );
    let pixels = list.logical_pixels();
    // The same palette-3 upper view and the same console below it.
    assert_eq!(pixels[0], 3);
    assert_eq!(pixels[100 * 320 + 300], 3);
    assert_eq!(pixels[158 * 320], menu_pixels[158 * 320]);
    // Not the navigation list's flat colour-1 panel.
    let view = &pixels[..320 * 158];
    assert!(!view.contains(&1), "no navigation-list background");
    assert!(view.contains(&16), "the red sub-heading");
    assert!(view.contains(&15), "the white labels");
    assert!(view.contains(&17), "the green heading");
}

#[test]
fn the_flight_background_is_the_original_space_colour() {
    let mut screen = screen();
    let state = Flight::free_flight();
    let view = 320 * 158;
    // The captured original: the default "Light Blue" is palette entry 182,
    // six-bit (3,3,19).
    assert_eq!(screen.sky, 0x0c_0c4c);
    for (index, expected) in [
        (4, 0x0c_0c4c),
        (0, 0x00_0000),
        (2, 0x08_0828),
        (3, 0x08_0838),
        (1, 0x04_0414),
    ] {
        screen.sky = prefs::space_colour_rgb(index);
        screen.draw(
            &state,
            ffe_rs::camera::ViewState::default(),
            None,
            true,
            "test",
        );
        let count = screen.logical_pixels()[..view]
            .iter()
            .filter(|pixel| **pixel == expected)
            .count();
        assert!(
            count > view / 2,
            "space colour {index}: {count} of {view} pixels"
        );
    }
}

/// The Graphic Detail page's "Black" fills the sky with true black. The stars drawn over it are
/// F1775's model-0x13b background, which needs the content cache and a scene; the local
/// `flight_background_model315_reaches_frame` test in `effects_view.rs` checks that they still
/// show over black space.
#[test]
fn the_black_space_colour_fills_the_sky() {
    let mut screen = screen();
    let state = Flight::free_flight();
    let view = 320 * 158;
    assert_eq!(prefs::space_colour_rgb(0), 0x00_0000);
    screen.sky = prefs::space_colour_rgb(0);
    screen.stars_visible = true;
    screen.draw(
        &state,
        ffe_rs::camera::ViewState::default(),
        None,
        true,
        "test",
    );
    let black = screen.logical_pixels()[..view]
        .iter()
        .filter(|pixel| **pixel == 0)
        .count();
    assert!(black > view / 2, "black sky: {black} of {view} pixels");
}

/// A selection made on the Graphic Detail page survives closing and
/// re-opening the pages in the same session.
#[test]
fn the_space_colour_persists_within_the_session() {
    let mut prefs = Prefs::default();
    assert_eq!(prefs.space_colour, 4);
    prefs.escape();
    prefs.page = GRAPHIC;
    // The existing "Black" row at y 80 (`DATA_004903`'s 0x85).
    assert_eq!(prefs.click(34, 80), Activate::None);
    assert_eq!(prefs.space_colour, 0);
    // Return to Game, then Escape again: the page state and the choice remain.
    assert_eq!(prefs.click(34, 136), Activate::None);
    assert_eq!(prefs.page, MAIN);
    prefs.key(Key::Escape);
    assert!(!prefs.open);
    prefs.escape();
    prefs.page = GRAPHIC;
    assert_eq!(prefs.space_colour, 0);
    let rows = prefs.rows();
    let black = rows.iter().find(|row| row.label == "Black").unwrap();
    let blue = rows.iter().find(|row| row.label == "Light Blue").unwrap();
    assert_eq!(black.mark, prefs::Mark::Radio(true));
    assert_eq!(blue.mark, prefs::Mark::Radio(false));
}

#[test]
fn the_save_name_box_matches_the_lists_and_keeps_json_fixed() {
    let mut list = screen();
    list.draw_pref_list("Load Commander", "Saved commanders:", &[], "Esc to go back");
    let list_pixels = list.logical_pixels().to_vec();

    let mut boxed = screen();
    boxed.draw_pref_textbox(
        "Save Commander",
        "Name for the new save:",
        "commander-00000006",
        ".json",
        true,
        "Type a name; Enter saves, Esc goes back",
    );
    let pixels = boxed.logical_pixels().to_vec();
    // The same upper-view colour and console as the list pages.
    assert_eq!(pixels[0], 3);
    assert_eq!(pixels[158 * 320], list_pixels[158 * 320]);
    // The box has a white border and a black inside.
    assert_eq!(pixels[30 * 320 + 32], 15);
    assert_eq!(pixels[37 * 320 + 33], 0);
    // The name is drawn white and the caret yellow, then the fixed ending.
    let row = |y: usize| &pixels[y * 320 + 34..y * 320 + 286];
    assert!((33..44).any(|y| row(y).contains(&15)), "name text");
    assert!((33..44).any(|y| row(y).contains(&18)), "caret");

    // A longer name pushes the ending along; the ending never moves into the name.
    let mut longer = screen();
    longer.draw_pref_textbox(
        "Save Commander",
        "Name for the new save:",
        "commander-000000066",
        ".json",
        false,
        "",
    );
    assert_ne!(longer.logical_pixels(), &pixels[..]);
}

/// The original's warning strings carry `0x01 0x02 WARNING: 0x01 0x03 text`: the prefix draws in
/// DATA_007778's red (0x10) and the text in DATA_007779's 0x5f, not in the entry's white.
#[test]
fn a_warning_message_draws_its_prefix_in_red() {
    let mut screen = screen();
    screen.messages_visible = true;
    screen.clock = "1-Jan-3250 12:00:00".to_owned();
    screen.messages.push(
        "WARNING: Incoming Missile",
        "1-Jan-3250 12:00:00".to_owned(),
    );
    screen.draw(
        &Flight::free_flight(),
        ffe_rs::camera::ViewState::default(),
        None,
        false,
        "",
    );
    // The message window: x 195..320, y 158..176.
    let window: Vec<u32> = (158..176usize)
        .flat_map(|y| (195..320usize).map(move |x| (x, y)))
        .map(|(x, y)| screen.logical_pixels()[y * 320 + x])
        .collect();
    assert!(window.contains(&0x10), "the red WARNING: prefix");
    assert!(window.contains(&0x5f), "the message text");
}

/// F271 appends a repeated discrete event (each missile launch warns again); a repeated
/// state-derived line is still ignored.
#[test]
fn a_repeating_event_posts_again_but_a_repeated_state_line_does_not() {
    use ffe_rs::messages::{Channel, Event, MessageLog, Post, Source};
    let mut log = MessageLog::new();
    let mut channel = Channel::default();
    let warn = || Event::repeating(Source::Combat, "WARNING: Incoming Missile");
    assert_eq!(channel.post(&mut log, warn(), "1-Jan-3250"), Post::Added);
    assert_eq!(channel.post(&mut log, warn(), "1-Jan-3250"), Post::Added);
    assert_eq!(log.len(), 2, "F271 appends each missile warning");
    let line = || Event::new(Source::Autopilot, "Autopilot engaged.");
    assert_eq!(channel.post(&mut log, line(), "1-Jan-3250"), Post::Added);
    assert_eq!(channel.post(&mut log, line(), "1-Jan-3250"), Post::Ignored);
    assert_eq!(log.len(), 3);
}
