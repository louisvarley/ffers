"""Run one checkpoint per playthrough stage against the executed original.

`release-1` chains the stage oracles into one gate. Each stage runs a reference
adapter mode (or a whole scripted scenario) on the unchanged JJFFE build and the
port test that must match it; the summary names the mode and the port evidence.

    import/start   --legacy-save game/FIRSTENC._01 ......... legacy_save.rs
    start/intro    --intro .................................. verify-intro.py
    launch/fly/dock --flight takeoff|hover|launch|dock|paddock  verify-flight.py
    trade          --trade 220 .............................. market_trade.rs
    save/load      --save-roundtrip 260 ..................... save_roundtrip.rs
    jump           --jump 2000 .............................. jump_reference.rs
    combat         --combat / --missile-encounter ........... combat_replay.rs
    missions/BBS   --daily 2000 ............................. daily_reference.rs
    render         --scene-frame / --game-frame ............. scene_frame.rs

The stage encodings that are not executed-original traces are recorded in
`docs/ACCEPTED_PARITY.md` (the mission chain's in-play run and the F909 arrival
draw ledger).

    python tools/reference/verify-playthrough.py \
        --reference-dir .cache/jjffe-reference --cache .cache/ui11/data.bin

Exit status: 0 when every stage passes or is environment-gated (77), 1 on a
failure, 2 when a required input is missing.
"""

import argparse
import json
from pathlib import Path
import subprocess
import sys
import time

SKIP = 77

# (stage, script, arguments, what the port proves). {smoke} and {cache} are
# filled from --reference-dir and --cache.
STAGES = [
    ("import/start", "verify-legacy-save.py", ["--smoke", "{smoke}"],
     "the real F70/F66/F171 on the supplied DOS file; saves::legacy_import field and byte for byte"),
    ("start/intro", "verify-intro.py", ["--reference", "{smoke}"],
     "the real F961/F962 intro loop and palette; the port's intro frame and actions"),
    ("launch/fly/dock", "verify-flight.py", ["--reference", "{smoke}", "--cache", "{cache}"],
     "the real F1013/F1031 launch, the free-flight tail and F1020's dock/paddock returns; the port's per-frame flight state"),
    ("trade", "verify-trade.py", ["--reference", "{smoke}"],
     "the real F555/F556 buy and sell with every refusal; market::Market::{buy,sell}"),
    ("save/load", "verify-save-roundtrip.py", ["--reference", "{smoke}"],
     "the real F214/F170/F174 write and F176/F171 restore; saves::legacy_import on the written file"),
    ("jump", "verify-jump.py", ["--reference", "{smoke}", "--cache", "{cache}"],
     "the real F908/F901/F909 from Gateway to Titican; Navigator::hyperspace's plan, transit and arrival"),
    ("combat", "verify-combat-replay.py", ["--reference", "{smoke}", "--cache", "{cache}"],
     "the real F750/F754/F931 combat and F719/F935/F1098 encounters; combat_replay.rs, missile_encounter.rs"),
    ("missions/BBS", "verify-daily-replay.py", ["--reference", "{smoke}", "--cache", "{cache}"],
     "F633's day boundary, the job/news draws and the market/board steps; daily_reference.rs"),
    ("render", "verify-frame.py", ["--smoke", "{smoke}", "--cache", "{cache}"],
     "the real F1537/F1680/F1772/F1330/F1789 scene; scene_frame.rs, 63,946/64,000 pixels"),
]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--reference-dir", type=Path, default=Path(".cache/jjffe-reference"))
    parser.add_argument("--cache", type=Path, default=Path("data.bin"))
    parser.add_argument("--stages", nargs="*", metavar="NAME",
                        help="run only these stages (substring match)")
    parser.add_argument("--list", action="store_true", help="print the stage table and exit")
    parser.add_argument("--timeout", type=int, default=3600,
                        help="per-stage timeout in seconds (default 3600)")
    args = parser.parse_args()

    root = args.root.resolve()
    smoke = (root / args.reference_dir / "jjffe-smoke.exe").resolve()
    cache = (root / args.cache).resolve()
    if args.list:
        for stage, script, _, evidence in STAGES:
            print(f"{stage:18} {script:28} {evidence}")
        return 0
    missing = [str(path) for path in (smoke, cache) if not path.exists()]
    if missing:
        print("Missing inputs:\n  " + "\n  ".join(missing) + "\n")
        return 2

    selected = [entry for entry in STAGES
                if not args.stages or any(name.lower() in entry[0].lower()
                                          or name.lower() in entry[1].lower()
                                          for name in args.stages)]
    if not selected:
        print(f"no stage matches {args.stages}")
        return 2

    results = {"PASS": [], "SKIP": [], "FAIL": []}
    summary = []
    started = time.monotonic()
    for stage, script, arguments, evidence in selected:
        command = [sys.executable, str(root / "tools" / "reference" / script)]
        command += [argument.format(smoke=smoke, cache=cache) for argument in arguments]
        stage_started = time.monotonic()
        try:
            result = subprocess.run(command, cwd=root, capture_output=True,
                                    timeout=args.timeout)
            returncode = result.returncode
            output = (result.stdout + result.stderr).decode("utf-8", errors="replace")
        except subprocess.TimeoutExpired as error:
            returncode = 1
            output = f"timed out after {args.timeout}s\n" + \
                (error.stdout or b"").decode("utf-8", errors="replace")[-2000:]
        elapsed = time.monotonic() - stage_started
        status = "PASS" if returncode == 0 else ("SKIP" if returncode == SKIP else "FAIL")
        results[status].append(stage)
        summary.append({
            "stage": stage,
            "oracle": script,
            "evidence": evidence,
            "status": status,
            "seconds": round(elapsed, 1),
        })
        print(f"{status} {stage} ({script}, {elapsed:.1f}s)", flush=True)
        if status == "FAIL":
            for line in output.strip().splitlines()[-10:]:
                print(f"     {line}")
        elif status == "SKIP":
            reasons = [line for line in output.splitlines() if line.startswith("SKIP ")]
            for reason in reasons:
                print(f"     {reason}")

    elapsed = time.monotonic() - started
    print()
    print(f"{len(results['PASS'])} passed, {len(results['SKIP'])} skipped, "
          f"{len(results['FAIL'])} failed of {len(selected)} stages in {elapsed:.1f}s")
    print(json.dumps({"scenario": "playthrough-v1", "stages": summary}, indent=2))
    if results["FAIL"]:
        print("failed: " + ", ".join(results["FAIL"]))
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
