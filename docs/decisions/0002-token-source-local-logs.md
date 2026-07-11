---
type: decision
status: accepted
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
tags:
  - decision
  - architecture
---

# Decision: Track tokens by tailing local provider logs, behind a provider-plugin trait

## Status

Accepted

## Date

2026-07-11

## Context

The core loop needs near-realtime token counts. Options: parse local logs of tools like Claude Code, call provider usage APIs, or proxy API traffic. The user chose Claude Code as the primary source with multi-LLM support later.

## Decision

Define a `TokenProvider` trait (see [[../knowledge/token-tracking|Token Tracking]]). v1 ships two providers: **Claude Code** (tail `~/.claude/projects/**/*.jsonl` usage fields) and **Manual/Demo**. Other LLMs (Codex CLI, OpenAI Usage API) arrive as v2 plugins behind the same trait.

## Consequences

Positive:

- Zero configuration and zero network for the primary path; realtime (food appears seconds after usage).
- No API keys stored; only numeric usage fields read — strong privacy stance.
- Plugin trait isolates unstable third-party formats from the economy engine.

Negative or tradeoffs:

- Claude Code log schema is undocumented and may change between versions; parsers must be defensive, fixture-tested, and quick to patch.
- Raw OpenAI API usage has no local logs — its plugin will be delayed/polled (acceptable: food arrives late).

## Alternatives Considered

- **Anthropic Admin/Usage API:** needs org admin key, minutes of delay, network dependency — poor fit for an ambient toy.
- **Local LLM proxy (man-in-the-middle):** accurate and provider-agnostic but invasive setup and a security liability.
- **OpenTelemetry export from Claude Code:** clean official channel; kept as documented fallback if log parsing breaks.

## References

- [[../knowledge/token-tracking|Token Tracking]]
- [[../architecture|Architecture]]
