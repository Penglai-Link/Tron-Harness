# Acceptance: Tool Registry

## Acceptance Index

- AC-TOOL-01 [must] [api] Platform tool catalog APIs expose stable code-defined business tool domains and concrete tool schemas; proof: backend tests and final review show catalog responses match registered definitions.
- AC-TOOL-02 [must] [business] Workspace internal-agent policy resolves `main-agent` defaults, overrides, and reset behavior without regressing system-agent APIs; proof: backend tests and final review show effective policy resolution and preserved system-agent behavior.
- AC-TOOL-03 [must] [api] Main-agent invocation derives effective assigned tools from internal-agent policy and materializes runtime tool exposure without requiring explicit request tools in normal direct-list runs; proof: backend tests and final review show runtime preparation behavior.
- AC-TOOL-04 [must] [interaction] `/settings/tools` renders the read-only platform business tool catalog with domain-aware browsing and schema cues; proof: frontend build and browser smoke passed.
- AC-TOOL-05 [must] [interaction] Workspace `/agents` renders an internal-agent section for `main-agent` while preserving the existing system-agent section and behaviors; proof: frontend build and browser smoke passed.

## Open Decisions

- None.
