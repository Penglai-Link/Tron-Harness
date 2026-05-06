# Tron - Product Requirements Document

## 1. Product Overview
Tron is a centralized enterprise management console designed for internal R&D teams to govern AI Agent resources. It provides a unified interface to manage the full lifecycle of agent operations, focusing on resource isolation, hierarchical inheritance, and deep observability across workspaces.

## 2. Core Concepts & Logic
*   **Workspace Isolation:** All data and resources (Models, Memory, Chats, etc.) are strictly isolated by Workspace to ensure security and organizational clarity.
*   **Hierarchical Inheritance:** Workspaces support up to 3 levels of inheritance. Resources can be:
    *   **Inherited:** Taken directly from a parent workspace.
    *   **Local:** Created specifically for the current workspace.
    *   **Override:** A local modification of an inherited resource.
*   **Core Workflows:**
    1.  **Manual/Git Workflow:** Enter Workspace -> Import App -> Create/Import Task -> Assign Agent -> Execution/Chat -> Memory R/W -> Output.
    2.  **Meta-Agent Workflow:** Enter Workspace -> Describe Need to Meta-Agent -> Plan Output -> User Confirmation -> Automatic Task/App Binding -> Execution.

## 3. Screen Inventory & Functional Requirements

### 3.1 Authentication
*   **Login Page:** Minimalist enterprise entry with Username, Password, and Captcha verification over a professional particle background.

### 3.2 Workspace & Resource Governance
*   **Dashboard:** High-level overview of workspace health, active agents, and resource inheritance visualization.
*   **Workspace Settings:** Full-page management of hierarchy, inheritance policies, and member access with a persistent audit log.
*   **App Management:** Governance for local and remote (GitHub/GitLab) projects. Includes Git worktree management and "Open with IDE" integration.
*   **Task Management:** Agile Kanban board with progress tracking and deep-dive task detail pages (including sub-task auto-calculation).

### 3.3 Agent Orchestration
*   **Chat Sessions:** Three-pane orchestration interface with a tree-style session browser (Task-Title nested), reasoning-transparent chat cards, and a multi-tab utility hub (Details, Artifacts, Tasks, Traces).
*   **Prompt Management:** Centralized registry for System, Persona, and Tool prompts with versioning and inheritance tracking.
*   **Skill/Tool Management:** Registry for custom JSON-schema skills and MCP (Model Context Protocol) server governance.
*   **Model Registry:** Vendor-agnostic model configuration (OpenAI, Anthropic, etc.) with real-time parameter testing.

### 3.4 Data & Observability
*   **Memory Management:** High-density repository for agent context with advanced retrieval strategies (Keyword, Semantic, Hybrid) and detailed provenance tracking.
*   **Knowledge Graph:** Visual and searchable management of entity relationships with domain-level data isolation.
*   **Observability:** Telemetry dashboard focusing on token governance (Input, Cached, Output), model-specific consumption, and agent activity heatmaps.

### 3.5 Administration & Security
*   **User Directory:** Management of platform users and roles.
*   **Role Matrix:** Granular permission grid for RBAC across menus, data, and actions.
*   **Audit Logs:** Searchable, paginated system-wide activity log for compliance and troubleshooting.

### 3.6 Internationalization & Localization
*   **Supported Locales:** Tron must support English (`en-US`) and Simplified Chinese (`zh-CN`) from the first production-ready implementation. `en-US` is the fallback locale when a translation is missing.
*   **Language Switching:** Authenticated users must be able to switch the UI language from the global App Shell, preferably through the user menu or workspace preferences. The selected locale must persist across sessions.
*   **Localized UI Copy:** All user-facing navigation labels, page titles, form labels, helper text, empty states, validation messages, table headers, filters, action labels, toasts, dialogs, and error messages must be localized.
*   **Localized Runtime Values:** Dates, times, durations, numbers, percentages, token counts, file sizes, and currency values must be formatted through locale-aware formatters rather than hard-coded string templates.
*   **Terminology Consistency:** Domain terms such as Workspace, Agent, Task, Memory, Skill, Model, Prompt, Resource, Inherited, Local, and Override must use a shared terminology glossary so translations remain consistent across modules.
*   **Content Scope:** User-generated content, logs, source code, identifiers, model names, API keys, repository paths, and external provider names must not be machine-translated by the UI layer unless a separate translation workflow is explicitly introduced.

## 4. Visual Identity & Design System
*   **Theme:** Light (Tron - Apple-style minimalism) and Dark (Autonomous Command - Developer-tool style).
*   **Layout:** Standardized "Top Bar + Left Sidebar + Right Sidebar" shell across all management views to ensure platform-wide consistency.
    *   Collapsible Multi-level Sidebar: The primary sidebar uses sectioned accordion navigation, with only the active section expanded by default; the secondary sidebar is an optional contextual panel.
    *   Persistent App Shell: After authentication, the Top Bar, Left Sidebar, global search, workspace switcher, notification entry, user entry, and Main Agent entry must remain mounted across internal route changes. Internal navigation must replace only the main content region and optional contextual right panel.
*   **Assistant:** A persistent, floating "Main Agent" button for instant contextual help.

## 5. Technical Constraints
*   Responsive web application optimized for Desktop (1440px+).
*   High-density information architecture without sacrificing whitespace and clarity.
*   Modular UI components to support consistent resource management patterns.

### 5.1 SPA Navigation & Route Behavior
*   The authenticated Tron console must provide a single-page application navigation experience. Except for login, logout, external links, and file downloads, internal navigation must not trigger a full browser document reload.
*   Sidebar navigation, top-level navigation, and primary in-product workflow transitions must use client-side routing. Internal links such as `/dashboard`, `/apps`, `/tasks`, `/automations`, `/chats`, `/prompts`, `/models`, `/skills`, `/resources`, `/memory`, `/knowledge-graph`, `/variables`, `/observability`, and `/admin/*` must not be implemented as raw document navigations.
*   Route changes must update the browser URL, active navigation state, page title, and main content region while keeping the global App Shell mounted.
*   Shell state must survive internal route changes, including sidebar collapse state, expanded sidebar sections, global search input state, workspace switcher state, notification panel state, user menu state, and Main Agent open state.
*   Page-level content may fetch data, re-render, or show local loading states during route changes, but the App Shell must not unmount, flash blank, or visually reset.

### 5.2 Navigation Acceptance Criteria
*   In a real browser test, navigating from `/dashboard` to `/tasks`, `/models`, `/memory`, and `/admin/users` through the sidebar must not cause a `document` reload.
*   Acceptance tests must verify that the App Shell remains mounted during internal route transitions. For example, collapse the sidebar or expand a sidebar section, navigate to another management page, and confirm that the same shell state is preserved.
*   Acceptance tests must verify that only the main content region and optional contextual right panel change during normal management-page navigation.
*   Browser back and forward controls must work for internal routes without forcing a full document reload.
*   Internal navigation must be covered by end-to-end tests or browser-driven smoke tests, not only by static screenshots or route existence checks.

### 5.3 Internationalization Acceptance Criteria
*   No production management page may ship with hard-coded user-facing UI copy in React components. Text must resolve through translation keys, except for proper nouns, technical identifiers, and user-generated content.
*   Switching between `en-US` and `zh-CN` must update the App Shell and current page content without a full browser document reload.
*   Locale selection must persist after route changes, browser refresh, and a new authenticated session.
*   Missing translations must fall back to `en-US` and must be detectable through development warnings, tests, or build-time checks.
*   Browser-driven tests must cover at least the login page, dashboard, task management, model registry, memory management, and administration routes in both supported locales.
*   Layout checks must verify that translated text does not overflow buttons, tabs, table headers, sidebar items, cards, dialogs, or form controls at desktop and mobile breakpoints.
