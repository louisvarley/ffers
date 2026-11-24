# Adding a toggle-gated feature (enhancements how-to)

Owner request (2026-11-23): new features (the enhanced galaxy map, multi-step
jumps, …) must land as **toggle additions** — a launcher switch, save-safe and
mid-game toggleable — not as always-on changes to the like-for-like port. This
page is the recipe for wiring one up. The existing items and the rules they obey
live in [`enhancements.md`](enhancements.md); read that first.

## The rules (why this shape)

1. Every enhancement is a clear **on/off** toggle.
2. Toggling mid-game, or saving/loading, never changes or corrupts game state.
   The behaviour is applied at a safe boundary (input/frame/render) and its
   state lives in `saves/config.cfg`, never in a commander save.
3. Defaults are decided per item: bug fixes may default on, balance/QoL items
   off, but each stays individually toggleable.

Because the toggle is read from `config.cfg` at the point the behaviour runs,
turning it off always reproduces the original path byte-for-byte.

## The checklist (in order)

Adding `enhancement-N` takes seven edits across four crates/documents, plus a
regression test. Work top to bottom; each step keeps the whole thing compiling.

### 1. The setting — `crates/ffe-rs/src/settings.rs`

Add one `pub <name>: bool` field to the `Enhancements` struct, then update the
three places that list the fields in order:

- `Enhancements::KEYS` — add the config-file suffix (e.g. `"enhanced_galaxy_map"`).
- `Enhancements::slot` — add the `index => &mut self.<name>` arm.
- `Enhancements::values` — add `self.<name>` to the returned array.

```rust
pub struct Enhancements {
    // ...
    /// `enhancement-12`: the enhanced (flat) galaxy map.
    pub enhanced_galaxy_map: bool,
}

pub const KEYS: [&'static str; 12] = [
    // ...
    "enhanced_galaxy_map",
];

pub fn slot(&mut self, index: usize) -> Option<&mut bool> {
    Some(match index {
        // ...
        11 => &mut self.enhanced_galaxy_map,
        _ => return None,
    })
}

fn values(&self) -> [bool; 12] {
    [
        // ...
        self.enhanced_galaxy_map,
    ]
}
```

`to_config`/`set` iterate `KEYS`/`values`, so the key is written as
`enhancement.enhanced_galaxy_map=on|off` and read back automatically. Bump the
array length literals (`[bool; N]`, `[&'static str; N]`) to match.

### 2. The launcher row — `crates/ffe-launcher/src/main.rs`

Add an `Enhancement` entry to a group in `ENHANCEMENT_GROUPS` (or add a new
group). `slot` is the index from step 1.

```rust
Enhancement {
    slot: 11,
    title: "Enhanced galaxy map",
    about: "Replaces the 3D star map with a flat, clearer map: multi-step X \
            routes, hover station info, SOL indicator, middle-mouse zoom and \
            right-drag pan.",
},
```

The launcher reads `settings::Enhancements::KEYS` for the order and writes
`config.cfg` with the game's own reader/writer, so other settings are preserved.

### 3. The game side — the behaviour gate

Read `config.enhancements.<name>` at the boundary where the behaviour lives
(frame/input/render) and branch: the **off** path must be the existing code
untouched. In the run loop (`main.rs`) or the feature's module:

```rust
if config.enhancements.enhanced_galaxy_map {
    // new behaviour
} else {
    // existing behaviour, unchanged
}
```

For a large feature (a replacement map screen) prefer a new module
(`galaxy_map2.rs`, `map_routes.rs`, …) so the legacy path stays isolated and
diffable, and `main.rs` only switches which one it calls.

### 4. Document the item — `docs/notes/enhancements.md`

Add the `enhancement-N` heading, the toggle title, and what "off" means, then
add a row to the `Implemented` table (`Item | Where | Behaviour when off`).

### 5. Record the deviation — `docs/DEVIATIONS.md`

Every enhancement is a deliberate difference from the source, so add a `EXP-xxx`
row marked `enhancement` explaining what changed and that it is toggle-gated.

### 6. The regression test

Add a test that (a) the on-path behaves as intended and (b) the off-path is the
original. Keep simulation-relevant tests in `ffe-core`; presentation/UI tests in
`ffe-rs` (`--lib` or a `tests/*.rs` integration test, cache-gated with
`-- --ignored` where the world cache is required).

### 7. README / CONTROLS

If the feature adds or changes keys or options, update
[`docs/CONTROLS.md`](../CONTROLS.md) (keys) and the launcher/README mention so
owners can find the toggle.

## Example: a save-safe flag is read, never stored

An enhancement is **never** written into a commander save. `settings::load_config`
fills `Config::enhancements` at start-up and the in-game options path
(`config_from_prefs`) copies the block through, so saving a game or changing
options never clears the launcher's choices. State the feature *does* need across
save/load must be derived or recomputed, not persisted as part of the toggle.

## Sign-off

After the checklist, run the smallest verification (a focused `cargo test`) and,
at the completion gate, `cargo fmt --check` and `cargo clippy` on the changed
files. Record the item in [`WORK_HANDOFF.md`](../WORK_HANDOFF.md) and update
[`dashboard/burndown.js`](../../dashboard/burndown.js) in the same commit if the
milestone totals change.
