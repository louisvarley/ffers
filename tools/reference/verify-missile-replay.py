"""Replay missile, ECM, mine and escape outcomes against the original (PAR-22).

Five `jjffe-smoke --missile-encounter <scenario> 900 --start-key=0x39` runs:

* `hit`: F1096/F941's model-6 player missile reaches the pirate, damages it and
  leaves F719's model-0x9b explosion;
* `ecm`: standard F935/F718 E.C.M. destroys model 6;
* `naval`: model 7 survives the standard burst and is destroyed by naval E.C.M.;
* `mine`: the 0xc1 mine is F941's model 5 / state 0x14, ballistic and never
  expiring;
* `escape`: F1098 launches model 16 (Stowmaster) and leaves the old hull in
  state 0x16.

`crates/ffe-rs/tests/missile_encounter.rs` checks all five traces and the port's
new F719 hit/timeout path (`Navigator::tick_missiles_inner`).

Run:

    python tools/reference/verify-missile-replay.py --reference jjffe-smoke.exe \
        --cache data.bin
"""

import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

SCENARIOS = ("hit", "ecm", "naval", "mine", "escape")
EXPECTED = {
    "a_player_missile_hits_and_leaves_f719s_explosion",
    "standard_and_naval_ecm_choose_their_models",
    "a_mine_keeps_f941s_state_and_never_expires",
    "f1098_leaves_the_stowmaster_and_the_abandoned_hull",
    "the_port_spawns_f719_on_a_missile_hit_and_on_a_timeout",
}


def run_reference(reference: Path, directory: Path, scenario: str) -> bytes:
    result = subprocess.run(
        [str(reference), "--missile-encounter", scenario, "900", "--start-key=0x39"],
        cwd=directory, capture_output=True, timeout=300)
    if result.returncode:
        raise RuntimeError(f"reference --missile-encounter {scenario} failed "
                           f"({result.returncode}): {result.stderr.decode(errors='replace')}")
    return result.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True, help="data.bin")
    args = parser.parse_args()
    reference = args.reference.resolve(strict=True)
    cache = args.cache.resolve(strict=True)
    root = Path(__file__).resolve().parents[2]
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    summary = {}
    with tempfile.TemporaryDirectory(prefix="ffe-missile-replay-") as directory:
        directory = Path(directory)
        for scenario in SCENARIOS:
            first = run_reference(reference, directory, scenario)
            second = run_reference(reference, directory, scenario)
            if first != second:
                raise RuntimeError(f"repeated --missile-encounter {scenario} traces differ")
            (directory / f"{scenario}.jsonl").write_bytes(first)
            rows = [json.loads(line) for line in first.decode().splitlines()
                    if line.startswith('{"enc"')]
            summary[scenario] = {
                "rows": len(rows),
                "missile_models": sorted({m["model"] for row in rows for m in row["missiles"]}),
                "missile_ai": sorted({m["ai"] for row in rows for m in row["missiles"]}),
                "f719_frames": sum(1 for row in rows
                                   if any(e["model"] == 155 for e in row["effects"])),
            }
        env = dict(os.environ, FFE_MISSILE_TRACE=str(directory), FFE_WORLD_CACHE=str(cache))
        result = subprocess.run(
            [cargo, "test", "-p", "ffe-rs", "--test", "missile_encounter",
             "--", "--ignored", "--nocapture", "--test-threads=1"],
            cwd=root, env=env, capture_output=True, timeout=900)
        output = (result.stdout + result.stderr).decode(errors="replace")
        if result.returncode:
            raise RuntimeError(f"missile_encounter failed:\n{output[-3000:]}")
        passed = {test.rsplit("::", 1)[-1] for test in
                  re.findall(r"^test (\S+) \.\.\. ok$", output, re.MULTILINE)}
        missing = EXPECTED - passed
        if missing:
            raise RuntimeError(f"missile_encounter did not run {sorted(missing)} (ran {sorted(passed)})")
        print(json.dumps({
            "scenario": "missile-encounter-replay-v1",
            "scenarios": summary,
            "repeat_traces_match": True,
            "tests": sorted(passed),
            "accepted_gaps": ["whole-encounter NPC-vs-NPC missile chains are not traced",
                              "the rotating-frame rendezvous is not replayed as one scene"],
        }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
