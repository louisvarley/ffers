"""Verify F998 start records against the executed JJFFE baseline (local only)."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

p = argparse.ArgumentParser(description=__doc__)
p.add_argument("--reference", type=Path, required=True)
p.add_argument("--cache", type=Path, required=True)
a = p.parse_args()
root = Path(__file__).resolve().parents[2]
reference = a.reference.resolve(strict=True)
env = dict(os.environ, FFE_START_RECORD="1", FFE_WORLD_CACHE=str(a.cache.resolve(strict=True)))
with tempfile.TemporaryDirectory(prefix="ffe-source-starts-") as temporary:
    records = []
    for key in [57, 2, 3, 4, 6]:
        runs = [subprocess.run([str(reference), "--legal-probe", "163", str(key)], cwd=temporary,
                    env=env, capture_output=True, check=True, timeout=30).stdout for _ in range(2)]
        if runs[0] != runs[1]:
            raise RuntimeError("source start is not repeatable")
        rows = [json.loads(line) for line in runs[0].splitlines() if line.startswith(b"{")]
        records.append(rows[-1])
    trace = Path(temporary) / "starts.jsonl"
    trace.write_text("\n".join(json.dumps(record) for record in records), encoding="utf-8")
    env["FFE_START_RECORDS"] = str(trace)
    command = ["cargo", "test", "--locked", "-p", "ffe-rs", "--test", "source_boot",
        "--", "--include-ignored"]
    subprocess.run(command, cwd=root, env=env, check=True)
    records[0]["energy"] += 1
    trace.write_text("\n".join(json.dumps(record) for record in records), encoding="utf-8")
    tamper = subprocess.run(command[:7] + ["all_delivered_start_records_match_the_executed_original", "--", "--include-ignored"],
        cwd=root, env=env, capture_output=True)
    if tamper.returncode == 0 or b"assertion" not in tamper.stdout + tamper.stderr:
        raise RuntimeError("tampered source record was accepted")
print(json.dumps({"source_starts": 5, "repeat_matches": True, "rust_matches": True, "tamper_rejected": True}))
