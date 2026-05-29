= PRD: Agent Management

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron agent management.
- Problem: Tron has a runtime-facing `/agent` experience and backend agent execution surfaces, but it does not have a persisted product surface for configuring the platform `Main Agent`, discovering supported system coding agents, or controlling workspace-effective agent availability.
- Target users: platform admins who configure platform-wide AI behavior, workspace operators who manage effective agent availability inside one workspace, and engineering leads who need stable product contracts before feature delivery.
- Outcome: a platform settings page manages the singleton `Main Agent` and scanned system-agent registry, while a workspace page manages effective inherited system-agent availability without exposing local authoring complexity.

== Background
- Why now: the repo already has agent runtime, invocation, run history, AG-UI streaming, workspace inheritance, skill scanning, and settings-shell foundations, but there is still no product-level contract for agent management.
- Current pain: `Main Agent` instructions and personality are not managed as persisted product data, system coding agents such as `codex` and `opencode` are not exposed through an operator-facing catalog, and workspace owners cannot see or control effective system-agent availability.
- Business context: this feature should reuse the repo’s existing settings-shell, workspace inheritance, and registry-scanning patterns instead of inventing a parallel management system.

== Goals
- Make the platform `Main Agent` a real persisted singleton configuration.
- Expose scanned system coding agents through a platform registry surface.
- Let workspaces control effective system-agent availability with minimal local semantics.
- Reuse existing Tron response envelopes, settings navigation, workspace validation, and inheritance patterns.
- Keep v1 narrow enough that the configured `Main Agent` actually works everywhere it is used.

== Scope
- In scope: `/settings/agents`, workspace `/agents`, persisted `Main Agent` personality and instructions, startup scan of supported system agents, manual rescan, platform enable or disable of system agents, workspace effective system-agent enable or disable, runtime consumption of persisted `Main Agent` config, and bounded backend/frontend verification.
- Out of scope: per-workspace `Main Agent` authoring, archive semantics, custom system-agent authoring, agent-specific model routing, writes to external Codex or OpenCode config files, secret-reference redesign, and platform-wide code-runtime redesign.

== Feature Requirements
=== Module: Agent Management
==== Feature: Agent Management - Platform Main Agent Configuration
- name: Platform Main Agent Configuration
- description: Persist and edit one platform `Main Agent` definition that provides the global personality and instruction baseline for the main agent runtime.
- user or role: platform admin.
- entry point: `/settings/agents` `Main Agent` editor card and backend main-agent config endpoints.
- trigger: admin opens Agent settings and edits personality or instructions.
- preconditions: authenticated admin access to platform settings.
- main flow: load current `Main Agent` config -> edit `Personality` and `Instructions` -> save -> persist singleton config -> future main-agent invocations read the saved values.
- inputs and outputs: input is `personality` plus `instructions`; output is the saved singleton config in the uniform response envelope.
- permissions or visibility: visible only in platform settings; not shown in workspace agent management.
- related page headings: `Page: /settings/agents - Agent Settings`
- related business rule headings: `Rule: Agent Management - Main Agent Scope`, `Rule: Agent Management - Main Agent Runtime Consumption`
- related exception headings: `Exception: Agent Management - Main Agent Save Failure`, `Exception: Agent Management - Main Agent Missing Config`

==== Feature: Agent Management - System Agent Registry
- name: System Agent Registry
- description: Discover supported system coding agents such as `codex` and `opencode`, expose them through a platform registry table, and let the platform enable or disable them globally.
- user or role: platform admin.
- entry point: `/settings/agents` system-agent table and system-agent registry endpoints.
- trigger: app startup discovery, manual `Scan`, and platform enable or disable actions.
- preconditions: supported agent probes are available to the backend; authenticated admin access for manual actions.
- main flow: backend scans supported agents -> discovered rows load in the registry -> admin optionally rescans -> admin enables or disables a discovered system agent -> global effective state updates.
- inputs and outputs: inputs are supported runtime probes and enable or disable actions keyed by `agentCode`; outputs are registry rows with `name`, `description`, `source`, `status`, and policy-aware effective state.
- permissions or visibility: registry editing is platform-only; scanned metadata is read-only except for global enable or disable policy.
- related page headings: `Page: /settings/agents - Agent Settings`
- related business rule headings: `Rule: Agent Management - System Agent Discovery`, `Rule: Agent Management - Platform Agent Policy`, `Rule: Agent Management - Unavailable Agent Status`
- related exception headings: `Exception: Agent Management - Scan Partial Failure`, `Exception: Agent Management - Agent Unavailable During Action`

==== Feature: Agent Management - Workspace Effective System Agents
- name: Workspace Effective System Agents
- description: Show only inherited platform system agents in a workspace page and let the current workspace locally enable or disable them without editing metadata or authoring new rows.
- user or role: workspace operator.
- entry point: `/agents` workspace table and workspace system-agent endpoints.
- trigger: workspace operator opens Agent Management or toggles one inherited system agent.
- preconditions: authenticated user has access to the current workspace; the workspace is valid and active.
- main flow: load effective system-agent rows for workspace -> render inherited rows with `Inherited From = Platform` -> operator disables one row locally or re-enables it back to inherited platform state -> effective state refreshes.
- inputs and outputs: input is `workspaceCode` plus `agentCode` and enable or disable intent; output is a workspace-effective list with `status`, `source`, and `inheritedFrom`.
- permissions or visibility: visible in workspace scope; `Main Agent` is excluded entirely; metadata stays read-only.
- related page headings: `Page: /agents - Workspace Agent Management`
- related business rule headings: `Rule: Agent Management - Workspace Override Semantics`, `Rule: Agent Management - Platform Policy Dominance`
- related exception headings: `Exception: Agent Management - Missing Workspace`, `Exception: Agent Management - Archived Workspace`

==== Feature: Agent Management - Runtime Main Agent Consumption
- name: Runtime Main Agent Consumption
- description: Ensure the persisted platform `Main Agent` personality and instructions are actually used by main-agent runtime setup so the settings surface changes real behavior instead of storing dead configuration.
- user or role: platform admin and any user invoking the main agent.
- entry point: backend main-agent invocation preparation.
- trigger: a main-agent run is prepared after configuration exists.
- preconditions: main-agent runtime path is available; the platform `Main Agent` config can be loaded.
- main flow: invocation setup loads persisted `Main Agent` config -> merges it into main-agent runtime preparation -> main agent runs with the configured values everywhere.
- inputs and outputs: input is persisted `Main Agent` config plus existing runtime dependencies; output is runtime-ready main-agent configuration consumed by invocation setup.
- permissions or visibility: no separate UI page; behavior is observable through main-agent execution.
- related page headings: `Page: Backend Runtime - Main Agent Invocation Preparation`
- related business rule headings: `Rule: Agent Management - Main Agent Runtime Consumption`
- related exception headings: `Exception: Agent Management - Main Agent Missing Config`

== Page Prototypes And Interactions
=== Page: /settings/agents - Agent Settings
- entry: Settings shell navigation item `Agents`.
- layout: existing `SettingsShell` with one page composed of a top `Main Agent` editor card and a lower system-agent table with a `Scan` action.
- data display: current `Main Agent` personality and instructions; scanned system-agent rows with `Name`, `Description`, `Source`, `Status`, and `Actions`.
- actions: save main-agent edits, trigger manual scan, enable system agent, disable system agent.
- states: loading, empty registry, enabled, disabled, unavailable, saving, scanning, and action-level busy states.

==== Interaction: /settings/agents - Save Main Agent
- trigger control: `Save` button in the `Main Agent` editor card.
- preconditions: admin has modified personality or instructions.
- button or control behavior: submit the singleton config update and keep the editor scoped to human-facing fields only.
- form rules: `personality` and `instructions` are editable text fields; no redundant type or source fields are shown.
- confirmation behavior: no destructive confirmation; save is immediate.
- success feedback: toast or inline success confirms the `Main Agent` configuration was saved.
- failure feedback: error feedback states that the `Main Agent` configuration could not be saved and preserves draft edits.
- navigation or refresh behavior: remain on `/settings/agents`; refresh the saved config state in place.

==== Interaction: /settings/agents - Scan System Agents
- trigger control: `Scan` button above the system-agent table.
- preconditions: authenticated platform admin.
- button or control behavior: trigger backend rescan and temporarily mark the page as scanning.
- form rules: none.
- confirmation behavior: none.
- success feedback: visible refresh of registry rows and a success notice summarizing scan completion.
- failure feedback: scan error or partial-scan notice without dropping existing visible policy state.
- navigation or refresh behavior: remain on `/settings/agents` and refresh the registry table in place.

==== Interaction: /settings/agents - Toggle Platform System Agent
- trigger control: `Enable` or `Disable` action in one system-agent row.
- preconditions: row is currently discovered; unavailable rows are read-only.
- button or control behavior: submit a platform policy action for the selected `agentCode`.
- form rules: none.
- confirmation behavior: no extra modal in v1.
- success feedback: the row status updates and a success notice describes the new platform state.
- failure feedback: action-level error feedback explains that the selected system agent could not be updated.
- navigation or refresh behavior: remain on `/settings/agents` and refresh the affected row or table slice.

=== Page: /agents - Workspace Agent Management
- entry: workspace console route `/agents`.
- layout: console page with a table-oriented workspace capability view.
- data display: inherited system-agent rows only with `Name`, `Description`, `Source`, `Inherited From`, `Status`, and `Actions`.
- actions: enable inherited system agent back to platform behavior or disable it locally for the current workspace.
- states: loading, empty inherited set, enabled, disabled, unavailable, and action-level busy states.

==== Interaction: /agents - Toggle Workspace System Agent
- trigger control: `Enable` or `Disable` action for one effective system-agent row.
- preconditions: workspace is valid and active; `Main Agent` is not present on this page.
- button or control behavior: submit a workspace-local policy action keyed by `workspaceCode` and `agentCode`.
- form rules: metadata fields are read-only; no inline editing is available.
- confirmation behavior: none in v1.
- success feedback: row status updates and feedback confirms the workspace-effective change.
- failure feedback: error feedback preserves the current visible row and explains that the workspace action failed.
- navigation or refresh behavior: remain on `/agents` and refresh the effective table state in place.

=== Page: Backend Runtime - Main Agent Invocation Preparation
- entry: internal backend invocation path for the main agent.
- layout: no user-facing page; this is a product entry point for runtime behavior.
- data display: not applicable.
- actions: load persisted main-agent config during invocation preparation.
- states: valid config loaded, missing config fallback, and load failure.

== Business Rules
=== Rule: Agent Management - Main Agent Scope
- condition: the product needs one internal agent in v1.
- decision or calculation: `Main Agent` is a platform-level singleton and is not represented as a workspace-managed inherited row.
- state transition: save updates the singleton config only; no workspace row is created.
- permission or data boundary: only platform settings may edit `Main Agent`.
- conflict or priority rule: workspace pages must never expose local editing or lifecycle actions for `Main Agent`.
- example: `/settings/agents` shows a `Main Agent` editor card, while `/agents` shows only system-agent rows.

=== Rule: Agent Management - System Agent Discovery
- condition: the backend evaluates supported system coding agents at startup or during manual scan.
- decision or calculation: discovery probes only the supported set and updates registry facts without treating system agents as manually authored rows.
- state transition: supported discovered agents become visible registry rows; previously known but no longer found agents move to `Unavailable`.
- permission or data boundary: discovery facts come from backend probing, not from direct user editing.
- conflict or priority rule: scan updates availability facts but must preserve persisted policy for still-known agents.
- example: `codex` is found during startup and appears as discovered; a later scan cannot find `opencode`, so its row remains known but becomes `Unavailable`.

=== Rule: Agent Management - Platform Agent Policy
- condition: a platform admin enables or disables one discovered system agent.
- decision or calculation: platform policy defines the global effective baseline for that system agent.
- state transition: platform enable moves the row to effective enabled state; platform disable moves it to effective disabled state everywhere.
- permission or data boundary: only platform settings can change global system-agent policy.
- conflict or priority rule: platform disable dominates all workspace-local state.
- example: if the platform disables `codex`, no workspace can make `codex` effectively enabled until the platform enables it again.

=== Rule: Agent Management - Workspace Override Semantics
- condition: a workspace operator changes one effective inherited system-agent row.
- decision or calculation: no local row means inherit platform state; local disable creates a current-workspace override; local enable removes that override and returns to inherited behavior.
- state transition: enabled inherited row -> local disabled override; local disabled override -> inherited platform state.
- permission or data boundary: workspace actions affect only the current workspace-effective state and never edit discovered metadata.
- conflict or priority rule: workspace enable does not resurrect a platform-disabled agent into effective enabled state.
- example: the platform keeps `opencode` enabled, one workspace disables it locally, and another workspace still sees `opencode` enabled.

=== Rule: Agent Management - Unavailable Agent Status
- condition: a previously known system agent is not found by the latest scan.
- decision or calculation: the agent remains part of Tron’s known registry and policy set but changes to `Unavailable`.
- state transition: discovered enabled or disabled row -> unavailable row until a later successful scan finds it again.
- permission or data boundary: unavailable rows are read-only except for scan operations that may restore discovery.
- conflict or priority rule: unavailable state replaces archive semantics in v1 and must not silently delete platform or workspace policy.
- example: `codex` was known yesterday, disappears from the machine today, and the row remains visible as `Unavailable` after scan.

=== Rule: Agent Management - Main Agent Runtime Consumption
- condition: the main agent is invoked after the platform has a persisted `Main Agent` config.
- decision or calculation: runtime preparation must load the persisted personality and instructions and feed them into main-agent setup.
- state transition: updated platform config becomes the current effective main-agent configuration for later invocations.
- permission or data boundary: this rule changes runtime behavior only for the main agent and does not imply system-agent-specific runtime rewiring in v1.
- conflict or priority rule: the settings surface cannot be considered complete if saved `Main Agent` config is not actually consumed by runtime.
- example: an admin updates `Main Agent` instructions in settings, then a later main-agent run uses those saved instructions.

== Exceptions And Boundaries
=== Exception: Agent Management - Main Agent Save Failure
- scenario: platform admin submits updated `Main Agent` fields and persistence fails.
- expected product behavior: the save is rejected and the current draft remains visible for correction or retry.
- user-visible message or fallback: show that the `Main Agent` configuration could not be saved.
- data consistency expectation: no partial singleton update is exposed as success.
- retry, recovery, or compensation behavior: user can retry save without leaving the page.
- logging/audit expectation: log the failed update attempt with sufficient context for diagnosis.

=== Exception: Agent Management - Main Agent Missing Config
- scenario: runtime or settings requests the `Main Agent` config before a persisted singleton row exists or when loading fails.
- expected product behavior: settings shows a recoverable editable state and runtime uses the defined bootstrap fallback or fails in a controlled way instead of silently using stale configuration.
- user-visible message or fallback: settings can present an initial empty or default editor state; runtime-facing failure should be stable and diagnosable.
- data consistency expectation: the system does not invent a hidden workspace-local `Main Agent` substitute.
- retry, recovery, or compensation behavior: first successful save creates the singleton config; runtime can retry after config becomes available.
- logging/audit expectation: record missing-config events for backend diagnosis.

=== Exception: Agent Management - Scan Partial Failure
- scenario: one or more supported agent probes fail during startup scan or manual scan.
- expected product behavior: the app remains available, returns the latest known registry state, and surfaces partial-scan results clearly.
- user-visible message or fallback: show that scan completed with partial failure or that some agents could not be verified.
- data consistency expectation: existing visible policy rows are preserved; successful discoveries still update.
- retry, recovery, or compensation behavior: admin can rerun `Scan` without cleanup steps.
- logging/audit expectation: scan failure details are logged for backend diagnosis.

=== Exception: Agent Management - Agent Unavailable During Action
- scenario: user tries to act on a row that became unavailable between load time and action time.
- expected product behavior: reject the action and refresh the row to `Unavailable`.
- user-visible message or fallback: explain that the selected system agent is no longer available on the current machine.
- data consistency expectation: policy is not silently deleted; only the effective status changes.
- retry, recovery, or compensation behavior: rerun `Scan` and retry once the agent reappears.
- logging/audit expectation: record the rejected action against unavailable runtime state.

=== Exception: Agent Management - Missing Workspace
- scenario: workspace agent-management endpoints receive an unknown workspace code.
- expected product behavior: reject the request using the repo’s standard error envelope.
- user-visible message or fallback: show that the workspace was not found or is no longer available.
- data consistency expectation: no workspace-local policy row is written.
- retry, recovery, or compensation behavior: user must select a valid workspace and retry.
- logging/audit expectation: log the rejected request as an invalid workspace access attempt.

=== Exception: Agent Management - Archived Workspace
- scenario: workspace agent-management action targets an archived workspace.
- expected product behavior: reject effective-state mutation and avoid exposing mutable workspace controls.
- user-visible message or fallback: show that archived workspaces cannot update local agent availability.
- data consistency expectation: no workspace-local policy is created or modified.
- retry, recovery, or compensation behavior: restore or switch to an active workspace before retrying.
- logging/audit expectation: log the rejected archived-workspace mutation attempt.

== Shared Requirements
- Settings shell route contract: shared because platform settings pages already depend on `appRoutes`, `routePages`, `SettingsShell`, and `settingsNavigation`; consumers: Feature 1, Feature 2; decision: build first and reuse existing settings patterns.
- Workspace hierarchy resolution: shared because effective workspace state already depends on validated active workspaces and inheritance-aware path resolution; consumers: Feature 3, Feature 4; decision: build first by reusing existing workspace logic.
- Registry scanning and policy layering: shared because system-agent discovery needs the same separation of discovered catalog metadata and workspace policy that skill registry already models; consumers: Feature 2, Feature 3; decision: build first and keep local to agent management.
- Uniform API response envelope: shared because all agent-management endpoints must align with the repo’s `ApiResponse` shape; consumers: Feature 1, Feature 2, Feature 3, Feature 4; decision: build first and keep consistent with existing admin and agent APIs.
- Runtime main-agent configuration load: shared because the admin settings page is only valid if the main-agent invocation path actually consumes the saved config; consumers: Feature 1, Feature 4; decision: build first before calling the feature complete.

== Feature Inventory
1. Platform Main Agent Configuration: module Agent Management; source headings `Feature: Agent Management - Platform Main Agent Configuration`, `Page: /settings/agents - Agent Settings`, `Rule: Agent Management - Main Agent Scope`, `Rule: Agent Management - Main Agent Runtime Consumption`, `Exception: Agent Management - Main Agent Save Failure`, `Exception: Agent Management - Main Agent Missing Config`; goal persist and edit the singleton `Main Agent` so the platform can manage global main-agent behavior; in scope settings editor and persisted config; out of scope workspace-local main-agent authoring; depends on shared requirements: settings shell route contract, uniform API response envelope, runtime main-agent configuration load.
2. System Agent Registry: module Agent Management; source headings `Feature: Agent Management - System Agent Registry`, `Page: /settings/agents - Agent Settings`, `Rule: Agent Management - System Agent Discovery`, `Rule: Agent Management - Platform Agent Policy`, `Rule: Agent Management - Unavailable Agent Status`, `Exception: Agent Management - Scan Partial Failure`, `Exception: Agent Management - Agent Unavailable During Action`; goal expose supported system coding agents through a scanned platform registry with global enable or disable control; in scope startup scan, manual rescan, status handling, and platform policy; out of scope custom system-agent authoring and external tool-config writes; depends on shared requirements: settings shell route contract, registry scanning and policy layering, uniform API response envelope.
3. Workspace Effective System Agents: module Agent Management; source headings `Feature: Agent Management - Workspace Effective System Agents`, `Page: /agents - Workspace Agent Management`, `Rule: Agent Management - Workspace Override Semantics`, `Rule: Agent Management - Platform Policy Dominance`, `Exception: Agent Management - Missing Workspace`, `Exception: Agent Management - Archived Workspace`; goal let each workspace view and control effective inherited system-agent availability with minimal local semantics; in scope inherited rows, local enable or disable actions, and `Inherited From = Platform`; out of scope metadata editing and `Main Agent` visibility; depends on Feature 2 and shared requirements: workspace hierarchy resolution, registry scanning and policy layering, uniform API response envelope.
4. Runtime Main Agent Consumption: module Agent Management; source headings `Feature: Agent Management - Runtime Main Agent Consumption`, `Page: Backend Runtime - Main Agent Invocation Preparation`, `Rule: Agent Management - Main Agent Runtime Consumption`, `Exception: Agent Management - Main Agent Missing Config`; goal make persisted `Main Agent` configuration actually drive main-agent runtime preparation; in scope loading personality and instructions into invocation setup; out of scope system-agent-specific runtime routing or per-agent model policy; depends on Feature 1 and shared requirements: runtime main-agent configuration load, uniform API response envelope.

== Dependency Map
#mermaid(
  "flowchart LR
    SR1[\"Shared Requirement: Settings Shell Route Contract\"] --> F1[\"Feature 1: Platform Main Agent Configuration\"]
    SR4[\"Shared Requirement: Uniform API Response Envelope\"] --> F1
    SR5[\"Shared Requirement: Runtime Main-Agent Configuration Load\"] --> F1
    SR1 --> F2[\"Feature 2: System Agent Registry\"]
    SR3[\"Shared Requirement: Registry Scanning And Policy Layering\"] --> F2
    SR4 --> F2
    F2 --> F3[\"Feature 3: Workspace Effective System Agents\"]
    SR2[\"Shared Requirement: Workspace Hierarchy Resolution\"] --> F3
    SR3 --> F3
    SR4 --> F3
    F1 --> F4[\"Feature 4: Runtime Main-Agent Consumption\"]
    SR5 --> F4
    SR4 --> F4"
)

== Recommended Implementation Order
1. Platform Main Agent Configuration - the singleton config and settings-shell route establish the first stable source of truth.
2. System Agent Registry - scan and platform policy must exist before any workspace-effective view can inherit from them.
3. Workspace Effective System Agents - once registry and platform policy are stable, the workspace-effective page can stay narrow and deterministic.
4. Runtime Main Agent Consumption - integrate saved `Main Agent` config into invocation setup before declaring the feature complete.

== Acceptance
=== Acceptance: Agent Management - Platform Main Agent Configuration
- Platform settings expose `/settings/agents`, render a focused `Main Agent` editor card, persist `personality` and `instructions`, and do not show redundant internal-agent metadata or workspace-local authoring controls.

=== Acceptance: Agent Management - System Agent Registry
- Startup scan and manual `Scan` expose supported system agents through a platform registry table with stable `Enabled`, `Disabled`, or `Unavailable` state, and platform enable or disable actions update global effective policy without deleting known rows on scan churn.

=== Acceptance: Agent Management - Workspace Effective System Agents
- Workspace `/agents` shows inherited system agents only, excludes `Main Agent`, renders `Inherited From = Platform`, and supports current-workspace enable or disable semantics that never bypass platform policy dominance.

=== Acceptance: Agent Management - Runtime Main Agent Consumption
- A saved platform `Main Agent` configuration is consumed by main-agent runtime preparation so the settings surface changes real main-agent behavior rather than storing unused configuration.

== Risks And Open Questions
- The repo already has multiple inheritance patterns across models, skills, and variables; agent management should reuse the closest fit deliberately instead of mixing semantics from all three.
- Startup scan is new for this product area; the app must avoid hard failure when one supported probe fails.
- Current runtime invocation paths do not yet persist or load a main-agent config, so feature completion requires a real integration point instead of stopping at the settings API.
- No open product questions remain for the approved v1 scope.
