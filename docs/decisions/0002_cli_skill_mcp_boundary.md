# ADR 0002: CLI First, Skill as Guidance, MCP/UI as Optional

Date: 2026-08-13  
Status: Accepted for implementation planning

## Context

Build pipelines need a local deterministic command, while ChatGPT/Codex users may benefit from structured forms and confirmation. A remote UI alone cannot guarantee access to arbitrary local files on every host.

## Decision

Make the Rust CLI/library the normative interface. Ship a Codex Skill that orchestrates the CLI. Add an MCP server and MCP Apps UI only as an optional adapter; every MCP tool must remain useful without rendering a component.

## Consequences

- CI and local agents share one behavior and one error vocabulary.
- UI can improve inspection and confirmation without becoming a required runtime.
- File upload support is capability-detected, not promised universally.

