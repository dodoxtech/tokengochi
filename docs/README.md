---
type: index
status: active
created: 2026-07-10
updated: 2026-07-10
tags:
  - docs
  - ai-context
---

# Documentation

This folder is the source of truth for product context, engineering standards, architecture notes, decisions, and task tracking.

For Obsidian navigation, start with [[index]].

## Structure

- `index.md` - Obsidian map of content.
- `product.md` - what the project is, who it serves, goals, and non-goals.
- `architecture.md` - technical structure, system boundaries, and integration notes.
- `engineering.md` - coding standards, testing expectations, and delivery checklist.
- `agile.md` - Agile/Scrum workflow and project management rules.
- `decisions/` - ADR-style decision records.
- `sprints/` - sprint plans, goals, reviews, and retrospectives.
- `knowledge/` - reusable research, references, and domain knowledge.
- `tasks/backlog/` - prioritized backlog tasks.
- `tasks/active/` - current or planned implementation tasks.
- `tasks/done/` - completed tasks with verification notes.
- `templates/` - reusable templates for new tasks and decision records.

## Task Lifecycle

1. Create a task from `docs/templates/task.md`.
2. Put it in `docs/tasks/backlog/` unless implementation starts immediately.
3. Move it to `docs/tasks/active/` when work starts.
4. Keep the task updated as implementation progresses.
5. Verify the work using the checklist in the task.
6. Move the completed task to `docs/tasks/done/`.

Use file names like `2026-07-10-add-docs-workflow.md` so tasks are easy to sort chronologically.

## Obsidian Usage

- Use `[[wiki links]]` to connect tasks, decisions, architecture notes, and sprint notes.
- Use YAML frontmatter for filtering and Dataview-style queries.
- Use tags consistently: `#task`, `#backlog`, `#active`, `#done`, `#sprint`, `#decision`, `#product`, `#architecture`, `#engineering`.
- Keep each note focused on one topic.
- Put broad navigation links in [[index]].

## Writing Expectations

- Be specific enough that another agent can continue the work without guessing.
- Link to related files, decisions, issues, and commands where useful.
- Prefer short, current notes over long stale documentation.
- Update docs in the same change when behavior, architecture, or workflow changes.
