---
type: process
status: active
created: 2026-07-10
updated: 2026-07-10
tags:
  - agile
  - scrum
  - process
  - ai-context
---

# Agile and Scrum Workflow

This project uses a lightweight Scrum-inspired workflow optimized for humans and AI agents.

## Core Artifacts

- Product context: [[product]]
- Product backlog: [[tasks/backlog/README]]
- Active work: [[tasks/README]]
- Sprint notes: [[sprints/README]]
- Decisions: [[decisions/README]]

## Work States

- Backlog: task is captured but not currently being implemented.
- Ready: task has enough context and acceptance criteria to start.
- Active: task is being implemented in `docs/tasks/active/`.
- Review: implementation exists and needs verification or review.
- Done: task is verified and moved to `docs/tasks/done/`.

## Scrum Ceremonies

- Sprint planning: choose a sprint goal and tasks from backlog.
- Daily check-in: record blockers and next steps when useful.
- Sprint review: summarize completed work and product changes.
- Retrospective: capture process improvements and follow-up tasks.

## Definition of Ready

A task is ready when it has:

- Clear goal.
- Scope and non-scope.
- Acceptance criteria.
- Relevant references or links.
- Known dependencies and blockers.

## Definition of Done

A task is done when:

- Acceptance criteria are checked.
- Relevant tests, builds, or manual verification are recorded.
- Docs are updated.
- Follow-ups are captured as backlog tasks or explicit notes.
- The task file is moved to `docs/tasks/done/`.

## AI Agent Workflow

1. Search `docs/` for related context.
2. Read linked product, architecture, sprint, and decision notes.
3. Create or update a task using [[templates/task]].
4. Move the task into `docs/tasks/active/` while implementing.
5. Update verification and completion notes.
6. Move completed work to `docs/tasks/done/`.
