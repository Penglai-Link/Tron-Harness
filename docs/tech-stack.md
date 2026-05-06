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

The frontend follows a **component-first** development approach.  
UI is built from reusable and composable components before assembling pages and application flows.

### Stack

- **React**: latest stable version
- **Vite 8**: fast frontend build tool and development server
- **TypeScript**: latest stable version
- **PreText**: text/state-oriented UI abstraction  
  - https://github.com/chenglou/pretext
- **shadcn/ui**: component system built on Radix UI and Tailwind CSS
- **i18next + react-i18next**: React internationalization runtime and translation hook/component layer
- **npm**: package management

### Frontend Principles

- Component-first architecture
- Reusable and composable UI components
- Organize pages by route domain
- Strong typing with TypeScript
- UI primitives based on shadcn/ui, Radix UI, and Tailwind CSS
- All data-dependent components use mock data during frontend development
- Hooks are reserved as the boundary for future backend API integration
- Fast local development powered by Vite
- Minimal and composable frontend abstractions
- Internationalization is a first-class frontend concern; user-facing UI text must be resolved through translation keys rather than hard-coded component strings

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
