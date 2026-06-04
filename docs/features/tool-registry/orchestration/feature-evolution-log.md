# Feature Evolution Log: Tool Registry

## Summary

- feature_slug: tool-registry

This feature evolves the earlier agent-management and runtime assumptions:

- workspace `/agents` was previously system-agent only
- `main-agent` was intentionally absent from workspace agent rows
- runtime tools were a flat directly supplied list

## Change History

- 2026-06-02: reopened the workspace `/agents` contract to add internal-agent tool policy while preserving the existing system-agent section.
- 2026-06-02: introduced the assigned-tools versus materialized-tools runtime distinction as a required runtime evolution.
- 2026-06-04: reopened the original v1 business-only boundary to add a bounded coding domain (`bash`, `fs`, `search`) for `main-agent` without routing through `codex`/`opencode` black-box tool catalogs.
- 2026-06-04: tightened tool-schema contract so cataloged tool input JSON must carry top-level `description` and `examples`.

## Downstream Artifact Migration

- existing system-agent APIs stay in place
- new internal-agent APIs are parallel rather than mutating system-agent row semantics
- existing tool catalog APIs stay in place while their response set expands to include coding domains and coding tools

## Residual Risks

- runtime and frontend may temporarily diverge if direct-list behavior lands before domain-discovery seams are fully integrated.
- host-backed coding tools can widen the default `all_platform_tools` surface for `main-agent`, so future iterations may still need explicit assignment-mode tightening or permission layering.
