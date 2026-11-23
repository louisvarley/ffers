"""Replay the supplied DOS-era commander file against the executed original.

`jjffe-smoke --legacy-save game/FIRSTENC._01` runs the real F70 (F69's loose
global arrays), F66 (the F65 stream) and F171 (the F170 body: magic, XOR
checksum, zero-run expansion) and prints one JSONL row: the container offsets,
the commander's player-block fields and the whole 0x556a-byte player block plus
the 0x984a-byte object array as hex.

The Rust test `the_supplied_legacy_save_matches_the_reference_player_block`
imports the same file through `ffe_rs::saves::legacy_import` and requires every
field and both raw blocks to match. The original is run twice (byte equality)
and every field, the empty trace and a truncated block are replayed to prove
the test can fail.

The reference row is the authority for the DOS-era field map: the block is the
source `gmstruct` layout, not the shifted block the seven-dword scatter
misparse produced. Needs the local reference binary; never copies the file.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

SCENARIO = "jjffe-legacy-save-v1"


def run_reference(smoke, legacy, directory):
    result = subprocess.run([str(smoke), "--legacy-save", str(legacy)], cwd=directory,
                            capture_output=True, timeout=120)
    if result.returncode:
        raise RuntimeError(f"the reference failed ({result.returncode}): "
                           + result.stderr.decode(errors="replace"))
    if b"JJFFE legacy-save-v1" not in result.stderr:
        raise RuntimeError("the reference did not run the legacy-save mode")
    return result.stdout


def tamper_cases(row):
    def field(name, value):
        def change(r):
            r[name] = value
        return (name, change)

    def raw(index):
        def change(r):
            text = list(r["raw_player"])
            text[index] = "0" if text[index] != "0" else "1"
            r["raw_player"] = "".join(text)
        return (f"raw_player[{index}]", change)

    def objects(index):
        def change(r):
            text = list(r["raw_objects"])
            text[index] = "0" if text[index] != "0" else "1"
            r["raw_objects"] = "".join(text)
        return (f"raw_objects[{index}]", change)

    def offset(r):
        r["body_offset"] += 1

    return [
        field("name", "Jameson "),
        field("credits", row["credits"] + 1),
        field("cargo_capacity", row["cargo_capacity"] + 1),
        field("cargo_used", row["cargo_used"] + 1),
        field("hydrogen_fuel", row["hydrogen_fuel"] + 1),
        field("military_fuel", row["military_fuel"] + 1),
        field("radioactives", row["radioactives"] + 1),
        field("clock_tics", row["clock_tics"] + 1),
        field("clock_days", row["clock_days"] + 1),
        field("random_a", row["random_a"] ^ 1),
        field("random_b", row["random_b"] ^ 1),
        field("ship_object", row["ship_object"] + 1),
        field("ship_model", row["ship_model"] + 1),
        field("ship_name", "TY-199"),
        raw(0x114 * 2),
        raw(0x1BC * 2),
        objects(0x74 * 2),
        ("body_offset", offset),
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smoke", required=True, type=Path, help="jjffe-smoke.exe")
    parser.add_argument("--legacy", type=Path,
                        help="FIRSTENC._01 (default: <repo>/game/FIRSTENC._01)")
    args = parser.parse_args()
    smoke = args.smoke.resolve(strict=True)
    root = Path(__file__).resolve().parents[2]
    legacy = (args.legacy or root / "game" / "FIRSTENC._01").resolve(strict=True)
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    with tempfile.TemporaryDirectory(prefix="ffe-legacy-save-") as directory:
        directory = Path(directory)
        outputs = [run_reference(smoke, legacy, directory) for _ in range(2)]
        if outputs[0] != outputs[1]:
            raise RuntimeError("the original's legacy-save rows differ between runs")
        row = json.loads(outputs[0])
        if row.get("scenario") != SCENARIO:
            raise RuntimeError("wrong scenario")
        if len(bytes.fromhex(row["raw_player"])) != 0x5535:
            raise RuntimeError("wrong player block size")
        if row["player_bytes"] != 0x5535:
            raise RuntimeError("wrong player block byte count")
        if len(bytes.fromhex(row["raw_objects"])) != 0x984A:
            raise RuntimeError("wrong object array size")
        # The established commander: the supplied Jameson file.
        for key, expected in (("name", "Jameson"), ("credits", 10000),
                              ("ship_model", 31), ("ship_name", "TY-198"),
                              ("ship_object", 87),
                              ("clock_tics", 1014497784), ("clock_days", 1187064),
                              ("random_a", 0x38ACD1), ("random_b", 0x0FC168D5),
                              ("cargo_capacity", 7), ("cargo_used", 1),
                              ("hydrogen_fuel", 1)):
            if row[key] != expected:
                raise RuntimeError(f"the reference's {key} is {row[key]!r}, expected {expected!r}")

        trace = directory / "legacy-save.jsonl"
        trace.write_bytes(outputs[0])
        env = dict(os.environ, FFE_LEGACY_SAVE_TRACE=str(trace),
                   FFE_LEGACY_SAVE=str(legacy))
        command = [cargo, "test", "--locked", "-p", "ffe-rs", "--test", "legacy_save",
                   "the_supplied_legacy_save_matches_the_reference_player_block",
                   "--", "--ignored", "--exact"]
        subprocess.run(command, cwd=root, env=env, check=True, timeout=600)
        for label, change in tamper_cases(row):
            altered = json.loads(json.dumps(row))
            change(altered)
            trace.write_bytes(json.dumps(altered).encode() + b"\n")
            result = subprocess.run(command, cwd=root, env=env, capture_output=True, timeout=600)
            if result.returncode == 0:
                raise RuntimeError(f"an altered {label} was accepted")
        trace.write_bytes(b"")
        if subprocess.run(command, cwd=root, env=env, capture_output=True, timeout=600).returncode == 0:
            raise RuntimeError("an empty trace was accepted")
        trace.write_bytes(outputs[0])
        subprocess.run(command, cwd=root, env=env, check=True, timeout=600)
        digest = hashlib.sha256(outputs[0]).hexdigest()
    print(json.dumps({"scenario": "jjffe-legacy-save-replay-v1",
                      "fields": 14, "player_block_bytes": 0x5535,
                      "object_array_bytes": 0x984A, "repeat_matches": True,
                      "rust_results_match": True,
                      "tamper_fields_and_empty_trace_rejected": True,
                      "trace_sha256": digest}, indent=2))


if __name__ == "__main__":
    main()
