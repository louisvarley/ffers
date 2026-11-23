"""A commander save/load round trip against the executed original (release-1).

`jjffe-smoke --save-roundtrip 260` runs the real Lave docked game to frame 240,
then F214 (zero unused objects), F170 (mangle the F175 body) and F174 (F69 + F65
+ the body) write a commander file. Every saved field is changed and F176
(F70 + F66 + the body) plus F171 (unmangle) must restore it; the file is left on
disk for the port.

`crates/ffe-rs/tests/save_roundtrip.rs` imports the original's own file through
`ffe_rs::saves::legacy_import` and requires the same field map the original's
restore check used (name, cash, hold, fuel, clock, random pair, player object,
ship and all 38 cargo words). The verifier runs the reference twice, tampers the
before row and an empty trace, then runs the untouched trace.

    python tools/reference/verify-save-roundtrip.py --reference jjffe-smoke.exe
"""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

PHASES = {"before", "perturbed", "restored"}


def run_reference(reference: Path, directory: Path, save_file: Path) -> bytes:
    env = dict(os.environ, FFE_SAVE_FILE=str(save_file))
    result = subprocess.run([str(reference), "--save-roundtrip", "260"], cwd=directory,
                            env=env, capture_output=True, timeout=300)
    if result.returncode:
        raise RuntimeError(f"reference --save-roundtrip failed ({result.returncode}): "
                           f"{result.stderr.decode(errors='replace')[-2000:]}")
    if b"JJFFE save-roundtrip-v1" not in result.stderr:
        raise RuntimeError("the reference did not run the save-roundtrip mode")
    if b"F174 wrote" not in result.stderr:
        raise RuntimeError("the reference did not write a commander file")
    if not save_file.exists() or save_file.stat().st_size == 0:
        raise RuntimeError("the reference left no file behind")
    return result.stdout


def rows(payload: bytes) -> list:
    return [json.loads(line) for line in payload.decode().splitlines() if line.strip()]


def run_test(cargo: str, trace: Path, save_file: Path) -> tuple[bool, str]:
    env = dict(os.environ, FFE_SAVE_TRACE=str(trace), FFE_SAVE_FILE=str(save_file))
    result = subprocess.run([cargo, "test", "-p", "ffe-rs", "--test", "save_roundtrip",
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
    with tempfile.TemporaryDirectory(prefix="ffe-save-roundtrip-") as directory:
        directory = Path(directory)
        first_file = directory / "roundtrip-a.ffe"
        save_file = directory / "roundtrip-b.ffe"
        first = run_reference(reference, directory, first_file)
        second = run_reference(reference, directory, save_file)
        size = save_file.stat().st_size
        if first != second:
            raise RuntimeError("repeated --save-roundtrip traces differ")
        parsed = rows(first)
        saves = {row["save"]["phase"]: row["save"] for row in parsed if "save" in row}
        if set(saves) != PHASES:
            raise RuntimeError(f"the trace phases are {sorted(saves)}, not {sorted(PHASES)}")
        result = next(row["save_result"] for row in parsed if "save_result" in row)
        if not result.get("restored") or result.get("packed", 0) <= 0:
            raise RuntimeError("the reference's own restore check did not pass")
        for field in ("name", "credits", "capacity", "used", "random", "cargo"):
            if saves["restored"][field] != saves["before"][field]:
                raise RuntimeError(f"the original did not restore {field}")
        if saves["perturbed"]["credits"] == saves["before"]["credits"]:
            raise RuntimeError("the perturbation did not reach the cash")

        trace = directory / "save.jsonl"
        trace.write_bytes(first)
        ok, output = run_test(cargo, trace, save_file)
        if not ok:
            raise RuntimeError(f"save_roundtrip failed on the untouched trace:\n{output[-3000:]}")

        tampered = rows(first)
        row = next(row["save"] for row in tampered if row.get("save", {}).get("phase") == "before")
        row["credits"] += 1
        bad = directory / "save-tampered.jsonl"
        bad.write_text("\n".join(json.dumps(row) for row in tampered) + "\n")
        ok, _ = run_test(cargo, bad, save_file)
        if ok:
            raise RuntimeError("save_roundtrip accepted a tampered before row")

        empty = directory / "save-empty.jsonl"
        empty.write_text("")
        ok, _ = run_test(cargo, empty, save_file)
        if ok:
            raise RuntimeError("save_roundtrip accepted an empty trace")

        print(json.dumps({
            "scenario": "save-roundtrip-v1",
            "file_bytes": size,
            "packed_body": result["packed"],
            "fields": ["name", "credits", "capacity", "used", "fuel", "clock",
                       "random", "player_object", "ship", "ship_name", "cargo"],
            "restore_verified_in_both": True,
            "repeat_traces_match": True,
            "tamper_rejected": True,
            "empty_rejected": True,
            "tests": ["the_commanders_save_load_round_trip_matches_the_original"],
            "accepted_gap": "the string section's commander_name copy is two bytes short",
        }, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
