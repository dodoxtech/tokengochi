---
type: guide
status: active
created: 2026-07-10
updated: 2026-07-10
tags:
  - tasks
  - agile
  - scrum
  - ai-context
---

# Tasks

Tasks are lightweight working documents for agents and developers.

## Folders

- `backlog/` - product backlog items, user stories, technical work, and follow-ups.
- `active/` - work that is planned, in progress, or ready for an agent to pick up.
- `done/` - completed work with verification and completion notes.

## Creating a Task

Copy `docs/templates/task.md` into `docs/tasks/backlog/` and name it:

```text
NNNN-short-kebab-title.md
```

`NNNN` is the next four-digit delivery-order number, such as `0008`. It tells a human or agent the recommended implementation sequence at a glance. Add the same value to `delivery_order` in frontmatter. Do not renumber existing or completed tasks; gaps are acceptable.

Use `docs/templates/user-story.md` when the work represents user-facing product value.

Fill in the template before implementation starts. It is fine for some sections to say `TBD`, but acceptance criteria and verification should become concrete before the task moves to `active/`.

## Starting Work

Move the task from `backlog/` to `active/`, update the status in frontmatter, and add implementation notes as work progresses.

## Completing a Task

Before moving a task to `docs/tasks/done/`:

- Mark acceptance criteria as complete.
- Add implementation notes with changed files.
- Add verification commands and results.
- Record follow-ups, if any.
