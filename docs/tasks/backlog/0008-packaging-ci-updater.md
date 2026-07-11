---
type: task
status: backlog
priority: P1
delivery_order: 0008
estimate: 3d
created: 2026-07-11
updated: 2026-07-11
owner: AI agent
sprint: null
tags:
  - task
  - backlog
---

# Task: Packaging, signing, auto-update — MVP release

## Context

Ship the MVP (tasks 0001–0007) as installable builds. Targets and channels in [[../../architecture|Architecture]] §Runtime and Deployment.

## Goal

Tagged commits produce signed installers for all three OS with working auto-update.

## Scope

In scope: `tauri bundle` artifacts (.msi/NSIS, .dmg, .deb + AppImage); GitHub Actions release workflow; `tauri-plugin-updater` against GitHub Releases; macOS notarization and Windows code signing (or documented unsigned-MVP decision); version/changelog process.

Out of scope: Homebrew/winget/Snap (backlog later per demand).

## Acceptance Criteria

- [ ] `git tag vX.Y.Z` → CI publishes installers for all 3 OS.
- [ ] Auto-update from previous version verified on each OS; pet state survives update.
- [ ] Install docs in README.

## Dependencies

- [[0007-tray-settings-dashboard|0007]]

## Risks

- Signing certificates cost/lead time — decide early; unsigned macOS builds trigger Gatekeeper friction.

## Verification Plan

- [ ] Full release rehearsal from a test tag; record results below.

## Verification Results

TBD
