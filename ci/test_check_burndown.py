import tempfile
import unittest
from pathlib import Path

from ci import check_burndown

JS = """window.FFE_BURNDOWN = {
  milestones: [{
    id: "m",
    title: "Milestone",
    status: "planned",
    tasks: [
      { id: "a-1", title: "One", status: "done" },
      { id: "a-2", title: "Two", status: "in-progress" },
      { id: "a-3", title: "Three", status: "todo" }
    ]
  }],
  history: [
    { date: "2026-01-01", remaining: 5, note: "old" },
    { date: "2026-01-02", remaining: 2, note: "new" }
  ]
};
"""

MD = """# Burndown

**2 of 3 work items are open: 1 in progress, 1 not started.**

| Area | Open | In progress | Not started | Done / total |
|---|---:|---:|---:|---:|
| **Total** | **2** | **1** | **1** | **1 / 3** |

- [x] `a-1` One
- [~] `a-2` Two
- [ ] `a-3` Three
"""


def tree(js=JS, md=MD):
    root = Path(tempfile.mkdtemp())
    (root / "dashboard").mkdir()
    (root / "docs").mkdir()
    (root / "dashboard" / "burndown.js").write_text(js, encoding="utf-8")
    (root / "docs" / "BURNDOWN.md").write_text(md, encoding="utf-8")
    return root


class CheckBurndown(unittest.TestCase):
    def test_a_consistent_tree_passes(self):
        problems, counts = check_burndown.check(tree())
        self.assertEqual(problems, [])
        self.assertEqual(counts, (3, 1, 1, 1))

    def test_a_stale_summary_is_reported(self):
        problems, _ = check_burndown.check(tree(md=MD.replace("**2 of 3", "**3 of 3")))
        self.assertTrue(any("summary" in p for p in problems), problems)

    def test_a_missing_history_point_is_reported(self):
        problems, _ = check_burndown.check(tree(js=JS.replace("remaining: 2", "remaining: 3")))
        self.assertTrue(any("history" in p for p in problems), problems)

    def test_a_status_disagreement_and_a_duplicate_id_are_reported(self):
        md = MD.replace("- [~] `a-2`", "- [x] `a-2`")
        problems, _ = check_burndown.check(tree(md=md))
        self.assertTrue(any("a-2" in p for p in problems), problems)
        js = JS.replace('id: "a-3"', 'id: "a-1"')
        problems, _ = check_burndown.check(tree(js=js))
        self.assertTrue(any("more than once" in p for p in problems), problems)

    def test_the_real_repository_is_consistent(self):
        problems, _ = check_burndown.check()
        self.assertEqual(problems, [])


if __name__ == "__main__":
    unittest.main()
