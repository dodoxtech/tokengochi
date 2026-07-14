---
type: guide
status: active
created: 2026-07-10
updated: 2026-07-13
tags:
  - knowledge
  - ai-context
---

# Knowledge Base

Use this folder for reusable notes that are not tasks, decisions, or sprint records.

Good knowledge notes include:

- Domain research.
- API references.
- Product assumptions.
- User research.
- Glossary entries.
- Debugging notes that may help future agents.

Keep notes small and link them to related tasks, decisions, or architecture docs.

Start with [[code-map|Code Map]] before touching pet movement, behavior AI,
or any Tauri command — it indexes exactly which function in which file owns
which piece of behavior, so you don't have to grep the whole tree.

See [[sprite-asset-pipeline|Sprite Asset Pipeline]] before adding or
regenerating any sprite — it documents the scripts in
`ui/assets/sprites/scripts/` and the chroma-key threshold pitfall.

See [[pet-action-pack-spec|Pet Action Pack Spec]] before adding a new pet
form or a new gag/expression — it defines the fixed tag set a pet form
must ship to be a drop-in character swap with zero code changes.

See [[agent-status-notifications|Agent Status Notifications]] before touching
the Claude Code hook bridge or the pet's turn-completed/needs-approval badge
— it documents the local file bridge and the hook JSON to install.

