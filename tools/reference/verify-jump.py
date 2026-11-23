"""A full hyperspace jump against the executed original (release-1).

`jjffe-smoke --jump 2000` starts the default Gateway/Hope game, Tab launches, and
at frame 400 in free flight the real F908 runs with Titican selected (code
`0x12a53716`, 4.42 light years) and the F34/F35 dialog answered yes. The trace
carries F901's plan (the jump time, the fuel burnt, the mass and drive class it
used), the mode 4 timeout, F909's arrival point and the shared random pair.

`crates/ffe-rs/tests/jump_reference.rs` rebuilds the jump through
`Navigator::hyperspace`: the distance, jump time, fuel, days and transit must
match, and the arrival must be on F1507's 7.4..11.3 AU orbit. The verifier runs
the reference twice, tampers the jump time and an empty trace, then runs the
untouched trace.

    python tools/reference/verify-jump.py --reference jjffe-smoke.exe --cache data.bin
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

DESTINATION = 0x12A53716  # Titican, a real Gateway neighbour at 4.42 light years


def run_reference(reference: Path, directory: Path) -> bytes:
    result = subprocess.run([str(reference), "--jump", "2000"], cwd=directory,
                            capture_output=True, timeout=600)
    if result.returncode:
        raise RuntimeError(f"reference --jump failed ({result.returncode}): "
                           f"{result.stderr.decode(errors='replace')[-2000:]}")
    if b"JJFFE jump-v1" not in result.stderr:
        raise RuntimeError("the reference did not run the jump mode")
    return result.stdout


def rows(payload: bytes) -> list:
    return [json.loads(line) for line in payload.decode().splitlines() if line.strip()]


def run_test(cargo: str, trace: Path, cache: Path) -> tuple[bool, str]:
    env = dict(os.environ, FFE_JUMP_TRACE=str(trace), FFE_WORLD_CACHE=str(cache))
    result = subprocess.run([cargo, "test", "-p", "ffe-rs", "--test", "jump_reference",
                             "--", "--ignored", "--exact"],
                            cwd=Path(__file__).resolve().parents[2], env=env,
                            capture_output=True, timeout=900)
    return result.returncode == 0, (result.stdout + result.stderr).decode(errors="replace")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--reference", type=Path, required=True, help="jjffe-smoke.exe")
    parser.add_argument("--cache", type=Path, required=True, help="data.bin")
    args = parser.parse_args()
    reference = args.reference.resolve(strict=True)
    cache = args.cache.resolve(strict=True)
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    with tempfile.TemporaryDirectory(prefix="ffe-jump-") as directory:
        directory = Path(directory)
        first = run_reference(reference, directory)
        second = run_reference(reference, directory)
        if first != second:
            raise RuntimeError("repeated --jump traces differ")
        parsed = rows(first)
        plan = next((row["jump_plan"] for row in parsed if "jump_plan" in row), None)
        arrival = next((row["jump_arrival"] for row in parsed if "jump_arrival" in row), None)
        transit = [row["jump"] for row in parsed if "jump" in row]
        if plan is None or arrival is None or not transit:
            raise RuntimeError("the trace is missing the plan, transit or arrival")
        if plan["destination"] != DESTINATION or not (7 * 60 < plan["distance"] < 600):
            raise RuntimeError(f"the destination is not Titican at 4.42 ly: {plan}")
        if plan["mode"] != 4 or plan["timeout"] != plan["jump_time"]:
            raise RuntimeError(f"F908 did not leave mode 4 with its timeout: {plan}")
        if not (plan["fuel"][0] > plan["fuel"][1]):
            raise RuntimeError(f"the jump burnt no fuel: {plan['fuel']}")
        if arrival["system"] != DESTINATION or arrival["mode"] == 4:
            raise RuntimeError(f"the arrival is not the destination system: {arrival}")

        trace = directory / "jump.jsonl"
        trace.write_bytes(first)
        ok, output = run_test(cargo, trace, cache)
        if not ok:
            raise RuntimeError(f"jump_reference failed on the untouched trace:\n{output[-3000:]}")

        tampered = rows(first)
        row = next(row["jump_plan"] for row in tampered if "jump_plan" in row)
        row["jump_time"] += 1
        bad = directory / "jump-tampered.jsonl"
        bad.write_text("\n".join(json.dumps(row) for row in tampered) + "\n")
        ok, _ = run_test(cargo, bad, cache)
        if ok:
            raise RuntimeError("jump_reference accepted a tampered jump time")

        empty = directory / "jump-empty.jsonl"
        empty.write_text("")
        ok, _ = run_test(cargo, empty, cache)
        if ok:
            raise RuntimeError("jump_reference accepted an empty trace")

        print(json.dumps({
            "scenario": "jump-v1",
            "destination": hex(plan["destination"]),
            "distance": plan["distance"],
            "jump_time": plan["jump_time"],
            "days": plan["jump_time"] / 65536.0,
            "fuel_burnt": plan["fuel"][0] - plan["fuel"][1],
            "mass": plan["mass"],
            "drive": plan["drive"],
            "transit_frames_sampled": len(transit),
            "arrival_mode": arrival["mode"],
            "repeat_traces_match": True,
            "tamper_rejected": True,
            "empty_rejected": True,
            "tests": ["the_originals_full_jump_matches_the_port"],
            "accepted_gap": "F909's arrival draw ledger (the pair after the call) differs; "
                            "the arrival point is checked on the port's 7.4..11.3 AU orbit",
        }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
