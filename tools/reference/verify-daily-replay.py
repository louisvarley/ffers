"""Replay the first day boundary against the executed original (PAR-20).

`jjffe-smoke --daily 2000 --start-key=0x02` runs the Ross 154 pad at 10000x and
prints every F922/F1530 draw. The first day boundary is the frame with F633's
125 draws: the news/job chances, the place's 99 market steps, F621's board
replacement and F634's ship queue.

`crates/ffe-rs/tests/daily_reference.rs` checks the ledger shape (125 draws, the
first twelve limits, 105 F1530 steps), replays the pair through every recorded
step, then runs `Navigator::tick_days` from the same pair and compares the final
pair and the changed board/job state. The per-station markets keep their own
streams (EXP-006), so the 99 market steps are consumed but not read; that is the
accepted residual.

Run:

    python tools/reference/verify-daily-replay.py --reference jjffe-smoke.exe \
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


def run_reference(reference: Path, directory: Path) -> bytes:
    result = subprocess.run(
        [str(reference), "--daily", "2000", "--start-key=0x02"],
        cwd=directory, capture_output=True, timeout=300)
    if result.returncode:
        raise RuntimeError(f"reference --daily failed ({result.returncode}): "
                           f"{result.stderr.decode(errors='replace')}")
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
    with tempfile.TemporaryDirectory(prefix="ffe-daily-replay-") as directory:
        directory = Path(directory)
        first = run_reference(reference, directory)
        second = run_reference(reference, directory)
        if first != second:
            raise RuntimeError("repeated --daily traces differ")
        trace = directory / "daily.jsonl"
        trace.write_bytes(first)
        rows = [json.loads(line) for line in first.decode().splitlines() if line]
        boundary = next(row["day_boundary"] for row in rows if "day_boundary" in row)
        draws = [row["draw"] for row in rows if row.get("draw", {}).get("frame") == boundary]
        if len(draws) != 125:
            raise RuntimeError(f"the day boundary has {len(draws)} draws, not 125")
        env = dict(os.environ, FFE_WORLD_CACHE=str(cache), FFE_DAILY_TRACE=str(trace))
        result = subprocess.run(
            [cargo, "test", "-p", "ffe-rs", "--test", "daily_reference",
             "--", "--ignored", "--nocapture"],
            cwd=root, env=env, capture_output=True, timeout=900)
        output = (result.stdout + result.stderr).decode(errors="replace")
        if result.returncode:
            raise RuntimeError(f"daily_reference failed:\n{output[-3000:]}")
        passed = {test.rsplit("::", 1)[-1] for test in
                  re.findall(r"^test (\S+) \.\.\. ok$", output, re.MULTILINE)}
        expected = {"the_original_day_boundary_is_the_ports_day_boundary"}
        if expected - passed:
            raise RuntimeError(f"daily_reference did not run {sorted(expected - passed)}")
        print(json.dumps({
            "scenario": "daily-replay-v1",
            "boundary_frame": boundary,
            "draws": len(draws),
            "advance_draws": sum(1 for draw in draws if draw["kind"] == "advance"),
            "first_limits": [draw["limit"] for draw in draws[:12]],
            "pair_before": draws[0]["before"],
            "pair_after": draws[-1]["after"],
            "repeat_traces_match": True,
            "tests": sorted(passed),
            "accepted_gap": "EXP-006: per-station market streams are not the shared pair",
        }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
