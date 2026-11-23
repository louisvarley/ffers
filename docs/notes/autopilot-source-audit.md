# Autopilot source audit — 26 October 2026

The earlier coordinate fix was not a full autopilot fix. The old navigation
controller was a developer substitute, despite several comments describing
original routine names. This audit follows the player entry points, guidance,
physics integration, station approach, clearance, final leg, disengagement,
clock changes and persistence boundaries. JJFFE 2.8a7 `ffe.asm` is the authority;
the recovered `srctools/cphys.txt` notes contain mistakes (including F589 turn
signs) and are not an executable specification.

## What changed

- The player loop calls `Navigator::source_command` rather than the old
  developer controller. The latter is explicitly named `prototype_autopilot` /
  `prototype_command`; its old tests are developer-scenario tests, not parity
  evidence. Player flight no longer uses its fixed 90 m/s² braking envelope,
  direct `turn_toward`, long-leg position transfers or estimated clock jumps.
- `ffe-core::autopilot` implements F597's ordinary integer request/cap branch,
  F612 velocity normalisation, F587 turn words, F589 same-frame orientation
  matching and F610's large-body radius subtraction. It uses existing F599
  limit clamping; damaged/changed ship limits apply to the actual request.
- `OriginalFlight` applies thrust/caps/turn words through F727's order: F603
  gravity, F601 acceleration, F585 with the AI flag, F1512 position. Manual
  set-speed processing does not overwrite the AI request. F1018 restores the
  actual speed on disengagement.
- F1020's equipment bit 21 and flying-state gates are applied. A landed player
  cannot create a flight controller by clicking the list or pressing K.
- F741 approach offsets use the Q31 station row and signed offset words. An
  indoor station uses its source literal z offset, not an invented docking-axis
  waypoint.
- F745 uses the real fine ledger for both indoor and open-air ports. Refusal
  keeps the approach active rather than reporting arrival; unchanged refusal
  notices no longer flood the log. Lost clearance returns to approach.
- Final guidance uses F655/F650's pad **plane**, quantised as F745's signed
  words. The launch seat adds a ship radius and was therefore the wrong target.
  F746's -2 guidance modifier and F589 orientation matching are applied.
- At accelerated near-pad completion F746/F744/F652 must accept the allocated
  bay, then F671 seats the ship. A focused near-pad test checks this source
  branch. It does not establish a complete accelerated route.
- The body adapter retains the original velocity words and uses F594's frame
  rule where the navigator can represent it. All position boundaries retain the
  earlier absolute-position correction.

## Executed evidence

`tools/reference/verify-autopilot.py --reference PATH_TO_JJFFE_SMOKE` runs
original F597/F587/F589 and compares 512 deterministic cases with Rust. The
reference checks all requests, velocity caps and turn words, covering negative
and positive directions, behind-target turns, distance/velocity shifts and
modifiers. The F589 test exposed the wrong signs in the explanatory notes;
the implementation was corrected against assembly and the executed routine.
This verifies the ordinary F597 branch and **same-frame** F589, not every AI
routine. No original source or data files were changed.

`crates/ffe-rs/tests/autopilot.rs` checks real thrust/braking, no instantaneous
turn on engagement, clearance/fines/retry notices, continued pad guidance,
accelerated near-pad completion, and an actual pad landing at 1x. Cache tests
use `FFE_WORLD_CACHE`. The initial two-body contact adapter stopped 126 m short.
The original puts ship and starport as siblings under the planet. Installing
that three-body layout allows the 1x test to require and receive a real
F576/F575 landing flag, rather than merely asserting proximity. The game loop
uses the same layout. This proves the probed Old Blackelk approach, not all
planetary ports or rotating stations.

## Remaining source parity gaps

| Path | Status and practical limit |
| --- | --- |
| F610/F802 obstacle probing and avoidance | Not ported in the player controller. Large-body radius correction is implemented; it is not an obstacle planner. A trajectory around an intervening planet/building is not yet source-equivalent. |
| F597 accelerated near-offset frame match | Ordinary guidance is verified. The source branch which reselects the physical frame and matches the target at high acceleration still needs the complete object/frame adapter; it is not replaced by the old long-leg transfer. |
| F589 rotating-parent special branch | Same-frame arithmetic verified; rotating station frame coupling remains dependent on the unfinished flight-9 adapter. |
| Normal-speed F727/F1019 pad touch | Old Blackelk 1x approach now produces a real landing flag with the sibling-frame adapter. Other port models and rotating stations still need route traces. |
| NPC ship pursuit | The original can target ships; the current player UI still rejects a ship autopilot target. This remains outstanding. |
| AI event cadence | The source law is applied from the playable frame loop; F706/F732's complete object-event scheduling is not yet represented. Fixed-step/event parity remains flight-4/flight-8 work. |
| Whole-route parity | Existing sandbox arrival screenshots cannot prove original autopilot behaviour. Need reference traces for a complete planetary trip and rotating-station docking, including time acceleration. |

**Conclusion:** this is a source-guidance replacement and a set of concrete
approach/clearance fixes, not a claim that all autopilot source parity is done.
The remaining gaps must stay visible in the burndown.

## Replay against the executed original — 18 November 2026

`--flight dock` engages the original's autopilot on frame 1090, 1,039,351 mm out on
the Lave Station axis (station frame, nothing but z moves), and records every frame to
the clearance hand-off at 7660. `launch_replay.rs`
`the_autopilot_approach_follows_the_original_frame_by_frame` starts the port's
`Navigator::source_command` from that pose and compares every frame: the worst
difference is 0.2% of the 4.1 km leg (8 m at the end). The replay found three rules
the earlier tests could not:

- **Event cadence.** F1007 runs the AI states (F741-F746) from F152's whole-second
  event, after the flight step. F1020 only sets mode 2: the launch flight's thrust word
  (332) stays until the first event (frame 1104), then every 47-48 frames
  (1,043-tic frames, 49,710 tics per event). Thrust, caps and turn words hold between
  events. The player loop now calls the controller only after a frame that had an event.
- **The ship's modifier.** F742 hands F610 `ship+0x101`, set at creation from the x
  thrust limit (`ffe.asm:3601-3617`: 2 from 0x1329, 1 from 0xaa5, else 0; 1 for this
  ship). The port passed 0. F746's final leg is the constant -2, not -2 plus the byte.
- **The clearance event.** When F610's distance shift is 6 or less, F742 skips F599 and
  F587 and F745 only asks for clearance; the thrust stays as the last event left it.

What this does not cover: F610/F802 obstacle avoidance, the accelerated near-offset
frame match, rotating-parent F589, ship targets and the hand-off from mode 2 to 0xa
(frame 7660), which the docking-stage replay starts from.
