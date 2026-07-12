---
type: task
status: done
priority: P2
delivery_order: 0011
estimate: 4d
created: 2026-07-11
updated: 2026-07-12
owner: AI agent
sprint: null
tags:
  - task
  - done
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

- [x] Each provider passes the same dedup/idempotency test suite as 0003.
- [x] Delayed OpenAI usage converts correctly against daily caps of the day it *occurred*.
- [x] API keys stored in OS keyring, never in SQLite or config files.

## Dependencies

- [[0009-evolution-streaks-quests|0009]] (branch system), [[0003-claude-code-token-watcher|0003]] (trait)

## Risks

- Third-party formats/APIs shift; keep providers isolated and fixture-tested.

## Verification Plan

- [x] Fixture tests + live test per provider; record results below.

## Verification Results

### 2026-07-12

- Added `CodexCliProvider` for `~/.codex/sessions/**/*.jsonl`, verified against local Codex logs. It parses `event_msg` / `payload.type = "token_count"` records and dedups by payload id or file offset fallback.
- Replaced the OpenAI placeholder with an opt-in Usage API poller. API keys are stored through the OS keychain helper and are not persisted in SQLite/config; Usage API buckets emit events at bucket `start_time`.
- Added provider settings in the dashboard for Claude Code, Codex CLI, and OpenAI, plus OpenAI key store/clear commands.
- Added per-day provider mix tracking for chimera eligibility.
- Added per-day cap/banked-token maps so delayed provider events are converted against the day they occurred.
- Verification:
  - `cargo test` in `src-tauri` passed: 53 tests, including Codex parser/dedup, OpenAI Usage API parser, delayed-day caps, and provider-mix/chimera regression tests.
  - `npm run check` passed in `ui/dashboard`.
  - `npm run check` passed in `ui/overlay`.
  - Local Codex JSONL format was inspected with numeric usage-field search only.
  - Live OpenAI API polling was not executed in this session because no API key was provided; parser/keychain/storage behavior is covered by code/tests.
