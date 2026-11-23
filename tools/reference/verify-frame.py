"""Compare the port's whole flight frame with the executed original's.

`jjffe-smoke --scene-frame` draws a fixed scene (station model 74 and ship models
31, 23 and 25) through the real F1537/F1680/F1772 into the 320x200 framebuffer,
builds the runtime palette with F1330/F1331 and flushes the depth-sorted queue
with F1789. It writes `scene-frame.bin` (64000 indexed pixels + 768-byte
palette) and one JSONL row with the exact camera, objects, tics, light and
shades. The Rust test `the_scene_frame_matches_the_executed_original` rebuilds
the same scene through the port's verified F1680 set-up and model executor,
depth-sorts by the same F1788 keys and draws it with the flight view's
rasterisers. Every pixel is compared in RGB.

Accepted residual (`docs/ACCEPTED_PARITY.md`, `v0.48-14`): the original's F1747
3D text on the station's name board (the port's viewer counts the opcode-10
record but does not draw it) and a two-pixel station-edge depth tie leave fewer
than 64 of the 64000 pixels different. The original is run twice (frames and
rows byte-equal); a moved object, a shifted/blank frame and an empty trace are
replayed to prove the test can fail. Needs the local reference binary and a
content cache (a v35 cache adds the far/dot classes; near objects are identical
without them).
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ACCEPTED_PIXELS = 64


def run_reference(smoke, directory):
    result = subprocess.run([str(smoke), "--scene-frame"], cwd=directory,
                            capture_output=True, timeout=180)
    if result.returncode:
        raise RuntimeError(f"the reference failed ({result.returncode}): "
                           + result.stderr.decode(errors="replace"))
    if b"JJFFE scene-frame-v1" not in result.stderr:
        raise RuntimeError("the reference did not run the scene-frame mode")
    frame = (directory / "scene-frame.bin").read_bytes()
    if len(frame) != 64000 + 768:
        raise RuntimeError(f"wrong scene frame size {len(frame)}")
    return result.stdout, frame


def run_test(command, root, env):
    result = subprocess.run(command, cwd=root, env=env, capture_output=True, timeout=900)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smoke", required=True, type=Path, help="jjffe-smoke.exe")
    parser.add_argument("--cache", required=True, type=Path,
                        help="data.bin (a v35 cache includes the far/dot classes)")
    args = parser.parse_args()
    smoke = args.smoke.resolve(strict=True)
    cache = args.cache.resolve(strict=True)
    root = Path(__file__).resolve().parents[2]
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    with tempfile.TemporaryDirectory(prefix="ffe-scene-frame-") as directory:
        directory = Path(directory)
        runs = [run_reference(smoke, directory) for _ in range(2)]
        if runs[0][0] != runs[1][0] or runs[0][1] != runs[1][1]:
            raise RuntimeError("the original's scene frames differ between runs")
        trace = directory / "scene.jsonl"
        frame = directory / "scene-frame.bin"
        trace.write_bytes(runs[0][0])
        frame.write_bytes(runs[0][1])
        env = dict(os.environ, FFE_SCENE_TRACE=str(trace), FFE_SCENE_FRAME=str(frame),
                   FFE_WORLD_CACHE=str(cache), FFE_SHOT_DIR=str(directory))
        command = [cargo, "test", "--locked", "-p", "ffe-rs", "--test", "scene_frame",
                   "the_scene_frame_matches_the_executed_original", "--", "--ignored", "--exact",
                   "--nocapture"]
        result = run_test(command, root, env)
        if result.returncode:
            raise RuntimeError("the port's scene does not match the original:\n"
                               + result.stderr.decode(errors="replace")[-4000:])
        match = re.search(rb"scene frame: (\d+) of 64000 pixels differ", result.stderr)
        if not match:
            raise RuntimeError("the test did not report its pixel comparison")
        differing = int(match.group(1))
        if differing > ACCEPTED_PIXELS:
            raise RuntimeError(f"{differing} pixels differ, over the accepted {ACCEPTED_PIXELS}")

        # Tamper cases: a moved object, a shifted or blank original frame, an
        # empty trace. Each must make the same exact test fail.
        row = json.loads(runs[0][0])
        moved = json.loads(json.dumps(row))
        moved["objects"][0]["position"][2] += 0x10000
        cases = [
            ("moved object", json.dumps(moved).encode() + b"\n", runs[0][1]),
            ("shifted frame", runs[0][0], b"".join(
                bytes([runs[0][1][(y * 320 + x + 3) % 64000] for x in range(320)])
                for y in range(200)) + runs[0][1][64000:]),
            ("blank frame", runs[0][0], bytes(64000 + 768)),
            ("empty trace", b"", runs[0][1]),
        ]
        for label, altered_trace, altered_frame in cases:
            trace.write_bytes(altered_trace)
            frame.write_bytes(altered_frame)
            if run_test(command, root, env).returncode == 0:
                raise RuntimeError(f"an altered {label} was accepted")
        trace.write_bytes(runs[0][0])
        frame.write_bytes(runs[0][1])
        if run_test(command, root, env).returncode:
            raise RuntimeError("the restored scene no longer matches")
        digest = hashlib.sha256(runs[0][1]).hexdigest()
    print(json.dumps({"scenario": "jjffe-scene-frame-v1",
                      "objects": 4, "shapes": 97, "pixels": 64000,
                      "differing_pixels": differing,
                      "accepted_pixels": ACCEPTED_PIXELS,
                      "repeat_matches": True, "rust_results_match": True,
                      "tamper_cases_rejected": True,
                      "frame_sha256": digest}, indent=2))


if __name__ == "__main__":
    main()
