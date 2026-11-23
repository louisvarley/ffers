//! Local-only replay of F152's whole-second events under time acceleration
//! (PAR-01, PAR-12).
//!
//! `FFE_RESERVE_DUMP=1 FFE_ACCEL_AT=200:1,260:2,320:3,380:4,440:0 jjffe-smoke
//! --flight idle 520` runs the new game parked on its pad and applies F208's own
//! writes at the end of the named frames: 10x (frame 200), 100x (260), 1000x (320),
//! 10000x (380), then back to 1x (440). The dump's `tics` is each frame's length
//! (1,043 at 1x, 10x/100x/1000x/10000x of that when accelerated) and `marker` is
//! `DATA_008808` (state offset 8), which F152 moves by exactly the whole events it
//! dispatches and F208 moves back one event on a 10x-or-faster selection.
//!
//! The port's [`SecondEvents`] given the same tics and the same selections must
//! dispatch the same number of events every frame, the 43,200-event first
//! dispatch at frame 162 and F208's pulled-back event included.
//!
//! Run with `FFE_ACCEL_TRACE=<stderr of that run>` and `FFE_ACCEL_AT` set to the
//! same list.

use ffe_core::second_events::SecondEvents;
use serde::Deserialize;

const TICS_PER_EVENT: u32 = 0xc22e;

#[derive(Deserialize)]
struct Row {
    reserve: u64,
    tics: u32,
    marker: u32,
}

#[test]
#[ignore = "Local-only: needs FFE_ACCEL_TRACE and FFE_ACCEL_AT"]
fn the_event_scheduler_follows_the_original_through_every_acceleration_step() {
    let rows: Vec<Row> = std::fs::read_to_string(std::env::var_os("FFE_ACCEL_TRACE").unwrap())
        .unwrap()
        .lines()
        .filter(|line| line.starts_with("{\"reserve\""))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let selections: Vec<(u64, usize)> = std::env::var("FFE_ACCEL_AT")
        .unwrap()
        .split(',')
        .map(|pair| {
            let (frame, step) = pair.split_once(':').unwrap();
            (frame.parse().unwrap(), step.parse().unwrap())
        })
        .collect();
    let mut events = SecondEvents::new_game();
    let mut previous: Option<&Row> = None;
    let mut compared = 0;
    let mut dispatched = vec![0u64; 5];
    let mut step = 0usize;
    for row in &rows {
        let before = previous;
        previous = Some(row);
        if row.reserve < 162 {
            continue;
        }
        // The frame's F152 dispatch first, then F208's write at its end (the
        // dump is taken after both, so a selection frame's marker also shows
        // the pull-back: one event fewer).
        let port = events.advance(row.tics);
        let selected = selections.iter().find(|(frame, _)| *frame == row.reserve);
        let pulled_back = selected.is_some_and(|(_, chosen)| (1..=4).contains(chosen));
        if let Some((_, chosen)) = selected {
            events.select_acceleration(*chosen);
        }
        let Some(before) = before else {
            if let Some((_, chosen)) = selected {
                step = *chosen;
            }
            continue;
        };
        let mut delta = row.marker.wrapping_sub(before.marker);
        if pulled_back {
            delta = delta.wrapping_add(TICS_PER_EVENT);
        }
        assert_eq!(
            delta % TICS_PER_EVENT,
            0,
            "frame {}: marker is not on an event",
            row.reserve
        );
        let original = delta / TICS_PER_EVENT;
        assert_eq!(
            port, original,
            "frame {}: events at acceleration step {step}",
            row.reserve
        );
        dispatched[step] += u64::from(port);
        compared += 1;
        if let Some((_, chosen)) = selected {
            step = *chosen;
        }
    }
    eprintln!("{compared} frames compared; events dispatched per step (1x, 10x, 100x, 1000x, 10000x): {dispatched:?}");
    assert!(compared > 300);
    assert!(
        dispatched[1..].iter().all(|n| *n > 0),
        "every acceleration step must dispatch events"
    );
}
