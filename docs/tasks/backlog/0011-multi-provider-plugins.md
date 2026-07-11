---
type: task
status: backlog
priority: P2
delivery_order: 0011
estimate: 4d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
---

# Task: Multi-LLM provider plugins (Codex CLI, OpenAI Usage API)

## Context

User-selected multi-LLM support behind the `TokenProvider` trait ([[../../decisions/0002-token-source-local-logs|ADR-0002]], [[../../knowledge/token-tracking|Token Tracking]]). Unlocks the chimera evolution branch.

## Goal

Users of Codex CLI and the OpenAI API can feed the pet; provider mix is tracked for branching.

## Scope

In scope: Codex CLI session-log provider (verify current format first, fixtures like 0003); OpenAI Usage API provider (opt-in, keyring-stored key, polling with delay handling and dedup); settings UI for enabling providers; per-provider stats; chimera branch eligibility.

Out of scope: local proxy mode, Cursor/other tools (add per demand).

## Acceptance Criteria

- [ ] Each provider passes the same dedup/idempotency test suite as 0003.
- [ ] Delayed OpenAI usage converts correctly against daily caps of the day it *occurred*.
- [ ] API keys stored in OS keyring, never in SQLite or config files.

## Dependencies

- [[0009-evolution-streaks-quests|0009]] (branch system), [[0003-claude-code-token-watcher|0003]] (trait)

## Risks

- Third-party formats/APIs shift; keep providers isolated and fixture-tested.

## Verification Plan

- [ ] Fixture tests + live test per provider; record results below.

## Verification Results

TBD
