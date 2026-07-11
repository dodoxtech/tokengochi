# CLAUDE.md

Guidance for AI agents working in this repository.

## Fast Start

Before changing code or docs:

1. Read `docs/README.md`.
2. Search `docs/` first for product, architecture, task, or decision context.
3. Check `docs/tasks/active/` for work already in progress.
4. Check `docs/tasks/backlog/` before creating duplicate tasks.
5. Use `docs/templates/` when creating a new project artifact.

## Documentation Map

Start with the `docs/` folder before making implementation decisions:

- `docs/README.md` - documentation index and workflow.
- `docs/index.md` - Obsidian map of content for quick navigation.
- `docs/product.md` - product context, goals, users, and non-goals.
- `docs/architecture.md` - technical structure and system decisions.
- `docs/engineering.md` - development standards, testing, and review expectations.
- `docs/agile.md` - Agile/Scrum process used by this project.
- `docs/decisions/` - Architecture Decision Records (ADRs).
- `docs/sprints/` - sprint goals, plans, reviews, and retrospectives.
- `docs/knowledge/` - reusable notes, research, and domain references.
- `docs/tasks/backlog/` - prioritized product backlog items and technical tasks.
- `docs/tasks/active/` - tasks currently in progress or ready to pick up.
- `docs/tasks/done/` - completed tasks, moved here after verification.
- `docs/templates/` - templates agents should copy when creating new docs or tasks.

## Task Workflow

When creating a new task:

1. Copy `docs/templates/task.md`.
2. Save it in `docs/tasks/backlog/` by default, or `docs/tasks/active/` if work starts immediately, using the format:
   `NNNN-short-kebab-title.md`, where `NNNN` is the next four-digit delivery-order number.
3. Fill in frontmatter, context, acceptance criteria, implementation notes, and references.
4. Link related notes with Obsidian wiki links, for example `[[architecture]]` or `[[docs/tasks/README|Tasks]]`.
5. Move backlog tasks into `docs/tasks/active/` when implementation starts.
6. Keep the task updated while work is in progress.
7. After implementation and verification, move the task file to `docs/tasks/done/`.
8. Add a completion note with what changed, how it was verified, and any follow-up work.
9. Assign a `delivery_order` frontmatter value. Lower numbers are the recommended implementation order; do not renumber completed tasks.

## Obsidian Conventions

- Every important doc should include YAML frontmatter with `type`, `status`, `created`, `updated`, `tags`, and `owner` when relevant.
- Use tags for filtering, such as `#task`, `#sprint`, `#decision`, `#architecture`, `#product`, `#engineering`, and `#ai-context`.
- Use wiki links for important cross-references.
- Keep titles human-readable. File names should remain lowercase kebab-case.
- Prefer one concept per note so both AI search and Obsidian backlinks stay useful.
- Write all documentation, including tasks, ADRs, comments, and frontmatter values, in English.

## Agile/Scrum Conventions

- Product backlog lives in `docs/tasks/backlog/`.
- Sprint work lives in `docs/sprints/` and references tasks by link.
- Active implementation lives in `docs/tasks/active/`.
- Completed work lives in `docs/tasks/done/`.
- A task should have acceptance criteria before implementation is considered ready.

## Agent Rules

- Read relevant docs before editing code.
- Prefer updating existing docs over creating duplicate sources of truth.
- Capture meaningful product or technical decisions in `docs/decisions/`.
- Do not leave finished work in `docs/tasks/active/`.
- Do not create duplicate backlog items; search first.
- If docs and code disagree, treat it as a task: update the stale source or call out the mismatch.
- Keep task files useful for the next agent: include file paths, commands run, test results, and unresolved questions.
