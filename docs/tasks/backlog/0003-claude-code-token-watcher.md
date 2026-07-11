---
type: task
status: backlog
priority: P0
delivery_order: 0003
estimate: 3d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
---

# Task: Claude Code token watcher (JSONL tailing provider)

## Context

Primary token source per [[../../decisions/0002-token-source-local-logs|ADR-0002]]; design in [[../../knowledge/token-tracking|Token Tracking]].

## Goal

A `TokenProvider` implementation that emits accurate, deduplicated `TokenEvent`s from live Claude Code sessions within seconds of usage.

## Scope

In scope: `TokenProvider` trait + `TokenEvent`; directory watching of `~/.claude/projects` with `notify`; incremental JSONL tailing with persisted byte offsets; parsing `message.usage` fields; dedup by message id; fixture corpus from the currently installed Claude Code version; Manual/Demo provider.

Out of scope: other LLM providers, economy conversion.

## Acceptance Criteria

- [ ] Live Claude Code session produces events within 5s; restart does not double-count (offsets + dedup verified by test).
- [ ] Parser tolerates unknown fields and malformed lines (fixture tests).
- [ ] Only numeric usage fields and ids are read — content never leaves the parser (code-reviewed).

## Dependencies

- [[0001-scaffold-tauri-project|0001]]

## Risks

- Undocumented log schema; verify field names against installed version first and record them in [[../../knowledge/token-tracking|Token Tracking]].

## Verification Plan

- [ ] Unit tests on fixtures; manual live-session test; record results below.

## Verification Results

TBD
