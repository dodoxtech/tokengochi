---
type: engineering
status: active
created: 2026-07-10
updated: 2026-07-10
tags:
  - engineering
  - process
  - ai-context
---

# Engineering Guide

Use this guide for day-to-day development expectations.

## Development Principles

- Keep changes small, focused, and easy to review.
- Follow existing project conventions before introducing new patterns.
- Update docs and tasks when behavior or architecture changes.
- Prefer clear names and direct code over unnecessary abstraction.

## Before Starting Work

- Read `CLAUDE.md`.
- Check `docs/tasks/active/` for the relevant task.
- Review `docs/product.md`, `docs/architecture.md`, and recent files in `docs/decisions/` when the change may affect product or architecture.

## Before Finishing Work

- Run the relevant formatter, linter, tests, or build commands.
- Record verification results in the task file.
- Move completed task files from `docs/tasks/active/` to `docs/tasks/done/`.
- Note follow-up work explicitly instead of burying it in implementation comments.

## Testing Expectations

- Add or update tests when behavior changes.
- For UI changes, verify the affected screen at relevant viewport sizes.
- For bug fixes, include a regression check where practical.
- If a verification step cannot be run, document why in the task.

## Code Review Checklist

- Behavior matches the task acceptance criteria.
- The implementation is scoped to the requested work.
- Edge cases and error states are handled.
- Tests or manual verification cover the risky parts.
- Docs are updated when the change affects workflow, architecture, or product behavior.
