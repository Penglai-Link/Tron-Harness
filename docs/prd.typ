= PRD: Tron AI Workspace Operations Console

#import "@preview/mmdr:0.2.2": mermaid

== Summary
- Product: Tron is an internal AI workspace operations console for enterprise R&D teams.
- Problem: teams need one authenticated control plane to govern users, workspaces, AI assets, task delivery, and scheduled agent workflows without splitting context across separate tools.
- Target users: platform admins, workspace operators, AI workflow maintainers, and engineering leads.
- Outcome: a workspace-scoped console that combines secure access, hierarchical governance, reusable AI configuration, delivery workflows, and runtime visibility in one backend-served web app.

== Goals
- Unify authentication, workspace selection, and operator workflows inside one persistent console shell.
- Enforce workspace-scoped governance for users, roles, apps, tasks, automations, models, skills, prompts, variables, and data surfaces.
- Support inheritance-aware configuration where platform or parent workspace definitions can be reused, overridden, or localized.
- Give operators a practical delivery loop for app registration, task planning, assignment, and automation scheduling.
- Keep the product deployable as one integrated backend-driven runtime artifact.
- Ship locale-aware UI copy and runtime formatting, with `en-US` as fallback and `zh-CN` as a required production locale.

== Non-Goals
- Public self-service signup, customer-facing tenancy, billing, or marketplace monetization.
- Full enterprise IAM administration such as MFA policy, SCIM, device trust, or external IdP management UI.
- Distributed scheduler coordination, multi-node leader election, or large-scale runtime orchestration in this version.
- Deep external import and sync flows for apps, automations, or registries beyond the scoped feature specs.
- Fine-grained policy engines or deny-rule conflict resolution beyond the current authenticated admin and workspace model.

== Scope
- In scope:
  - authenticated console access with internal credentials and GitHub OAuth
  - user, role, and workspace administration
  - workspace-scoped app, task, and automation workflows
  - AI asset governance for models, prompts, skills, variables, and resources
  - memory, knowledge, and observability surfaces aligned to workspace operations
  - integrated frontend-plus-backend runtime with SQLite-backed persistence for the current version
- Out of scope:
  - public onboarding and customer account lifecycle
  - multi-cloud deployment control plane
  - external billing, quota purchasing, or marketplace distribution
  - production-grade distributed execution fabric

== Key Requirements
- Every non-auth management route must require a valid authenticated session and preserve shell state during internal navigation.
- Authenticated users without assigned workspace spaces must be stopped on a dedicated authorization-request page and must not operate shell navigation or business pages until a responsible owner grants space access.
- A fresh deployment must initialize one platform administrator account with the fixed `ADMIN` platform role and root workspace access so the first owner can assign spaces; that platform administrator must not be trapped by the no-space authorization page.
- Workspace context must be first-class: selected workspace drives visible apps, tasks, automations, effective users, and inherited AI configuration.
- Core registries must support operator-safe list, search, pagination, lifecycle controls, and consistent feedback patterns.
- User and role administration must be stable enough to back login, assignment, workspace grants, and task assignee selection from one shared source of truth.
- App and task workflows must support real backend persistence, role-aware task statuses, and workspace-valid task assignments.
- Automations must support create, edit, run-now, status transitions, and execution history within the selected workspace.
- The product must keep locale-aware UI resolution and runtime value formatting rather than hard-coded English strings in page code.
- API responses must remain consistent enough for shared frontend hooks and predictable admin workflows.

== Shared Requirements
- Authenticated session model: shared because the entire console depends on one operator identity and protected routing; consumers: Features 1, 2, 3, 4, 5, 6; decision: build first
- Workspace context and inheritance rules: shared because registries, delivery flows, and data surfaces resolve from the selected workspace and lineage; consumers: Features 2, 3, 4, 5, 6; decision: build first
- Stable user and role directory: shared because auth, workspace grants, task assignees, and role-aware administration use the same canonical records; consumers: Features 1, 2, 4; decision: build first
- Search, pagination, and operator feedback patterns: shared because nearly every management surface needs consistent filtering, paging, loading, and toast feedback; consumers: Features 2, 3, 4, 5, 6; decision: build first
- Locale-aware UI and runtime formatting: shared because navigation, forms, tables, empty states, and counts must switch locale coherently; consumers: Features 1, 2, 3, 4, 5, 6; decision: build first
- Audit and activity ledger: shared because admin, workflow, and automation mutations eventually need a common compliance trail; consumers: Features 2, 4, 5, 6; decision: defer

== Feature Inventory
1. Console Access and Identity: goal establish secure operator access and protected shell bootstrap; in scope login, OAuth callback, current-session bootstrap, logout, bearer-token protected APIs, refresh-cookie session renewal, bootstrap platform admin initialization, and no-space authorization blocking; out of scope signup, MFA, invitation acceptance, external IdP admin UI; depends on stable user and role directory
2. Administration and Workspace Governance: goal let admins manage people, roles, and hierarchical workspaces from one control plane; in scope user directory, create/detail flows, role matrix catalog, workspace tree CRUD, workspace access grants, effective-user inspection, workspace switcher projections; out of scope billing, per-workspace custom role systems, advanced compliance policy; depends on Feature 1
3. AI Asset Registry: goal let each workspace control effective AI configuration without duplicating everything locally; in scope platform and workspace model management, prompt registry, skill registry and bind/install flows, variables, resources, inheritance-aware source states; out of scope marketplace sync, external package governance, automatic runtime synthesis; depends on Feature 2
4. Workspace Delivery Workflow: goal support practical planning and delivery inside the selected workspace; in scope app registry CRUD, task kanban, task detail, parent-child tasks, assignee validation, app-to-task binding, role-aware status flows; out of scope external import, execution engine orchestration, strict downstream runtime enforcement beyond current registry validation; depends on Feature 2
5. Scheduled Automations: goal enable repeatable workspace-scoped prompt execution with clear lifecycle control; in scope automation CRUD-like flows, active or paused lifecycle, run-now, execution history, in-process scheduling, latest-result rollups; out of scope DAGs, webhook triggers, distributed scheduler ownership, external scheduler sync; depends on Feature 2
6. Knowledge, Memory, and Observability: goal provide the data and visibility plane around agent operations; in scope memory management, knowledge graph/base surfaces, resources and observability views, runtime usage dashboards, operational context browsing; out of scope federated analytics, automated remediation, external knowledge-market integration; depends on Feature 2 and Feature 4

== Dependency Map
- Feature 2 depends on Feature 1
- Feature 3 depends on Feature 2
- Feature 4 depends on Feature 2
- Feature 4 depends on shared requirement: stable user and role directory
- Feature 5 depends on Feature 2
- Feature 6 depends on Feature 2
- Feature 6 depends on Feature 4
- Feature 6 depends on shared requirement: audit and activity ledger

#mermaid(
  "flowchart LR
    SR1[\"Shared Requirement: Authenticated Session Model\"] --> F1[\"Feature 1: Console Access and Identity\"]
    F1 --> F2[\"Feature 2: Administration and Workspace Governance\"]
    F2 --> F3[\"Feature 3: AI Asset Registry\"]
    F2 --> F4[\"Feature 4: Workspace Delivery Workflow\"]
    F2 --> F5[\"Feature 5: Scheduled Automations\"]
    F2 --> F6[\"Feature 6: Knowledge, Memory, and Observability\"]
    SR2[\"Shared Requirement: Stable User and Role Directory\"] --> F4
    F4 --> F6
    SR3[\"Shared Requirement: Audit and Activity Ledger\"] --> F6"
)

== Recommended Implementation Order
1. Stable user and role directory plus authenticated session model - this is the minimum foundation for any protected console behavior and assignment-driven workflow.
2. Workspace governance and workspace switcher projections - this establishes the tenancy boundary every other feature resolves against.
3. AI asset registry - models, prompts, skills, variables, and resources become reusable building blocks for downstream workflow surfaces.
4. Workspace delivery workflow - apps and tasks convert the governance foundation into operator execution planning and assignment loops.
5. Scheduled automations - scheduler-backed prompt workflows become valuable once workspace context and core operator patterns are stable.
6. Knowledge, memory, observability, and deferred audit deepening - these surfaces deliver the highest value after real workflows are already generating operational data.

== Risks And Open Questions
- Chat-session orchestration and Main Agent behavior exist in navigation and page surfaces, but their stable backend product contract is less explicit than admin, task, model, and automation modules.
- Audit-log expectations appear in design materials, but a shared cross-feature audit contract is still weaker than the other admin foundations.
- Production localization requires `zh-CN` alongside `en-US`, while the current codebase still centers delivery around `en-US` resources.
- The fixed three-level workspace hierarchy may be too shallow for some enterprise org structures; validate before broader rollout.
- Several data domains still support mixed mock and API modes in development; confirm the launch bar for calling a module production-ready.
