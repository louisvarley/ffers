# The 3D system map (module 5, F963-F986)

Added 2026-11-19 (`ui-29`, `ui-30`) after the owner's capture of the original's page: the
cabin with green orbit curves, white body names, a "Time point: 1-Jan-3250 11:02:00" line and
the console date. Reworked 2026-11-20 to the original's own view and checked against the executed
original. Code: `crates/ffe-rs/src/system_map.rs` (the page's model), `Screen::draw_system_map`,
and the input block in `main.rs` beside the galaxy map's.

## Where it sits in the original

`DATA_007239` is the event table of module 5 (`DATA_004892`'s sixth entry): F963 enter, F966 re-enter
this system, F982 keys and mouse, F977 draw, F985 time step, F964 leave. No console key reaches
module 5 directly; the galaxy map's handler (F840) does:

| Galaxy map input | Original | System map |
|---|---|---|
| **F6** (and the console's 0x23 icon), from any page of the map | `JUMP_003436` -> F966 | this system, time point = the game clock at that moment |
| **`n`, `r`** with a star selected | `JUMP_003438` -> F974 | the selected star's system, generated for now (slider cubed) |

F982 hands F5-F10 back to the galaxy handler, so **F5 and F7-F10** close the page and act as they
do on the galaxy map. F6 on the page resets the time point and the focus.

## The view (F963, F969, F982, F984, F1676, F1537/F1680/F1479)

Read from the source and from the executed original (`--game-frame 560 0x3d 0x40` with
`FFE_MAP_DUMP`, `tools/reference/verify-system-map.py`):

- The camera is an object (`DATA_008968`) whose **parent is the focus object** (`DATA_008979`, so its
  position is relative to the focus). Its matrix is F1676 `MatBuildYZ(a, b)` with `a = y accumulator << 6`
  and `b = -(x accumulator << 6)` (16-bit binary angles); F969 starts both accumulators at 0x100, a
  quarter turn each, which looks straight down the system's y axis (rows `(0,0,-1)`, `(1,0,0)`, `(0,-1,0)`).
- The camera stands back along the matrix's third row: `-mantissa * (row >> 16) << (exponent - 31)`, i.e.
  `mantissa * 2^(exponent - 16)` millimetres. The mantissa and exponent are an ffp number
  (`DATA_008984`/`5`).
- **F963** (new game or load) sets it to `0x7fff` at exponent `0x2a` (2.2e12 mm), whatever the system, and
  the focus to slot `0x6c`, `0x6f`, `0x60` or `0x70` for start types 0, 1/4+, 2 and 3 (`arg >> 8`).
  F964 keeps the focus for the next visit (`DATA_008983`); F974 (another system) starts at slot `0x72` and
  F968 steps down to a drawn object. `View` carries both between visits.
- F1537 projects every drawn object (flag 8) through F1680 and F1479 (`160 + 256 x / z`, `79 - 256 y / z`,
  integer division truncating toward zero): far objects become F1700 dot sprites, nearer ones models.
  The port runs the same `ffe_core::model_animation::setup_object` and draws the same sprites
  (`ModelViewer::object_dot_frame`); `Dot::sprite` carries them. It needs cache v35's object-dot tables.
- **F11/F12** (held, once a frame) multiply the distance by `0x7998` at exponent 0 (about 0.95) and `0x435d`
  at exponent 1 (about 1.05). F11 stops at F984's size of the focus's model plus ten
  (`ModelViewer::model_distance_exponent`), F12 at exponent `0x3f`.

## What the page does (F977, F978, F980-F986)

- **Time point** (`DATA_007243`, F977's string 0, white at (10,148)): the date at the 64-bit time
  stored at `+0x400`, copied from the game clock when the page opens (F969), not kept current.
- **Slider** (action `0x80`, `DATA_007241`'s hot area (268,156)-(319,177)): a click at x moves the time
  point by `|x - 0x12c|^2` (this system) or `^3` (another) `<< 18` in a clock of 2^32 to the day, back when
  x is left of `0x12c`. Left/Right step it while held (the original's keys 3 and 4,
  `DATA_009122 << 13`, about 2.3 hours a second).
- **Names** (F975/F978): stars, planets and moons only (`+0x87 == 1`), white, written at the dot's
  (x + 3, y - 4); dropped when x + 3 reaches `0x13c` or y - 4 reaches `0x95`, and when the parent's dot is
  within ten pixels (sum of the offsets). The picked body is marked.
- **Rings** (F981/F986): for each such object with a parent that is not a starport, the ring model `0x96`
  stands at the parent's position with `+0x7e` = the orbit radius's ffp exponent less `0x19` and is a
  far object like any other, so F1680 drops it past its far class. The port draws F1507's circle (the
  plane through the system's z axis and the body) as a polyline when `setup_object` would draw the ring.
- **Click** (F983): the body whose point is within 8 pixels (sum of the offsets) becomes the target
  (`Navigator::lock`). After **`t`** (F982's `0x74`, which logs "Click on a target or destination for
  autopilot", `DATA_007244`) the click engages the autopilot to it (`Navigator::engage_body`) and the
  map closes. Another star's system can be looked at but not flown to.

## Port's own parts (see EXP-025 in `DEVIATIONS.md`)

- The ring is a polyline in a flat green, not the model; a ring thousands of pixels across is dropped
  (the original draws none either, though F1680's far class would allow it).
- The right mouse button turns the view (added to the accumulators), Page Up/Down or +/- zoom smoothly,
  and a press of F11/F12 is five steps.
- **C** (or a second click on a picked body) centres on that body and refits the distance to what orbits
  it; **Home** returns to the star. The original never changes the focus on the page.
- The picked body's facts (name, kind, orbit radius, radius, range) are written over the lower left of
  the view.

## Not done

F986's ring model (so the ring colour, the original's greens 184-191, and the orbit plane differ by a
few pixels), the planet model when the camera is close enough for F1680 to draw one (a disc is drawn),
the slider's pressed-arrow redraw (`BlitClipIndexToBuf`), and the console message window for `t` going
through F271's replace flag exactly. The dot sprites take their tint from the viewer's shade table,
which is not set to the system's star here.

## Evidence

`tools/reference/verify-system-map.py` against the executed original at the Ross 154 start (the page
at the game clock 1.68 s after the start): focus slot 111, zoom `0x7fff`/`0x2a`, camera `(0, 0x1fffc000000, 0)`
and all six bodies' screen positions equal; the three names at the same places; 837 of the original's
977 overlay pixels shared. `system_map::tests` (12) and `tests/system_map_view.rs` (local: Gateway's
orbits, names and time point, a slider step, a pick, a click and `t`; `the_page_draws_at_its_start` for
the oracle; `FFE_SHOT_DIR` writes the frames).
