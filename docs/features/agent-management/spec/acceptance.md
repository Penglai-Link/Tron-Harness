# Acceptance: Agent Management

## Acceptance Index

- AC-AGENT-01 [must] [business] Platform settings persist one `Main Agent` singleton with `personality` and `instructions`, and later main-agent runtime preparation consumes the saved values in real requests; proof: backend tests and final review must show persisted config changes runtime input.
- AC-AGENT-02 [must] [api] Startup scan and manual `Scan` maintain a known system-agent registry for supported agents, preserve policy across rescans, and mark missing agents as `Unavailable` instead of deleting them; proof: backend tests and browser smoke passed.
- AC-AGENT-03 [must] [business] Workspace `/agents` shows only inherited system agents, excludes `Main Agent`, respects platform disable dominance, and uses current-workspace enable or disable semantics without metadata editing; proof: backend tests and browser smoke passed.
- AC-AGENT-04 [must] [interaction] `/settings/agents` and `/agents` expose the agreed controls, status labels, and feedback flows with uniform API wiring and no archive action for system agents; proof: frontend build plus browser smoke passed.

## Open Decisions

- None.
