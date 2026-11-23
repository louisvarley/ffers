//! `enhancement-7`: the mission fixes from Jades' known-issues list.
//!
//! Each helper is called only while the launcher's
//! `enhancement.mission_fixes` toggle is on. With it off the callers keep the
//! original path exactly (no id serial, no bay eviction, no extra signal, no
//! extra text and no acceleration clamp); see `docs/DEVIATIONS.md` EXP-046.
//!
//! The fixes are:
//!
//! 1. Assassination kills are credited after a hyperspace jump: a jump no
//!    longer reports every ship it left behind as destroyed, and script ship
//!    ids continue above the old system's ids ([`removed`],
//!    [`crate::navigation::Navigator::script_ship_id`]).
//! 2. The view-change signal F1125 sets global 0 to 4 and back to 0;
//!    missions 69 and 78 read it to record how close the player is to an
//!    object. Without it the hitch-hiker run always ends "you have returned
//!    too late" ([`ffe_core::script::engine::Engine::camera_changed`]).
//! 3. F312 draws every medal bit in two columns, not only the first three
//!    ([`ffe_core::status::medal_slots`]).
//! 4. The Alioth flower delivery's hidden 3252 deadline is added to its
//!    briefing text ([`decorate_briefing`]).
//! 5. A full station frees a bay and launches its occupant when the player
//!    asks to dock ([`crate::navigation::Navigator::request_clearance`]).
//! 6. A mission dialogue holds time acceleration at 1x
//!    ([`dialogue_blocks_acceleration`]).
//! 7. A mission ship that waits in a starport's bay reserves it and boots an
//!    occupant when the port is full
//!    ([`crate::navigation::Navigator::reserve_script_bay`]).

/// The hidden deadline of the Alioth flower delivery (mission 90/91's
/// Fortress Culloden run): Turner's Requiem, about 30 December 3252. The date
/// is not in the script or its briefing; this is the UI-only statement the
/// enhancement adds.
pub const ALIOTH_DEADLINE: &str = "before 30 December 3252";

/// The message codes whose text is the Alioth flower delivery: the task line
/// (`0xa837`) and its bulletin-board advert (`0xa838`).
pub const ALIOTH_BRIEFING: [i32; 2] = [0xa837, 0xa838];

/// Fix 1: the objects that left the game since the last frame.
///
/// `previous` and `present` are script object ids (the navigator's live ships
/// and the scripts' canisters). A hyperspace jump clears the whole system:
/// the source's F909 does not report each left-behind object to F924, so a
/// mission's target is not credited as killed by the jump itself. With
/// `system_changed` the result is empty; otherwise it is `previous - present`,
/// in `previous`'s order.
pub fn removed(
    previous: &std::collections::BTreeSet<i32>,
    present: &std::collections::BTreeSet<i32>,
    system_changed: bool,
) -> Vec<i32> {
    if system_changed {
        return Vec::new();
    }
    previous.difference(present).copied().collect()
}

/// Fix 6: a mission dialogue (the scripts' menu, which carries the YES/NO
/// rows) is open while docked. The original lets time acceleration run in the
/// station, which makes the menu's input starve; the enhancement holds the
/// frame at 1x until the dialogue closes.
pub fn dialogue_blocks_acceleration(enabled: bool, docked: bool, menu_active: bool) -> bool {
    enabled && docked && menu_active
}

/// Fix 4: add the hidden Alioth deadline to the flower delivery's briefing.
/// Other codes are returned unchanged, and the text is left exactly as it is
/// when the message is not one of [`ALIOTH_BRIEFING`].
pub fn decorate_briefing(code: i32, text: String) -> String {
    if !ALIOTH_BRIEFING.contains(&code) {
        return text;
    }
    let trimmed = text.trim_end();
    format!("{trimmed} ({ALIOTH_DEADLINE})")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn a_hyperspace_jump_does_not_report_the_ships_it_left() {
        let previous = BTreeSet::from([17, 26, 40]);
        let present = BTreeSet::from([17]);
        // A normal frame: 26 and 40 left the game (destroyed or scooped).
        assert_eq!(removed(&previous, &present, false), vec![26, 40]);
        // A system change: F909 cleared the system; nothing was destroyed.
        assert!(removed(&previous, &present, true).is_empty());
        // A kill in the new system is still reported once tracking resumes.
        let present = BTreeSet::from([17, 26]);
        assert_eq!(
            removed(&BTreeSet::from([17, 26, 41]), &present, false),
            vec![41]
        );
    }

    #[test]
    fn only_a_docked_mission_dialogue_holds_time_at_one() {
        assert!(dialogue_blocks_acceleration(true, true, true));
        assert!(!dialogue_blocks_acceleration(false, true, true));
        assert!(!dialogue_blocks_acceleration(true, false, true));
        assert!(!dialogue_blocks_acceleration(true, true, false));
    }

    #[test]
    fn the_alioth_briefing_states_its_deadline() {
        let briefing = "Deliver some flowers to Fortress Culloden in Alioth (0,7)  ";
        let fixed = decorate_briefing(0xa837, briefing.to_owned());
        assert_eq!(
            fixed,
            "Deliver some flowers to Fortress Culloden in Alioth (0,7) \
             (before 30 December 3252)"
        );
        assert!(decorate_briefing(0xa838, "FRESH FLOWERS".to_owned())
            .ends_with("(before 30 December 3252)"));
        // Every other message is untouched, including one that already ends
        // in a space.
        assert_eq!(decorate_briefing(0x9c2d, "Text  ".to_owned()), "Text  ");
    }
}
