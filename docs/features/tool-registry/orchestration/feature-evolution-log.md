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

## Downstream Artifact Migration

- existing system-agent APIs stay in place
- new internal-agent APIs are parallel rather than mutating system-agent row semantics

## Residual Risks

- runtime and frontend may temporarily diverge if direct-list behavior lands before domain-discovery seams are fully integrated.
