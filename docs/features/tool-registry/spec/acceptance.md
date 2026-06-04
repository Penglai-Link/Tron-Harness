# Acceptance: Tool Registry

## Acceptance Index

- AC-TOOL-01 [must] [api] Platform tool catalog APIs expose stable code-defined business and coding domains with concrete schemas that include top-level `description` and `examples`; proof: backend tests and final review show catalog responses match registered definitions.
- AC-TOOL-02 [must] [business] Workspace internal-agent policy resolves `main-agent` defaults, overrides, and reset behavior without regressing system-agent APIs; proof: backend tests and final review show effective policy resolution and preserved system-agent behavior.
- AC-TOOL-03 [must] [api] Main-agent direct-list runs derive policy-assigned coding tools and execute bounded `fs.*`, `search.*`, and `bash.exec` tools through registered executors; proof: backend tests and final review show runtime materialization and execution behavior.
- AC-TOOL-04 [must] [interaction] `/settings/tools` renders the read-only platform business tool catalog with domain-aware browsing and schema cues; proof: frontend build and browser smoke passed.
- AC-TOOL-05 [must] [interaction] Workspace `/agents` renders an internal-agent section for `main-agent` while preserving the existing system-agent section and behaviors; proof: frontend build and browser smoke passed.

## Open Decisions

- None.
