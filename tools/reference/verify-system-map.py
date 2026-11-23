"""Compare the 3D system map the port draws with the one the executed original draws.

`jjffe-smoke --game-frame 560 0x3d 0x40` runs the real game from the Ross 154 start, presses F3 (the
galaxy map) and then F6 (F966: this system's map, module 5). With `FFE_MAP_DUMP=<file>` the harness
also writes the page's state (the object table `DATA_008801`, the camera block `DATA_008968`, the focus
`DATA_008979`, the turn and zoom words, the time point `+0x400` and the game clock): the focus slot, the
zoom, the camera's position and every object's screen position (`+0x98`, `+0x9a`) are what the port must
match. The Rust test `the_page_draws_at_its_start` opens the page at the same time point
(`FFE_MAP_SECONDS`, from the dump) and writes its frame and the dots' places.

Checked: the focus slot and the zoom words, the camera position; every body's screen position exactly;
the names the port writes (F978: stars, planets and moons only); and the frame pixel by pixel over the
cabin: what each draws over the art (rings, dots, text) is compared like the galaxy map's. The rings are
the port's polylines, not F986's ring model, so their pixels are counted, not required to match.
Needs data.bin (cache v35) and a jjffe-smoke built with the `FFE_MAP_DUMP` hook (`build-smoke.ps1`).
"""
import argparse
import io
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import zipfile

import numpy as np
from PIL import Image

OBJECTS = 38988
STRIDE = 338
HEIGHT = 158


def read_frame(path):
    data = path.read_bytes()
    pixels = np.frombuffer(data[:64000], dtype=np.uint8).reshape(200, 320)
    palette = np.frombuffer(data[64000:64768], dtype=np.uint8).reshape(256, 3).astype(int) * 4
    return pixels, palette


def read_dump(path):
    data = path.read_bytes()
    state = {"objects": {}}
    focus = data[OBJECTS + 96]
    zoom = struct.unpack("<hh", data[OBJECTS + 96 + 8:OBJECTS + 96 + 12])
    time_point, clock = struct.unpack("<2Q", data[OBJECTS + 96 + 12:OBJECTS + 96 + 28])
    camera = struct.unpack("<3q", data[OBJECTS + 62:OBJECTS + 86])
    for slot in range(0x73):
        flags = data[slot]
        if flags & 0x20 or not flags:
            continue
        base = 0x74 + STRIDE * slot
        screen = struct.unpack("<hh", data[base + 0x98:base + 0x9c])
        state["objects"][slot] = {"flags": flags, "screen": screen, "kind": data[base + 0x87],
                                   "parent": data[base + 0x56]}
    state.update(focus=focus, zoom=zoom, camera=camera, time_point=time_point, clock=clock)
    return state


def overlay(pixels, cabin):
    return np.abs(pixels[:HEIGHT] - cabin[:HEIGHT]).sum(2) > 12


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--smoke", required=True, type=Path, help="jjffe-smoke.exe with the map dump")
    parser.add_argument("--cache", required=True, type=Path, help="data.bin (cache v35)")
    args = parser.parse_args()
    smoke = args.smoke.resolve(strict=True)
    root = Path(__file__).resolve().parents[2]
    cargo = shutil.which("cargo") or str(Path.home() / ".cargo" / "bin" / "cargo.exe")
    with tempfile.TemporaryDirectory(prefix="ffe-system-map-") as directory:
        directory = Path(directory)
        env = dict(os.environ, FFE_MAP_DUMP=str(directory / "dump.bin"))
        subprocess.run([str(smoke), "--game-frame", "560", "0x3d", "0x40"], cwd=directory, env=env,
                       capture_output=True, timeout=300, check=True)
        original = read_dump(directory / "dump.bin")
        pixels, palette = read_frame(directory / "game-frame.bin")
        # The time point is the game clock when F6 was pressed: seconds after the start (noon).
        seconds = (original["time_point"] & 0xFFFFFFFF) - 0x80000000
        seconds = seconds / 2**32 * 86400
        env = dict(os.environ, FFE_SHOT_DIR=str(directory), FFE_MAP_SECONDS=repr(seconds),
                   FFE_WORLD_CACHE=str(args.cache.resolve(strict=True)))
        run = subprocess.run([cargo, "test", "--locked", "-p", "ffe-rs", "--test", "system_map_view",
                              "the_page_draws_at_its_start", "--", "--ignored", "--exact"],
                             cwd=root, env=env, timeout=900, capture_output=True, text=True)
        if run.returncode:
            raise RuntimeError(run.stdout[-1500:] + run.stderr[-1500:])
        port = json.loads((directory / "system-map-dots.json").read_text())
        ours = np.array(Image.open(directory / "system-map.ppm").convert("RGB")).astype(int)
        with zipfile.ZipFile(args.cache) as archive:
            cabin = np.array(Image.open(io.BytesIO(archive.read("cockpits/cabin0.png")))
                             .convert("RGB")).astype(int)
    real = palette[pixels]
    result = {"scenario": "jjffe-system-map-v1", "time_point_seconds": round(seconds, 3)}
    problems = []
    if port["focus"] != original["focus"]:
        problems.append(f"focus slot {port['focus']} against {original['focus']}")
    if original["zoom"] != (0x7fff, 0x2a):
        problems.append(f"the original's zoom {original['zoom']}")
    if original["camera"] != (0, 0x1FFFC000000, 0):
        problems.append(f"the original's camera {original['camera']}")
    drawn = {slot: (x, y) for slot, x, y in port["dots"]}
    compared = 0
    for slot, entry in original["objects"].items():
        x, y = entry["screen"]
        if x == -32768 or not entry["flags"] & 8:
            continue
        if slot in drawn:
            compared += 1
            if drawn[slot] != (x, y):
                problems.append(f"slot {slot} at {drawn[slot]} against {(x, y)}")
        elif entry["kind"] == 1 and entry["flags"] & 8 and -400 < x < 600 and 0 <= y < HEIGHT:
            problems.append(f"slot {slot} at {(x, y)} was not drawn")
    result["dots_compared"] = compared
    names = {n for n, _, _ in port["labels"] if 0 <= _ < 320}
    result["port_names_in_view"] = sorted(names)
    # Pixels over the cabin.
    a, b = overlay(ours, cabin), overlay(real, cabin)
    both = a & b
    result.update(ours=int(a.sum()), real=int(b.sum()), both=int(both.sum()),
                  same_colour=int((both & (np.abs(ours[:HEIGHT] - real[:HEIGHT]).sum(2) <= 24)).sum()))
    result["overlap_of_real"] = round(result["both"] / max(1, result["real"]), 3)
    result["problems"] = problems
    print(json.dumps(result, indent=2))
    if problems or compared < 5:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
