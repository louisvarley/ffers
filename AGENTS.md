# Working rules: finish existing backlog items efficiently

The owner wants completed existing backlog items, with low token overhead.
Read `docs/WORK_HANDOFF.md` first. Read the relevant parts of `CLAUDE.MD` for
project direction; preserve its source-baseline, local-data and scope rules.
Do not reread the full history, port log or backlog on every turn.

## Scope and completion

- Work on ONE existing open dashboard ID at a time. A request for ten items
  means ten existing IDs changing from open to done, not ten new subtasks.
- Before editing, write a short task contract in the handoff: ID, current gap,
  acceptance criteria, relevant files, and the smallest verification command.
- Reuse existing implementation, tests and source fixtures before building a
  new harness. Choose items by readiness and verification cost, while respecting
  dependencies and the owner's requested priorities.
- Do not weaken acceptance criteria, rename scope, close a broad item on a
  narrow fix, or add dashboard tasks to satisfy a requested completion count.
  Record discovered gaps as notes under the existing item.
- Keep dashboard totals fixed during an existing-backlog completion batch.
  Change an existing status only when all its acceptance criteria are met.
  Append the evidence and new remaining count; update summary documents once
  per completed item, not after each experimental edit.

## Investigation and implementation

- Start with the current diff and a bounded `rg` search. Read specific functions
  and relevant documentation sections, not whole large files.
- Aim for at most two initial discovery calls, with about 2,000 output tokens
  per call. Exceed this only for an identified missing fact or required routine.
- Preserve unrelated and previously uncommitted changes. Do not reset, clean,
  commit or push mixed work just to make the checkout look tidy.
- If an item requires a new subsystem, revise the task contract and estimate
  before expanding edits. Do not silently begin a second backlog item.
- After two failed targeted attempts, reassess the hypothesis before editing
  again. Avoid speculative fixes and repeated unchanged test runs.
- Use one worker by default; delegate only when explicitly authorized.

## Verification and reporting

- During implementation, run the smallest relevant test. Use existing local
  reference binaries/fixtures when their inputs have not changed. Check the
  build manifest before rebuilding; rebuild only affected components.
- At the completion gate, run applicable source comparisons, targeted regression
  tests, formatting, and strict lint/build checks. Run broader suites only when
  the actual change or a failure justifies them. Capture large output in ignored
  local logs and return only the result or useful error excerpt.
- Report evidence precisely: distinguish unit tests, executed-source comparisons,
  UI checks and compilation. Do not claim one proves another.
- Report a closed item as: ID; delivered behavior; evidence; remaining count.
  Partial progress is not a completed item.
- Keep the handoff short and current so a fresh chat can continue without this
  conversation. Record only the active contract, changed files, verified results,
  unresolved facts and next command. Link historical detail rather than copying it.
- Treat token limits supplied by the owner as explicit constraints. Do not invent
  exact token usage or claim a hard cap is enforced without measured usage.
