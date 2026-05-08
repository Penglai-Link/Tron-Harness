# Frontend Coding Spec

## Frontend Naming Conventions

- React component files (`.tsx`) use PascalCase, for example `UserCard.tsx` and `App.tsx`
- TypeScript files whose primary export is an interface or type use PascalCase, for example `UserProps.ts` and `ApiData.ts`
- Ordinary TypeScript modules, functions, services, adapters, stores, schemas, and constants use camelCase, for example `utils.ts`, `authService.ts`, `taskApi.ts`, and `taskStatus.ts`
- React hook modules use camelCase and start with `use`, for example `useTaskList.ts` and `useTaskDetail.ts`
- Directory names use kebab-case for multi-word feature or domain folders, for example `create-task`, `knowledge-graph`, and `surface-card`
- `index.ts` is allowed only as an explicit public barrel for a directory; do not hide unrelated implementation code in `index.ts`

## Frontend Principles

- Domain-driven frontend architecture
- Reusable and composable UI components
- Use toast notifications for all user prompts.
- App bootstrapping belongs in `src/app/`
- Route declarations and page registration belong in `src/router/`
- Route-domain code is colocated under `src/domain/<domain>/`:
  - `api/` contains endpoint clients and backend contract names only
  - `constants/` contains domain constants and enum-like contracts
  - `schema/` contains validation, query, and form schemas
  - `model/` contains domain data models, DTO contracts, and domain-local stores
  - `adapters/` contains mapping between API DTOs, mock data, and UI models
  - `hooks/` contains domain-specific React hooks, including query/mutation hooks and mock-to-backend data-source switching calls
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

## Frontend Directory Standard

frontend code must follow this target structure.  

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

## Frontend Internationalization

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