# Tech Stack

This project is organized as a **full-stack monorepo**, containing both the frontend and backend codebases in a single repository.

The frontend and backend are developed independently but integrated into one deployable application.  
During the build process, the frontend static assets are generated and embedded or copied into the backend service, so that once the backend starts, the web UI can be accessed directly from the backend server.

---

## Architecture Overview

- **Monorepo structure**
  - Frontend and backend live in the same repository
  - Shared development workflow
  - Unified build and release process

- **Integrated deployment**
  - Frontend is built into static assets
  - Build output is placed into the backend service
  - Backend serves both API endpoints and frontend pages
  - Running the backend is enough to access the complete application

---

## Frontend

The frontend follows a **domain-driven frontend** architecture.
Application bootstrapping, routing, domain modules, shared UI, global state, and internationalization are separated by responsibility so that route features can evolve without turning shared components or app shell code into domain-specific dumping grounds.

### Stack

- **React**: latest stable version
- **React Router**: route matching, browser history integration, redirects, and routable page composition
- **Zustand**: lightweight global frontend state for app-shell UI and cross-route client state
- **Vite 8**: fast frontend build tool and development server
- **TypeScript**: latest stable version
- **PreText**: text/state-oriented UI abstraction  
  - https://github.com/chenglou/pretext
- **shadcn/ui**: component system built on Radix UI and Tailwind CSS
- **i18next + react-i18next**: React internationalization runtime and translation hook/component layer
- **npm**: package management

### Frontend Naming Conventions

- React component files (`.tsx`) use PascalCase, for example `UserCard.tsx` and `App.tsx`
- TypeScript files whose primary export is an interface or type use PascalCase, for example `UserProps.ts` and `ApiData.ts`
- Ordinary TypeScript modules, functions, services, adapters, stores, schemas, and constants use camelCase, for example `utils.ts`, `authService.ts`, `taskApi.ts`, and `taskStatus.ts`
- React hook modules use camelCase and start with `use`, for example `useTaskList.ts` and `useTaskDetail.ts`
- Directory names use kebab-case for multi-word feature or domain folders, for example `create-task`, `knowledge-graph`, and `surface-card`
- `index.ts` is allowed only as an explicit public barrel for a directory; do not hide unrelated implementation code in `index.ts`

### Frontend Principles

- Domain-driven frontend architecture
- Reusable and composable UI components
- App bootstrapping belongs in `src/app/`
- Route declarations and page registration belong in `src/router/`
- Route-domain code is colocated under `src/domain/<domain>/`:
  - `api/` contains endpoint clients, query/mutation hooks, and data-source switching logic
  - `constants/` contains domain constants and enum-like contracts
  - `schema/` contains validation, query, and form schemas
  - `model/` contains domain data models, DTO contracts, and domain-local stores
  - `adapters/` contains mapping between API DTOs, mock data, and UI models
  - `hooks/` contains domain-specific React hooks that are not a feature implementation detail
  - `components/` contains domain-specific presentational components
  - `features/` contains user-action slices such as create, import, and delete flows
  - `pages/` contains thin route page components that compose domain features and components
  - `mock/` contains static development fixtures
- Shared, domain-agnostic UI, layout, API, hooks, utilities, constants, and types belong in `src/shared/`
- Global app-shell state that is not owned by one domain belongs in `src/store/`
- Strong typing with TypeScript
- UI primitives based on shadcn/ui, Radix UI, and Tailwind CSS
- All data-dependent components use mock data during frontend development
- Hooks are reserved as the boundary for future backend API integration
- Routing uses React Router as the browser routing runtime; route metadata may remain in domain-specific typed registries when navigation labels, active menu state, and page archetypes need to stay colocated
- Cross-route UI state uses Zustand rather than ad hoc router state or deeply nested shell-local state
- Fast local development powered by Vite
- Minimal and composable frontend abstractions
- Internationalization is a first-class frontend concern; user-facing UI text must be resolved through translation keys rather than hard-coded component strings

### Frontend Directory Standard

New frontend code must follow this target structure. Existing legacy paths should migrate toward this shape when touched; avoid adding new domain code to legacy catch-all component folders.

```text
src/
|-- app/
|   |-- main.tsx
|   |-- App.tsx
|   |-- providers/
|   |   |-- RouterProvider.tsx
|   |   |-- I18nProvider.tsx
|   |   `-- QueryProvider.tsx
|   `-- styles/
|       `-- globals.css
|-- router/
|   |-- routes.tsx
|   `-- pageRegistry.ts
|-- domain/
|   |-- task/
|   |   |-- api/
|   |   |   `-- taskApi.ts
|   |   |-- constants/
|   |   |   |-- taskStatus.ts
|   |   |   |-- taskPriority.ts
|   |   |   `-- index.ts
|   |   |-- schema/
|   |   |   |-- taskSchema.ts
|   |   |   |-- taskQuerySchema.ts
|   |   |   `-- taskFormSchema.ts
|   |   |-- model/
|   |   |   |-- TaskModel.ts
|   |   |   `-- taskStore.ts
|   |   |-- adapters/
|   |   |   `-- taskAdapter.ts
|   |   |-- hooks/
|   |   |   |-- useTaskList.ts
|   |   |   `-- useTaskDetail.ts
|   |   |-- components/
|   |   |   |-- TaskTable.tsx
|   |   |   |-- TaskStatusBadge.tsx
|   |   |   `-- TaskFilter.tsx
|   |   |-- features/
|   |   |   |-- create-task/
|   |   |   |   |-- CreateTaskForm.tsx
|   |   |   |   |-- useCreateTask.ts
|   |   |   |   `-- index.ts
|   |   |   |-- import-task/
|   |   |   |   |-- ImportTaskForm.tsx
|   |   |   |   |-- useImportTask.ts
|   |   |   |   `-- index.ts
|   |   |   `-- delete-task/
|   |   |       |-- DeleteTaskButton.tsx
|   |   |       |-- useDeleteTask.ts
|   |   |       `-- index.ts
|   |   |-- pages/
|   |   |   |-- TaskListPage.tsx
|   |   |   |-- TaskDetailPage.tsx
|   |   |   |-- TaskNewPage.tsx
|   |   |   `-- TaskImportPage.tsx
|   |   |-- mock/
|   |   |   `-- taskMock.ts
|   |   `-- index.ts
|   |-- prompt/
|   |   |-- api/
|   |   |-- constants/
|   |   |-- schema/
|   |   |-- model/
|   |   |-- adapters/
|   |   |-- hooks/
|   |   |-- components/
|   |   |-- features/
|   |   |-- pages/
|   |   |-- mock/
|   |   `-- index.ts
|   `-- user/
|       |-- api/
|       |-- constants/
|       |-- schema/
|       |-- model/
|       |-- adapters/
|       |-- hooks/
|       |-- components/
|       |-- features/
|       |-- pages/
|       |-- mock/
|       `-- index.ts
|-- shared/
|   |-- ui/
|   |   |-- Button.tsx
|   |   |-- Badge.tsx
|   |   |-- Dialog.tsx
|   |   `-- DataTable.tsx
|   |-- layout/
|   |   |-- Shell.tsx
|   |   |-- PageHeader.tsx
|   |   `-- SurfaceCard.tsx
|   |-- api/
|   |   |-- httpClient.ts
|   |   `-- ApiError.ts
|   |-- hooks/
|   |   `-- useDebounce.ts
|   |-- lib/
|   |   |-- utils.ts
|   |   `-- cn.ts
|   |-- constants/
|   |   `-- pagination.ts
|   `-- types/
|       `-- Common.ts
|-- i18n/
|   |-- index.ts
|   `-- locales/
|       `-- en-US/
|-- store/
|   `-- appShellStore.ts
`-- vite-env.d.ts
```

### Frontend Internationalization

The frontend must support internationalization from the first implementation pass rather than retrofitting it after pages are built.

- **Required locales**
  - `en-US`: default source and fallback locale
  - `zh-CN`: required Simplified Chinese locale

- **Runtime behavior**
  - Locale state is available globally through the React app shell
  - Users can switch locale without a full browser document reload
  - Locale preference is persisted locally during frontend-only development and must later sync to the authenticated user profile
  - Missing keys fall back to `en-US` and must be visible during development

- **Translation structure**
  - Translation resources live under `src/i18n/locales/<locale>/`
  - Namespaces are organized by stable product areas, such as `common`, `shell`, `auth`, `dashboard`, `tasks`, `apps`, `models`, `memory`, `prompts`, `skills`, `resources`, `admin`, and `errors`
  - Route metadata, navigation labels, page headers, form labels, validation text, table headers, action labels, dialogs, empty states, toast messages, and error messages all use translation keys
  - Domain terminology is centralized in a shared glossary namespace to keep terms such as Workspace, Agent, Task, Memory, Skill, Model, Prompt, Resource, Inherited, Local, and Override consistent

- **Formatting**
  - Dates, times, durations, numbers, percentages, token counts, file sizes, and currency values use locale-aware formatters based on the JavaScript `Intl` APIs
  - User-generated content, logs, code, identifiers, repository paths, model names, provider names, and secrets are displayed as authored and are not translated by the UI runtime

- **Quality gates**
  - TypeScript checks must catch invalid translation key usage where practical
  - Browser-driven smoke tests cover the app shell and representative pages in both `en-US` and `zh-CN`
  - Layout checks verify translated strings do not overflow sidebar items, buttons, tabs, table headers, cards, dialogs, or form controls

---

## Backend

The backend is implemented in **Rust**, with a focus on performance, safety, and modularity.

### Stack

- **Rust**: primary backend language
- **Cargo**: package management, build automation, testing, and workspace management
- **SQLite**: default lightweight storage option

### Backend Principles

- Modular architecture
- Pluggable storage layer
- Storage abstraction based on interfaces/traits
- SQLite as the default lightweight deployment option
- Extensible design for future databases or storage engines
- Serves backend APIs and frontend static assets from a single runtime service

---

## Build and Runtime Model

The project is designed to produce a single backend-driven runtime artifact.

### Development Mode

- Frontend can run independently using Vite dev server
- Backend can run independently using Cargo
- Frontend components use mock data by default
- API integration is isolated behind hooks
- Frontend data source selection is environment-driven:
  - `VITE_TRON_DATA_SOURCE=mock|api` sets the global default
  - `VITE_TRON_<DOMAIN>_DATA_SOURCE=mock|api` overrides one route domain, for example `VITE_TRON_VARIABLE_DATA_SOURCE=api`
  - `VITE_TRON_API_BASE_URL` configures the backend API origin when API mode is enabled
  - Page components must import route-domain hooks from `src/domain/<domain>/api/index.ts`; static fixtures stay isolated under `src/domain/<domain>/mock/data.ts`

### Production Mode

- Frontend is built into static files
- Frontend build output is placed inside the backend service
- Backend serves:
  - REST or application APIs
  - Frontend static assets
  - Web application entry point

After the backend service starts, users can access the application UI directly through the backend server.

---

## Summary

This project uses a **Rust + React monorepo architecture**.

The frontend focuses on component-first development and strong typing, while the backend provides a modular, high-performance Rust service with a pluggable storage layer.

The final application is deployed as an integrated full-stack service: the frontend build output is bundled into the backend, and the backend serves both the API and the web UI.
