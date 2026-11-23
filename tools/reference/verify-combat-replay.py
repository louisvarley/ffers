"""Replay the Hope combat scenario against the executed original (gameplay-8).

`jjffe-smoke --combat 900 --start-key=0x39` starts the default new game, puts
the player 2,000 km behind the pirate leader and lets the original's F750/F754
AI and F931 run. The adapter writes one JSONL row per frame with every hostile
ship's whole 0x152-byte record plus the effect objects.

`crates/ffe-core/tests/combat_replay.rs` checks the combat loop (state changes,
the player's fire, hostile damage and F939/F942 effects) and replays the leader's
guidance and physics. Two reference runs must be byte-identical.

Run:

    python tools/reference/verify-combat-replay.py --reference jjffe-smoke.exe \
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

EXPECTED = {"the_original_combat_loop_fires_hits_and_draws_effects",
            "the_leaders_guidance_and_physics_replay"}
OFFSET = "-2000000"


def run_reference(reference: Path, directory: Path, name: str) -> bytes:
    env = dict(os.environ, FFE_COMBAT_OFFSET=OFFSET)
    result = subprocess.run(
        [str(reference), "--combat", "900", "--start-key=0x39"],
        cwd=directory, env=env, capture_output=True, timeout=300)
    if result.returncode:
        raise RuntimeError(f"reference --combat failed ({result.returncode}): "
                           f"{result.stderr.decode(errors='replace')}")
    return result.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True, help="data.bin")
    args = parser.parse_args()
    reference = args.reference.resolve(strict=True)
    args.cache.resolve(strict=True)
    root = Path(__file__).resolve().parents[2]
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    with tempfile.TemporaryDirectory(prefix="ffe-combat-replay-") as directory:
        directory = Path(directory)
        first = run_reference(reference, directory, "first")
        second = run_reference(reference, directory, "second")
        if first != second:
            raise RuntimeError("repeated --combat traces differ")
        trace = directory / "combat.jsonl"
        trace.write_bytes(first)
        rows = [json.loads(line) for line in first.decode().splitlines()
                if line.startswith('{"combat"')]
        if len(rows) < 300:
            raise RuntimeError(f"only {len(rows)} combat rows")
        env = dict(os.environ, FFE_COMBAT_TRACE=str(trace))
        result = subprocess.run(
            [cargo, "test", "-p", "ffe-core", "--test", "combat_replay",
             "--", "--ignored", "--nocapture"],
            cwd=root, env=env, capture_output=True, timeout=900)
        output = (result.stdout + result.stderr).decode(errors="replace")
        if result.returncode:
            raise RuntimeError(f"combat_replay failed:\n{output[-3000:]}")
        passed = {test.rsplit("::", 1)[-1] for test in
                  re.findall(r"^test (\S+) \.\.\. ok$", output, re.MULTILINE)}
        missing = EXPECTED - passed
        if missing:
            raise RuntimeError(f"combat_replay did not run {sorted(missing)} (ran {sorted(passed)})")
        states = sorted({ship["state"] for row in rows for ship in row["ships"]})
        print(json.dumps({
            "scenario": "combat-replay-v1",
            "rows": len(rows),
            "repeat_traces_match": True,
            "hostile_states": states,
            "player_hull": rows[0]["player"]["hull"],
            "player_hull_min": min(row["player"]["hull"] for row in rows),
            "tests": sorted(passed),
            "note": "leader F750 guidance/physics replay; F754 state-1/2 pass recorded as residual",
        }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
