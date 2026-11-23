#!/usr/bin/env python3
"""Check that docs/BURNDOWN.md, docs/STATUS.md and dashboard/burndown.js agree.

CLAUDE.MD asks that every change to milestone progress updates the dashboard in
the same commit. This catches the usual slip: the dashboard changed and a summary
page did not (or the last history point was not appended).

Checks, all read-only:
  * the task count and the done / in-progress / todo counts in burndown.js
    against the summary line and the totals row of docs/BURNDOWN.md;
  * the last history point's `remaining` against the open count;
  * every task id has one status from the known set and no id repeats;
  * every item BURNDOWN.md lists as open ([~] or [ ]) is open in burndown.js.

Usage: python ci/check_burndown.py   (exit 0 when consistent, 1 otherwise)
"""
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
STATUSES = {"done", "in-progress", "todo"}


def parse_tasks(text):
    """Return [(id, status)] for every task line (id, title and status together)."""
    tasks = []
    for line in text.splitlines():
        ident = re.search(r'"?\bid"?\s*:\s*"([^"]+)"', line)
        status = re.search(r'"?\bstatus"?\s*:\s*"([^"]+)"', line)
        if ident and status and re.search(r'"?\btitle"?\s*:', line):
            tasks.append((ident.group(1), status.group(1)))
    return tasks


def last_history_remaining(text):
    points = re.findall(r'remaining\s*:\s*(\d+)', text[text.index("history"):])
    return int(points[-1]) if points else None


def check(root=ROOT):
    problems = []
    js = (root / "dashboard" / "burndown.js").read_text(encoding="utf-8")
    md = (root / "docs" / "BURNDOWN.md").read_text(encoding="utf-8")
    tasks = parse_tasks(js)
    ids = [ident for ident, _ in tasks]
    for ident in sorted({i for i in ids if ids.count(i) > 1}):
        problems.append(f"task id {ident} appears more than once in burndown.js")
    for ident, status in tasks:
        if status not in STATUSES:
            problems.append(f"task {ident} has unknown status {status!r}")
    counts = Counter(status for _, status in tasks)
    total, done = len(tasks), counts["done"]
    progress, todo = counts["in-progress"], counts["todo"]
    open_items = total - done

    summary = re.search(
        r"\*\*(\d+) of (\d+) work items are open: (\d+) in progress, (\d+) not started",
        md,
    )
    if not summary:
        problems.append("BURNDOWN.md has no '**N of M work items are open: ...' summary")
    else:
        want = tuple(map(int, summary.groups()))
        have = (open_items, total, progress, todo)
        if want != have:
            problems.append(
                f"BURNDOWN.md summary says open/total/in-progress/not-started {want}, "
                f"burndown.js has {have}"
            )
    row = re.search(r"\| \*\*Total\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \| \*\*(\d+)\*\* \| \*\*(\d+) / (\d+)\*\*", md)
    if row:
        have_row = tuple(map(int, row.groups()))
        want_row = (open_items, progress, todo, done, total)
        if have_row != want_row:
            problems.append(f"BURNDOWN.md totals row {have_row} differs from burndown.js {want_row}")
    else:
        problems.append("BURNDOWN.md has no totals row")

    remaining = last_history_remaining(js)
    if remaining != open_items:
        problems.append(
            f"the last history point says remaining {remaining}, but {open_items} tasks are open"
        )

    status_of = dict(tasks)
    for mark, ident in re.findall(r"^- \[(.)\] `([^`]+)`", md, re.MULTILINE):
        if ident not in status_of:
            problems.append(f"BURNDOWN.md lists {ident}, which burndown.js does not have")
        elif mark != "x" and status_of[ident] == "done":
            problems.append(f"BURNDOWN.md lists {ident} as open, but burndown.js has it done")
        elif mark == "x" and status_of[ident] != "done":
            problems.append(f"BURNDOWN.md ticks {ident}, but burndown.js has it {status_of[ident]}")

    status_md = root / "docs" / "STATUS.md"
    if status_md.exists():
        match = re.search(r"\((\d+) of (\d+) open on", status_md.read_text(encoding="utf-8"))
        if match and (int(match.group(1)), int(match.group(2))) != (open_items, total):
            problems.append(
                f"STATUS.md says {match.group(1)} of {match.group(2)} open, "
                f"burndown.js has {open_items} of {total}"
            )
    return problems, (total, done, progress, todo)


def main():
    problems, (total, done, progress, todo) = check()
    if problems:
        print("burndown is inconsistent:", file=sys.stderr)
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        return 1
    print(f"burndown consistent: {total} items, {done} done, {progress} in progress, {todo} not started")
    return 0


if __name__ == "__main__":
    sys.exit(main())
