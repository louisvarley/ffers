"""Tests for tools/next-version.py (the release workflow's version rules)."""
import importlib.util
import tempfile
import unittest
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "next_version", Path(__file__).resolve().parent.parent / "tools" / "next-version.py"
)
nv = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(nv)


class NextVersion(unittest.TestCase):
    def test_the_first_release_is_the_cargo_version_as_is(self):
        self.assertEqual(nv.next_version(None, "0.9.0", "patch"), ((0, 9, 0), False))

    def test_a_push_after_a_release_bumps_the_patch(self):
        self.assertEqual(nv.next_version("v0.9.0", "0.9.0", "patch"), ((0, 9, 1), True))
        self.assertEqual(nv.next_version("v0.9.7", "0.9.7", "patch"), ((0, 9, 8), True))

    def test_minor_and_major_bumps(self):
        self.assertEqual(nv.next_version("v0.9.4", "0.9.4", "minor"), ((0, 10, 0), True))
        self.assertEqual(nv.next_version("v0.9.4", "0.9.4", "major"), ((1, 0, 0), True))

    def test_a_hand_bumped_cargo_version_is_released_as_is(self):
        self.assertEqual(nv.next_version("v0.9.4", "0.10.0", "patch"), ((0, 10, 0), False))

    def test_a_stale_cargo_version_follows_the_latest_tag(self):
        self.assertEqual(nv.next_version("v0.9.9", "0.9.0", "patch"), ((0, 9, 10), True))

    def test_numbers_compare_as_numbers_not_text(self):
        self.assertEqual(nv.next_version("v0.9.9", "0.9.9", "patch"), ((0, 9, 10), True))
        self.assertEqual(nv.next_version("v0.10.0", "0.9.9", "patch"), ((0, 10, 1), True))

    def test_the_commit_message_picks_the_bump(self):
        self.assertEqual(nv.bump_kind("Fix a bug"), "patch")
        self.assertEqual(nv.bump_kind("New feature #minor"), "minor")
        self.assertEqual(nv.bump_kind("Rewrite #MAJOR"), "major")
        self.assertEqual(nv.bump_kind("Fix", "minor"), "minor")
        self.assertEqual(nv.bump_kind("Fix #major", "patch"), "patch")

    def test_the_version_is_written_to_both_crates_and_the_lock(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for crate in nv.CRATES:
                (root / "crates" / crate).mkdir(parents=True)
                (root / "crates" / crate / "Cargo.toml").write_text(
                    f'[package]\nname = "{crate}"\nversion = "0.9.0"\nedition = "2021"\n'
                    '[dependencies]\nx = { version = "1" }\n'
                )
            (root / "Cargo.lock").write_text(
                '[[package]]\nname = "ffe-launcher"\nversion = "0.9.0"\n\n'
                '[[package]]\nname = "ffe-rs"\nversion = "0.9.0"\n\n'
                '[[package]]\nname = "other"\nversion = "0.9.0"\n'
            )
            nv.apply_version((0, 9, 1), root)
            for crate in nv.CRATES:
                text = (root / "crates" / crate / "Cargo.toml").read_text()
                self.assertIn('version = "0.9.1"', text)
                self.assertIn('x = { version = "1" }', text)
            lock = (root / "Cargo.lock").read_text()
            self.assertIn('name = "ffe-rs"\nversion = "0.9.1"', lock)
            self.assertIn('name = "ffe-launcher"\nversion = "0.9.1"', lock)
            self.assertIn('name = "other"\nversion = "0.9.0"', lock)
            self.assertEqual(nv.cargo_version(root), "0.9.1")

    def test_the_real_workspace_can_take_a_version(self):
        # Only checks that the patterns match the real files (no write).
        self.assertIsNotNone(nv.parse(nv.cargo_version()))
        lock = (nv.ROOT / "Cargo.lock").read_text(encoding="utf-8")
        for crate in nv.CRATES:
            self.assertRegex(lock, rf'\[\[package\]\]\nname = "{crate}"\nversion = "[^"]+"')


if __name__ == "__main__":
    unittest.main()
