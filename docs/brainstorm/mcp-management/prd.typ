= PRD: MCP Management

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: MCP Management extends Tron with platform-managed MCP gateway, registry, workspace-effective server policy, and internal-agent MCP access control.
- Problem: Tron has static platform tools and internal-agent runtime policy, but no productized way to expose one inbound MCP endpoint or consume workspace-effective remote MCP servers as runtime tools.
- Target users: platform operators, workspace operators, and workspace internal-agent maintainers.
- Outcome: operators can manage platform MCP servers, workspaces can locally enable or disable inherited servers, internal agents can consume effective MCP tools by server assignment, and runtime can materialize remote MCP tools safely for each run.

== Background
- Why now: Tron already has settings-shell patterns, workspace inheritance foundations, internal-agent policy, and runtime tool-calling infrastructure, so MCP management can now be added as one bounded cross-surface feature.
- Current pain: there is no platform-owned MCP registry, no workspace-effective MCP server view, no internal-agent-by-server MCP assignment, and no runtime federation path for remote MCP tools.
- Business context: the product needs one operator-facing way to connect bounded external MCP capabilities without mixing them into browser CRUD APIs, workspace-local secrets, or static platform tool definitions.

== Goals
- Add one platform-managed inbound MCP gateway switch for the variables MCP endpoint.
- Add one platform-owned registry for remote MCP server definitions.
- Add one workspace-effective inherited MCP server surface with enable or disable behavior.
- Extend internal-agent policy so MCP access can be disabled, inherited from all enabled servers, or limited to selected servers.
- Materialize effective MCP tools into the current run as callable `mcp.<server_code>.<tool_name>` tools.
- Keep delete and disable semantics explicit and operator-safe.

== Scope
- In scope:
  - platform MCP gateway singleton config
  - platform-owned remote MCP server registry
  - workspace inherited MCP server enable or disable behavior
  - internal-agent MCP access mode and selected-server assignment
  - runtime discovery and materialization of effective MCP tools
  - authenticated management APIs and MCP transport endpoint
- Out of scope:
  - workspace-local MCP server authoring
  - workspace override of parent connection definition
  - stdio transport
  - permission-matrix enforcement beyond authenticated APIs
  - per-server timeout or pool tuning
  - secret-vault integration
  - per-tool MCP assignment
  - inbound MCP endpoints other than `/mcp/variables`

== Feature Requirements
=== Module: MCP Management
==== Feature: MCP Management - Platform MCP Gateway
- name: Platform MCP Gateway
- description: Control whether the platform exposes the variables MCP endpoint.
- user or role: platform operator
- entry point: `/settings/mcp-servers`
- trigger: operator opens settings and toggles gateway or variables endpoint state
- preconditions: authenticated access to platform settings
- main flow: load current gateway config -> change gateway state -> save singleton config -> future transport requests respect current enabled state
- inputs and outputs: input is gateway enable state plus variables endpoint enable state; output is the saved singleton config in the uniform response envelope
- permissions or visibility: visible only in platform settings; not shown on workspace pages
- related page headings: `Page: /settings/mcp-servers - Platform MCP Settings`
- related business rule headings: `Rule: MCP Management - Platform Gateway Ownership`
- related exception headings: `Exception: MCP Management - Gateway Disabled`

==== Feature: MCP Management - Platform MCP Server Registry
- name: Platform MCP Server Registry
- description: Persist and maintain platform-owned remote MCP server definitions.
- user or role: platform operator
- entry point: `/settings/mcp-servers`
- trigger: operator creates, edits, disables, or deletes a platform MCP server
- preconditions: authenticated access to platform settings
- main flow: load registry -> create or edit server definition -> validate stable server identity and transport fields -> save definition -> future workspace and runtime projections refresh from the platform source
- inputs and outputs: input is server code, metadata, URL, auth shape, headers, and enabled state; output is saved platform server records in the uniform response envelope
- permissions or visibility: visible only in platform settings; workspaces consume inherited effective projections only
- related page headings: `Page: /settings/mcp-servers - Platform MCP Settings`
- related business rule headings: `Rule: MCP Management - Platform Server Ownership`, `Rule: MCP Management - Disable Versus Delete`
- related exception headings: `Exception: MCP Management - Invalid Server Definition`, `Exception: MCP Management - Deleted Selected Server`

==== Feature: MCP Management - Workspace Effective MCP Servers
- name: Workspace Effective MCP Servers
- description: Show inherited platform MCP servers in the current workspace and allow local enable or disable behavior only.
- user or role: workspace operator
- entry point: `/mcp-servers`
- trigger: operator opens the workspace MCP page or changes one inherited server's effective state
- preconditions: authenticated access with an active workspace
- main flow: load effective inherited server list -> show source ownership and current effective state -> disable one inherited enabled server locally or remove the local disable override to restore inherited behavior
- inputs and outputs: input is the current workspace code plus target server code and desired local effective action; output is refreshed effective server state in the uniform response envelope
- permissions or visibility: visible in workspace shell only; platform-owned connection fields are read-only in this surface
- related page headings: `Page: /mcp-servers - Workspace MCP Servers`
- related business rule headings: `Rule: MCP Management - Workspace Enable Disable Override`
- related exception headings: `Exception: MCP Management - Missing Workspace`, `Exception: MCP Management - Platform Disabled Server`

==== Feature: MCP Management - Internal Agent MCP Server Access
- name: Internal Agent MCP Server Access
- description: Extend internal-agent policy so each internal agent can disable MCP, inherit all enabled workspace MCP servers, or use a selected-server list.
- user or role: workspace operator
- entry point: `/agents`
- trigger: operator changes MCP access mode or selected servers for an internal agent
- preconditions: authenticated access with an active workspace and an internal-agent row
- main flow: load workspace internal-agent policy -> choose MCP access mode -> if selected mode is active choose workspace-effective MCP servers -> save policy and assignments -> future runs use the updated MCP server set
- inputs and outputs: input is agent code, MCP access mode, and optional selected server codes; output is the refreshed internal-agent policy in the uniform response envelope
- permissions or visibility: visible in workspace `/agents` internal-agent controls only; not part of platform system-agent registry
- related page headings: `Page: /agents - Workspace Agent Management`
- related business rule headings: `Rule: MCP Management - Agent Assignment By Server`
- related exception headings: `Exception: MCP Management - Deleted Selected Server`, `Exception: MCP Management - Platform Disabled Server`

==== Feature: MCP Management - Runtime MCP Tool Federation
- name: Runtime MCP Tool Federation
- description: Discover tools from the effective MCP servers for the current run and materialize them as callable runtime tools.
- user or role: internal agent runtime
- entry point: internal agent invocation preparation
- trigger: an internal-agent run starts with MCP access enabled
- preconditions: workspace is valid, internal-agent MCP policy is resolved, and at least one effective server remains eligible
- main flow: resolve workspace-effective servers -> apply internal-agent MCP access policy -> discover tools from each selected effective server -> materialize runtime tool schemas under `mcp.<server_code>.<tool_name>` -> append them to the run context -> execute later calls through the MCP federation layer
- inputs and outputs: input is workspace code, agent code, and run context; output is a runtime-effective MCP tool set for the current run only
- permissions or visibility: no separate operator page; behavior is reflected through runtime execution only
- related page headings: none
- related business rule headings: `Rule: MCP Management - Runtime Tool Materialization`, `Rule: MCP Management - Future Runs Observe Configuration Changes`
- related exception headings: `Exception: MCP Management - Runtime Discovery Failure`

== Page Prototypes And Interactions
=== Page: /settings/mcp-servers - Platform MCP Settings
- entry: settings-shell navigation item for MCP Servers
- layout: top gateway settings card plus lower server registry table and create or edit flow
- data display: gateway state, endpoint state, platform MCP servers, connection metadata, enabled state, and latest update summary
- actions: save gateway config, create server, edit server, disable server, enable server, delete server
- states: loading, empty registry, dirty form, save success, validation error, delete confirmation

==== Interaction: /settings/mcp-servers - Save Gateway Config
- trigger control: gateway form save button
- preconditions: operator changed gateway or endpoint state
- button or control behavior: submit singleton config only; keep server registry table stable
- form rules: at least one endpoint toggle may be false; no endpoint can be effectively enabled if gateway is disabled
- confirmation behavior: none beyond normal save intent
- success feedback: show saved confirmation and refreshed gateway state
- failure feedback: show inline save failure and keep draft values visible
- navigation or refresh behavior: remain on `/settings/mcp-servers`

==== Interaction: /settings/mcp-servers - Save Platform MCP Server
- trigger control: create or edit form save button
- preconditions: required server fields are valid
- button or control behavior: submit server definition and refresh the platform registry table
- form rules: server code must be stable and unique; URL must be present; auth and headers must serialize into supported config shape
- confirmation behavior: delete requires explicit confirmation; save does not
- success feedback: show created or updated confirmation and refreshed table row
- failure feedback: show field-level or form-level validation error and keep entered values
- navigation or refresh behavior: remain on `/settings/mcp-servers` and refresh the affected row or table

=== Page: /mcp-servers - Workspace MCP Servers
- entry: workspace shell navigation item for MCP Servers
- layout: one table of effective inherited MCP servers for the active workspace
- data display: server identity, source ownership, platform enabled state, current workspace effective state, and local override summary
- actions: disable inherited server locally, restore inherited enabled behavior
- states: loading, empty effective list, no active workspace, action success, action failure

==== Interaction: /mcp-servers - Toggle Workspace Effective Server
- trigger control: row action button
- preconditions: active workspace exists and the target server is platform-enabled
- button or control behavior: disable writes a local override; enable removes the local override
- form rules: workspace cannot edit URL, auth, or headers; workspace cannot make a platform-disabled server effectively enabled
- confirmation behavior: none
- success feedback: show row-level or toast confirmation and updated effective state
- failure feedback: show action failure without losing the current table
- navigation or refresh behavior: remain on `/mcp-servers` and refresh the affected row

=== Page: /agents - Workspace Agent Management
- entry: workspace shell navigation item for Agents
- layout: existing internal-agent section plus system-agent section
- data display: current internal-agent policy plus MCP access mode and selected-server summary
- actions: change MCP access mode, save selected servers when selected mode is active
- states: loading, no active workspace, dirty policy, save success, stale selected server error

==== Interaction: /agents - Save Internal Agent MCP Access
- trigger control: internal-agent MCP policy save button
- preconditions: active workspace exists and the target internal agent is editable
- button or control behavior: submit MCP access mode and optional selected server list together with the existing internal-agent policy surface
- form rules: selected mode may only submit servers currently effective in the workspace; deleted servers must be rejected as stale
- confirmation behavior: none
- success feedback: show saved confirmation and refreshed policy summary
- failure feedback: show stale-assignment or save failure message and keep current selections visible
- navigation or refresh behavior: remain on `/agents`

== Business Rules
=== Rule: MCP Management - Platform Gateway Ownership
- condition: gateway state for `/mcp/variables` changes
- decision or calculation: only platform settings update the gateway singleton config
- state transition: future transport requests observe the latest saved gateway state
- permission or data boundary: workspace pages do not edit gateway state
- conflict or priority rule: if gateway is disabled, endpoint-specific enabled flags do not make the endpoint effectively reachable
- example: gateway disabled plus variables endpoint enabled still yields an unavailable transport endpoint

=== Rule: MCP Management - Platform Server Ownership
- condition: an MCP server definition is created or updated
- decision or calculation: platform persists the canonical connection definition
- state transition: future workspace projections and runtime discovery read from the saved platform definition
- permission or data boundary: workspaces cannot rewrite the upstream URL, auth, or headers in v1
- conflict or priority rule: platform source always wins over workspace-local edits because local definition edits do not exist in v1
- example: a workspace sees `server-a` inherited from platform and may only change effective enabled state

=== Rule: MCP Management - Workspace Enable Disable Override
- condition: a workspace toggles one inherited MCP server
- decision or calculation: workspace stores only local effective-state override behavior, not a copied connection definition
- state transition: disabling writes a local override; enabling removes the local disable override and returns to inherited behavior
- permission or data boundary: workspace cannot make a platform-disabled server effectively enabled
- conflict or priority rule: platform disable dominates all workspace-local state
- example: platform disables `server-a`; workspace row becomes disabled even if it previously inherited enabled behavior

=== Rule: MCP Management - Agent Assignment By Server
- condition: an internal agent uses MCP access mode `selected_servers`
- decision or calculation: persist one row per `(workspace_code, agent_code, server_code)`
- state transition: future runs resolve selected servers from the current saved assignment set
- permission or data boundary: assignments may only reference servers currently effective in that workspace
- conflict or priority rule: deleted or no-longer-effective servers are not materialized into runtime tools
- example: `main-agent` in workspace `DATA` selects `server-a` and `server-b`; later platform disables `server-b`, so only `server-a` remains eligible for new runs

=== Rule: MCP Management - Runtime Tool Materialization
- condition: an internal-agent run starts with MCP access enabled
- decision or calculation: discover tools from each selected effective server and materialize them as `mcp.<server_code>.<tool_name>`
- state transition: the current run receives a tool snapshot; future config changes affect only later runs
- permission or data boundary: runtime tool discovery happens after workspace-effective server resolution and agent policy filtering
- conflict or priority rule: static platform tools and dynamic MCP tools share the run tool list but do not share the same startup registration source
- example: current run starts with `server-a`; platform later deletes `server-a`; current run keeps its snapshot while the next run does not receive those tools

=== Rule: MCP Management - Disable Versus Delete
- condition: operator disables or deletes a platform MCP server
- decision or calculation: disable preserves definitions and references; delete removes the definition and cascades dependent references
- state transition: disable removes the server from future effective visibility and future runtime discovery while preserving overrides and assignments; delete removes overrides, selected-server assignments, and cached pool state
- permission or data boundary: delete is platform-only
- conflict or priority rule: delete wins over all inherited state because the upstream record no longer exists
- example: deleting `server-a` clears workspace local overrides and `main-agent` selected-server assignments that pointed to `server-a`

=== Rule: MCP Management - Future Runs Observe Configuration Changes
- condition: gateway state, platform server state, workspace-effective state, or agent selected-server assignment changes while runs already exist
- decision or calculation: only future runs use the new resolved state
- state transition: current runs keep the snapshot they started with
- permission or data boundary: runtime snapshot isolation follows existing run-context principles
- conflict or priority rule: active runs are not mutated mid-flight
- example: an operator disables one server during an active run; that run keeps its snapshot and the next run does not receive tools from the disabled server

== Exceptions And Boundaries
=== Exception: MCP Management - Gateway Disabled
- scenario: a client calls `/mcp/variables` while the gateway or variables endpoint is disabled
- expected product behavior: the transport surface rejects the request and indicates the endpoint is unavailable
- user-visible message or fallback: management UI shows the endpoint as disabled; transport clients receive a stable disabled response shape
- data consistency expectation: no server registry or workspace policy rows change
- retry, recovery, or compensation behavior: retry only after the platform operator enables the gateway
- logging/audit expectation: record the disabled transport access attempt in normal server logs

=== Exception: MCP Management - Invalid Server Definition
- scenario: platform save payload omits required connection fields or uses unsupported auth shape
- expected product behavior: save is rejected without mutating the current registry
- user-visible message or fallback: show field-level or form-level validation error
- data consistency expectation: previously saved platform definitions remain unchanged
- retry, recovery, or compensation behavior: operator corrects the payload and retries save
- logging/audit expectation: validation failure may be logged for diagnostics

=== Exception: MCP Management - Platform Disabled Server
- scenario: workspace or internal-agent flow references a server that is platform-disabled
- expected product behavior: workspace cannot make it effectively enabled and internal-agent assignment cannot materialize it for new runs
- user-visible message or fallback: show that the server is disabled at platform scope
- data consistency expectation: existing local overrides or selected-server rows may remain stored, but they do not make the server effective
- retry, recovery, or compensation behavior: operator must re-enable the server at platform scope first
- logging/audit expectation: normal mutation logs should show the rejected effective-enable attempt when applicable

=== Exception: MCP Management - Deleted Selected Server
- scenario: an internal-agent selected-server submission or persisted selection references a platform server that has since been deleted
- expected product behavior: management save rejects stale submitted codes; platform delete path cascades persisted assignments so stale rows do not linger
- user-visible message or fallback: show a stale-assignment message and refresh available selections
- data consistency expectation: deleted server references are removed from persisted assignments by the delete cascade
- retry, recovery, or compensation behavior: operator refreshes the page and selects currently valid servers
- logging/audit expectation: delete flow logs the cascade and stale submissions log a typed validation failure

=== Exception: MCP Management - Runtime Discovery Failure
- scenario: a selected effective MCP server cannot be discovered or invoked during runtime preparation
- expected product behavior: runtime handles the server failure without corrupting platform registry state
- user-visible message or fallback: the affected run receives stable runtime failure or degraded-tool feedback according to runtime error policy
- data consistency expectation: management records remain unchanged; the failure affects the run, not the saved registry
- retry, recovery, or compensation behavior: later runs may retry once the remote MCP server recovers
- logging/audit expectation: runtime logs the server-specific discovery or invocation failure with the server code

=== Exception: MCP Management - Missing Workspace
- scenario: workspace MCP page or internal-agent MCP policy is requested without an active valid workspace
- expected product behavior: the request is rejected or the page shows the existing no-active-workspace state
- user-visible message or fallback: show the standard active-workspace-required message
- data consistency expectation: no workspace override or agent assignment rows are written
- retry, recovery, or compensation behavior: operator selects a valid active workspace and retries
- logging/audit expectation: standard request validation logs are sufficient

== Shared Requirements
- Settings shell route contract: shared because the platform MCP page must reuse the existing settings-shell information architecture; consumers: MCP Management and future platform admin surfaces; decision: keep local to current settings-shell pattern
- Workspace context and active-workspace validation: shared because workspace MCP server state and internal-agent MCP policy both resolve from the active workspace; consumers: MCP Management, workspace-management, agent-management; decision: reuse existing foundation
- Internal-agent policy foundation: shared because MCP access mode extends the same internal-agent policy surface already used for static platform tools; consumers: MCP Management and agent-management; decision: reuse and evolve the existing agent-management foundation
- Shared HTTP client profiles: shared because runtime MCP connections should align with the existing app-level HTTP client model rather than feature-local transport clients; consumers: MCP Management and other runtime HTTP callers; decision: reuse existing foundation
- Auth-only management API boundary: shared because v1 intentionally relies on authenticated access without introducing a new permission matrix; consumers: MCP Management and other MVP management surfaces; decision: keep current product boundary

== Feature Inventory
1. MCP Management: module MCP Management; source headings all `Feature: MCP Management - ...` entries in this PRD plus related page, interaction, rule, and exception headings; pages `/settings/mcp-servers`, `/mcp-servers`, and `/agents`; goal add platform MCP gateway and registry, workspace-effective MCP servers, internal-agent MCP access, and runtime MCP tool federation as one bounded first version; in scope platform-owned server registry, workspace enable or disable only, agent-by-server assignment, and dynamic runtime federation; out of scope workspace-local authoring, full override inheritance, row-level transport tuning, and per-tool assignment; depends on existing settings shell, workspace context, agent-management internal-agent policy, runtime invocation path, and shared HTTP client profiles.
2. MCP Local Authoring And Override Evolution: module MCP Management; source heading future workspace-local authoring; pages future workspace MCP create or edit flows; rules future parent override semantics and secret handling; exceptions conflicting overrides and secret ownership; goal let workspaces create their own servers or override inherited connection definition safely; in scope future local authoring and override rules; out of scope first version; depends on Feature 1.
3. MCP Fine-Grained Access And Secret Governance: module MCP Management; source heading future policy evolution; pages future policy and secret admin surfaces; rules per-tool assignment, secret vault, and permission matrix; exceptions revoked secret and denied tool access; goal harden MCP access control and secret governance after the base management surface is stable; in scope future permission and secret work; out of scope first version; depends on Feature 1.

== Dependency Map
#mermaid(
  "flowchart LR
    SR1[\"Shared Requirement: Settings Shell Route Contract\"] --> F1[\"Feature: MCP Management\"]
    SR2[\"Shared Requirement: Workspace Context And Active Workspace Validation\"] --> F1
    SR3[\"Shared Requirement: Internal Agent Policy Foundation\"] --> F1
    SR4[\"Shared Requirement: Shared HTTP Client Profiles\"] --> F1
    F1 --> F2[\"Feature: MCP Local Authoring And Override Evolution\"]
    F1 --> F3[\"Feature: MCP Fine-Grained Access And Secret Governance\"]
  "
)

== Recommended Implementation Order
1. MCP Management base feature - platform gateway, platform registry, workspace-effective enable or disable behavior, internal-agent MCP access mode, and runtime tool federation must land together because each surface depends on the same saved server identity and runtime resolution contract.
2. MCP Local Authoring And Override Evolution - only after the platform-owned first version proves the inheritance and runtime invalidation boundaries.
3. MCP Fine-Grained Access And Secret Governance - only after the base feature establishes stable server identity, lifecycle, and runtime federation semantics.

== Acceptance
=== Acceptance: MCP Management - First Version
- Platform settings persist one MCP gateway singleton and one platform-owned remote MCP server registry with stable create, update, disable, enable, and delete behavior.
- Workspace MCP page shows inherited platform servers for the active workspace and supports current-workspace enable or disable behavior without local connection-definition editing.
- Workspace internal-agent policy supports disabling MCP, inheriting all enabled workspace MCP servers, or using a selected-server list.
- Internal-agent runtime preparation discovers tools from the effective MCP servers and materializes them as callable `mcp.<server_code>.<tool_name>` tools for the current run only.
- Disable and delete semantics remain distinct: disable preserves references while removing future effective availability, and delete cascades dependent workspace overrides and selected-server assignments.

== Risks And Open Questions
- The MCP transport auth boundary should follow MCP HTTP bearer-token and resource-server conventions, but the exact platform authorization-server integration should remain implementation-scoped and not expand this first product surface into a full IAM feature.
- Dynamic MCP tool federation changes runtime preparation and execution boundaries, so stale-assignment and discovery-failure behavior must be tested carefully against the current static tool-catalog assumptions.
- Workspace-local authoring was intentionally deferred; product copy and page layout must make the first-version platform-ownership rule obvious to avoid operator confusion.
