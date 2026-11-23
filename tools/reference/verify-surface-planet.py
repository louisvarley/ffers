"""Check the saved planet-scale F1876 vectors (`--surface-draw`) against this port.

The full `--surface-draw` table (`tools/reference/verify-world.py --scenarios
surface-draw`) forks a child per case and uses `open_memstream`, so it is
generated on Linux. This script does not regenerate anything: it checks that
the saved table really holds planet-scale cases and runs the port's comparison
against it. On Windows the only surface records that can be produced in place
are the four near-pad ones (`--surface-near`, `verify-surface-near.py`).
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("--vectors", type=Path, required=True)
parser.add_argument("--cache", type=Path, required=True)
args = parser.parse_args()
root = Path(__file__).resolve().parents[2]
data = args.vectors.resolve().read_bytes()
rows = [json.loads(line) for line in data.splitlines() if line.strip()]
planet_scale = 0
for row in rows:
    if row["fault"]:
        continue
    vectors = row["vectors"]
    radius = sum(
        (vectors[3 * k] ** 2 + vectors[3 * k + 1] ** 2 + vectors[3 * k + 2] ** 2) ** 0.5
        for k in range(3)
    ) / 3
    origin = row["origin"]
    distance = sum(coordinate * coordinate for coordinate in origin) ** 0.5
    if radius > 0 and distance > 1.4 * radius:
        planet_scale += 1
assert planet_scale >= 40, f"only {planet_scale} planet-scale cases in {args.vectors}"
env = os.environ.copy()
env["FFE_SURFACE_VECTORS"] = str(args.vectors.resolve())
env["FFE_WORLD_CACHE"] = str(args.cache.resolve())
subprocess.run(["cargo", "test", "-p", "ffe-rs", "--test", "surface_draw_reference",
                "executed_reference_planet_surface_draw", "--", "--ignored"],
               cwd=root, env=env, check=True)
print(json.dumps({"scenario": "planet-scale-surface-draw-v1", "cases": len(rows),
                  "planet_scale_cases": planet_scale,
                  "sha256": hashlib.sha256(data).hexdigest()}, indent=2))
