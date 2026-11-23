# Missions that cannot complete (static audit)

Status: **audited and, where the source fixes the intent, corrected
(2026-11-20, `gameplay-42`).** This note inventories entries of the original
mission-script table whose success/acceptance/reward path cannot fire, or whose
payment logic runs on the wrong state. Five sites are now corrected in the
port's decoder (`## Fixes`); the orphaned starters that the audit could not
resolve remain documented and unchanged. It does not claim that a *medium* or
*low* row is broken in play.

## Method

- The mission scripts are **not** in `game/MISSION.DAT`. `MISSION.DAT` is the
  journal text archive (671 articles). The script graph is the 57,560-byte
  region of the player's `FIRSTENC.EXE` (imported as
  `base:script/missions`; 105 mission structs, 333 reachable states, 665
  conditions). Statements are F41 opcodes, conditions F31 expressions
  (`docs/notes/mission-scripts.md`).
- Every mission was decoded with the existing interpreter
  (`crates/ffe-core/src/script.rs`, `mission_listing`/`reachable_states`), and
  the listing was checked against the raw executable bytes at the script
  region's file offset 433604, so the operand values below are data, not
  decoder artefacts.
- A mission is called **broken** only when its completion path is impossible
  under the statements in its own streams plus the engine's semantics:
  articles are only testable when a `14` (F98) statement or an F88 story
  queue produces them; menu responses only exist when a `23`/`24`/`27`/`55`/
  `56`/`59` (F345/F346/...) statement puts them on the screen; a mission
  instance only exists when `1` (F52) starts it or F55 starts mission 0
  (`ffe.asm:6332` F52; only call sites `4279` F41 statement 1 and `6558`
  F55; `ffe.asm:6541`).
- F31's issue test is token `0x20` -> F96 (`ffe.asm:11683`) -> F95
  (`ffe.asm:11612`), which walks the journal's issue list and then its queue
  looking for the article number. A publish (`F89`) only shows what was
  queued; it does not invent article numbers.
- Journal stories (`DATA_002280`, 170 entries) are a *separate* source of
  queued articles (F88). They were decoded in full; an article that appears in
  neither a `14` statement nor a story can never be in an issue.
- Script constants were decoded as the engine does: system codes from
  `x|y<<13|z<<26` with the `0x1718`/`0x1524` biases, so a target system is
  always a real coordinate (the constants below all match the coordinates in
  their own message strings).

**"Completable?"** means the mission's own success path. A mission can be
reachable and partially functional and still be listed (for example a dead
dialogue option). High confidence means the path is provably unreachable from
the script and engine data; medium means the path is reachable in principle but
its outcome depends on state a runtime probe must observe; low is recorded but
unconfirmed.

## Inventory

| Mission id / name | Broken behaviour | Source evidence (ffe.asm label/line + script token) | Confidence | Completable? |
|---|---|---|---|---|
| **78 — Turner's Quest / Argent's Quest, the 0x215–0x219 issue** | State 16's entry queues articles `0x215, 0x216, 0x218, 0x219` and queues **`0x197`** into the Imperial Herald slot `G17`; its only exit condition requires **`0x217`** in `G17`. `0x217` is queued by no `14` statement anywhere in the 57,560-byte region and is not in the 170 F88 stories, so the condition is false forever and the instance stalls in state 16. `0x217` exists in the archive (article 535, headline "'QUEST' STOLEN BY WANTED CRIMINAL", the same text as 0x197). | Condition expression at blob `0x7edd–0x7f18`: `(0x1b … (0x20 G17 0x217) …)`; entry at `0x7eac`: `14 04 11 … 02 97 01 00 00` (`G17`, `0x197`). Raw `FIRSTENC.EXE` file offset 433604+0x7eac = `0e 04 11 00 00 00 02 97 01 00 00 02 1e 00 00 00`; condition bytes `20 04 11 00 00 00 02 17 02 00 00`. Token `0x20` is F96/F95 (`ffe.asm:11683`, `11612`); queue `14` is F98 (`ffe.asm:11731`). | high | No — the branch dead-ends permanently |
| **90 — Jo Merion's message, "deliver some flowers"** | Creates script contract `L2` (`37`) and sets `G60 = 1`. The contract is removed and the mission killed only when `G60 == 2`, which only mission 91 writes. 91 never runs, so the contract row never clears and the board advert it points at never appears. | `when (0x15 G60 0x2)` body at `0x222f` -> `38 L2; 8 L0` (`0x2235`). `G60` writes: `0x202e` (89, 0), `0x21d0` (90, 1), `0x303c` (91, 2). No `1 91` byte pattern in the whole script region; F52 (`ffe.asm:6332`) is called only from F41 statement 1 and F55. | high | No |
| **91 — Fortress Culloden "FRESH FLOWERS" advert (0xa838)** | Never started by any script and therefore never advertised; it is the only writer of `G60 = 2` (see 90) and the only place message 0xa839/0xa83a can appear. | State 0 entry is empty; the body at `0x2ffb` (condition `(0x1a (0x1a (0x15 G0 0x1) (0x15 P4 0x2a51718)) (0x15 (0x38 P13) 0xf86d420))`) inserts `24 0xa838`. No `1 91` in the script blob; F52 call sites `ffe.asm:4279`, `6558`. | high | No — never runs |
| **37 — bulletin-board bounty (0xa913) offering 34/38** | Orphaned offer: no `1 37` statement exists and the game's only mission starter is F52, called only by F41 statement 1 and F55 (which starts mission 0). Its computed offer, its `0xa913` advert, and the `34`/`38` it starts therefore never happen. | Whole-region byte scan for `01 25 00 00 00` (opcode `1`, mission 37) finds nothing; its entry computes `L3..L9` and `1 34` (`0xa643`), `24 0xa913` on `G0==1` (`0xa681`). F52 `ffe.asm:6332`. | high | No — never runs |
| **38 — bounty target mission (child of 37)** | Only starter is 37, which never runs. If it did run, its entry debits `$1,000,000` immediately (`7 (0xe P2 0xf4240) => P2`, `0xa832`). | `1 38 …` only at `0xa6aa` (mission 37). F52 `ffe.asm:6332`. | high | No |
| **39 — bounty contract text (child of 38)** | Only starter is 38, which is itself unreachable. | `1 39 …` only at `0xa85a` (mission 38). | high | No |
| **55 — Dolphin transporter (Junior's friend)** | The offer's entry adds responses `0xa95f, 0xa961, 0xa965, 0xa900` but state 0 tests `G0 == 0xa963` ("When does it need to get there?"). No statement anywhere appends `0xa963`, so that question and message `0xa964` ("By [MiR(dolphin date)") can never be shown. The mission still completes through `0xa900`. | Entry at `0x0522–0x055f`: `23 0xa95f`, `23 0xa961`, `23 0xa965`, `23 0xa900`; the `G0 == 0xa963` condition's body starts at `0x05a1`. Statement 23 is F345 (`ffe.asm:26941`). | high | Yes — one dead dialogue branch |
| **20 — journal subscription purchase (all five journals)** | The advertised/checked price and the actual debit disagree by a factor of ten on three of four options: "1 issue" displays and checks `L4` but debits `10*L4`; "year" displays/checks `12*L4` but debits `120*L4`; "10 years" displays/checks `100*L4` but debits `1000*L4`. "2 years" is consistent (`22*L4`). Cash can go negative in the original; the port clamps at zero, so the mismatch is visible as an overcharge. | Entry words: `27 0xa8d6 L4`, `0xa8d7 (0xa L4 0xc)`, `0xa8d8 (0xa L4 0x16)`, `0xa8d9 (0xa L4 0x64)`; checks `(0x11 P2 …)` and debits `(0xe P2 …)` at `0x5eda/0x5efa`, `0x5f34/0x5f60`, `0x5fa0/0x5fcc`, `0x600c/0x6038`. `0xa` = multiply, `0xe` = subtract (F31, `ffe.asm:2240`). | high (arithmetic) | n/a — service dialogue, not a mission |
| **83 — Argent's Quest, post-launch state 4** | The second condition uses token `0x2b`, which F31 reads with **two** operands; the second operand swallows the intended `9 7` (`goto 7`) statement. The stream then ends, so that branch never runs. Byte-exact in the original; already noted in `mission-scripts.md`. State 4 still exits on its five-day timer to state 5. | Condition expression at `0x4239` (body at `0x4248`): `15 2b 03 04 00 00 00 02 04 00 00 00 09 07 00 00 00 00`; the `09 07 00 00 00` is consumed by the `0x2b` operands. F31 `ffe.asm:2240`; the quirk was pinned by `tests/mission_scripts.rs` (`quirky == [83]` before EXP-034). | high | Yes — one lost branch, mission continues |
| **81 — Argent's Quest hand-over reward screen (state 7)** | The reward condition is `(G0 == 2) || (G0 == 0xa841)`. `G0 == 2` is F342's "menu cleared" signal (raised by F331 when any screen is built), not a player choice, so rebuilding any screen while state 7 waits pays the reward and skips the `0xa84a` follow-up mission 83. The state's own entry pulse is not observed (F53 runs entries unsubscribed), which is why this is medium, not high. | Condition body at `0xcfee`: `(0x1b (0x15 G0 0x2) (0x15 G0 0xa841))` -> `P2 += …` (`0xd004`, `0xd032`), `9 13`. F342 `ffe.asm:26750` pulses global 0 with 2 then 0; F331 `ffe.asm:26336`. | medium | Yes — reward timing can be forced |

The "Completable?" column describes the player's original data. The port now
corrects rows 78, 55, 20, 83 and 81 at load time (`## Fixes` below); rows 37,
38, 39, 90 and 91 remain as audited.

## Fixes (2026-11-20, `gameplay-42`)

The corrections are applied in memory by `ffe_core::script::Image::new`
(`CORRECTIONS` in `crates/ffe-core/src/script.rs`). The player's
`FIRSTENC.EXE` and the cached `base:script/missions` asset keep their original
bytes; a length change in the mission-55 insertion moves the relocated
pointers after it. The regression is
`cargo test -p ffe-rs --test mission_scripts -- --ignored` with
`FFE_WORLD_CACHE` set: `the_broken_mission_data_is_corrected_in_memory` checks
both the untouched asset bytes and the corrected decoded image, and
`every_script_decodes_as_the_interpreter_reads_it` pins the new
state/condition totals and the empty quirk list.

| Mission | What was broken (source reason) | Fix applied | Deviation |
|---|---|---|---|
| **78** | State 16 queued article `0x197` into `G17`, but its exit condition tested `0x217`, which no `14` statement or F88 story queues; the instance stalled forever. | The condition operand at blob `0x7efe` is read as `0x197`, the article the entry queued. | EXP-030 |
| **55** | The offer inserted responses `0xa95f`, `0xa961`, `0xa965`, `0xa900`, but state 0 has a condition for `0xa963` (and message `0xa964`); that question could never be chosen. | Insert `23 0xa963` before `23 0xa965` at blob `0x054d`; all four questions now appear in message order. The completion path `0xa900` is unchanged. | EXP-031 |
| **20** | Three of the four journal options displayed/checked one price but debited ten times it (`L4` vs `10*L4`; `12*L4` vs `120*L4`; `100*L4` vs `1000*L4`); "2 years" alone agreed. | The debit constants at blob `0x5f08`, `0x5f6e` and `0x6046` become `1`, `12` and `100`, so every debit is the displayed `L4 / 12*L4 / 22*L4 / 100*L4`. | EXP-032 |
| **83** | The second condition in state 4 is `0x2b`, which the shipping F31 reads with **two** operands although the token record and both uses in mission 83 give one. The second read swallowed the intended `09 07 00 00 00` (`goto 7`), so state 7 was unreachable. | `EXPRESSION_SHAPES[0x2b]` reads one operand: state 4 becomes `(0x15 (0x2b P4) 0x4)` followed by `goto 7`, and state 7 (and its `!= 4` exit) become reachable. `0x2b` still has no evaluator case and returns 0, so the source's undefined *query* is not invented; only the lost statement is recovered. | EXP-034 |
| **81** | State 7's reward condition was `(G0 == 2) || (G0 == 0xa841)`; `G0 == 2` is F342's menu-cleared pulse, not a choice, so rebuilding any screen could pay the reward and skip the `0xa84a` → mission 83 follow-up. | The first disjunct is replaced with a constant false (`2 == 3`); only the `0xa841` response pays. The body is unchanged. Reachability of the spurious pulse still needs a played probe, hence the cautious fix. | EXP-033 |
| **37 / 38 / 39** | No `1 37` statement exists; 38 is only started by 37 and 39 only by 38. Mission 37 is a board advert that dies on leaving its system, so it must be started when the player is in that system, but neither the script region nor JJFFE's native F52 call sites name a starter. | **Not applied.** Adding a starter would invent gameplay not in the source. Recorded as "identified broken, starter unknown". | EXP-035 |
| **90 / 91** | 90 is reachable (mission 89 starts it and it creates the flower contract), but its contract clears only on `G60 == 2`, which only 91 writes; 91's advert waits at Fortress Culloden and no `1 91` statement or native call starts it. | **Not applied.** 90's flow is intact; 91's intended starter is not determined. Recorded as "identified broken, starter unknown"; the contract row and mission 90 therefore persist. | EXP-035 |

### Discovered while fixing

- Mission 81's states 8 and 15 use the same `(G0 == 2) || …` shape as state 7
  (their conditions `(G0 == 2) || (G0 == 0xa84f)` and
  `(G0 == 2) || (G0 == 0xa87a)`). The audit only flagged state 7, and those
  states' intended "screen gone" behavior is not established; they were left
  unchanged and are noted here rather than silently patched.

## What still needs a runtime probe

- **The corrected stories in play.** The state-16 fix removes mission 78's
  permanent stall, but only a played save can show the story now reaches states
  20 and beyond and recovers by another ending (states 33–39, reached through
  `G57 == 7`).
- **Mission 81 state 7.** A played capture is needed to show whether a normal
  player can rebuild a screen between the reward screen and the answer (the
  state-7 pulse itself is suppressed by F53), i.e. whether the spurious
  `G0 == 2` payout was reachable in practice; the port now ignores it.
- **Original-1.02-only starts.** The claim that 37/38/39 and 91 have no starter
  is proven for the imported script graph and for JJFFE's engine (F52 call
  sites). The floppy 1.02 `FIRSTENC.EXE` could in principle start one of them
  from native code that JJFFE does not have; confirming that needs a
  disassembly/probe of 1.02, not the script data.
- **Body-seed targets.** Missions 25, 32, 35, 38, 55, 69, 74–76, 81–84 and 91
  require docking at a starport/body by its `+0xa0` seed (e.g. `0x9c80553e`,
  `0x1ad02ff0`, `0x4f2018fb`, `0xf86d420`). The scripts are internally
  consistent, but whether each seed is generated in its named system can only
  be checked by generating the systems and probing the bodies.
- **Payment clamping.** The original stores cash signed; the port's
  `set_player` clamps at zero. Mission 20's corrected debit now matches its
  displayed price, so the clamp no longer shows there; mission 38's
  `$1,000,000` debit (if its starter is ever found) and the other debit-heavy
  offers still need a runtime check of the original's display when cash would
  go negative.

## Minor data notes (not counted as broken missions)

- Mission 78 state 20 queues `G19 = 0x19d` where the neighbouring blocks use
  `0x21d`; its condition also tests `0x19d`, so it is self-consistent and not
  flagged, but it requires an article already published in state 19's issue to
  still be on display.
- Message `0xa837` (mission 90's contract text) says "Fortress Culloden in
  Alioth (0,7)", while the body checks used elsewhere for Alioth decode to
  `(0,4)` (`0x2a51718`). Mission 82's `0x6a57718` is `(0,7)`. This is a text
  coordinate mismatch in the original data, not a control-flow bug.
