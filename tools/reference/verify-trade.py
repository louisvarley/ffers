"""The station trade against the executed original (release-1).

`jjffe-smoke --trade 220` runs the real Lave docked game to frame 200, then F555
(one or more tonnes bought) and F556 (sold) on the player block with fixed
market entries, cash, hold and stock. Ten cases cover the success path and all
five refusals (`0x98e1`..`0x98e5`); each row carries the cash, cargo word, hold
used, station stock and F557's message code before and after.

`crates/ffe-rs/tests/market_trade.rs` rebuilds the same market and hold and
replays every case through `ffe_core::market::Market::{buy,sell}`. The verifier
runs the reference twice, tampers a row and an empty trace to prove the test is
not vacuous, then runs the untouched trace.

    python tools/reference/verify-trade.py --reference jjffe-smoke.exe
"""

import argparse
import json
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

CASES = 10


def run_reference(reference: Path, directory: Path) -> bytes:
    result = subprocess.run([str(reference), "--trade", "220"], cwd=directory,
                            capture_output=True, timeout=300)
    if result.returncode:
        raise RuntimeError(f"reference --trade failed ({result.returncode}): "
                           f"{result.stderr.decode(errors='replace')}")
    if b"JJFFE trade-v1" not in result.stderr:
        raise RuntimeError("the reference did not run the trade mode")
    return result.stdout


def rows(payload: bytes) -> list:
    out = []
    for line in payload.decode().splitlines():
        line = line.strip()
        if line:
            out.append(json.loads(line))
    return out


def run_test(cargo: str, trace: Path) -> tuple[bool, str]:
    import os
    env = dict(os.environ, FFE_TRADE_TRACE=str(trace))
    result = subprocess.run([cargo, "test", "-p", "ffe-rs", "--test", "market_trade",
                             "--", "--ignored", "--exact"],
                            cwd=Path(__file__).resolve().parents[2], env=env,
                            capture_output=True, timeout=900)
    return result.returncode == 0, (result.stdout + result.stderr).decode(errors="replace")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--reference", type=Path, required=True, help="jjffe-smoke.exe")
    args = parser.parse_args()
    reference = args.reference.resolve(strict=True)
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    with tempfile.TemporaryDirectory(prefix="ffe-trade-") as directory:
        directory = Path(directory)
        first = run_reference(reference, directory)
        second = run_reference(reference, directory)
        if first != second:
            raise RuntimeError("repeated --trade traces differ")
        parsed = rows(first)
        table = next((row["trade_table"] for row in parsed if "trade_table" in row), None)
        cases = [row["trade"] for row in parsed if "trade" in row]
        if table is None or len(cases) != CASES:
            raise RuntimeError(f"the trace has {len(cases)} cases, not {CASES}")
        codes = {case["code"] for case in cases}
        if codes != {0, 0x98E1, 0x98E2, 0x98E3, 0x98E4, 0x98E5}:
            raise RuntimeError(f"the trace's message codes are {sorted(codes)}")
        trace = directory / "trade.jsonl"
        trace.write_bytes(first)
        ok, output = run_test(cargo, trace)
        if not ok:
            raise RuntimeError(f"market_trade failed on the untouched trace:\n{output[-3000:]}")

        # A tampered row must fail the test.
        tampered = rows(first)
        row = next(row["trade"] for row in tampered if row.get("trade", {}).get("case") == 0)
        row["cash"][1] += 1
        bad = directory / "trade-tampered.jsonl"
        bad.write_text("\n".join(json.dumps(row) for row in tampered) + "\n")
        ok, _ = run_test(cargo, bad)
        if ok:
            raise RuntimeError("market_trade accepted a tampered cash result")

        # An empty trace must be rejected too.
        empty = directory / "trade-empty.jsonl"
        empty.write_text("")
        ok, _ = run_test(cargo, empty)
        if ok:
            raise RuntimeError("market_trade accepted an empty trace")

        print(json.dumps({
            "scenario": "trade-v1",
            "cases": len(cases),
            "message_codes": sorted(codes),
            "repeat_traces_match": True,
            "tamper_rejected": True,
            "empty_rejected": True,
            "tests": ["the_originals_buy_and_sell_match_the_port"],
        }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
