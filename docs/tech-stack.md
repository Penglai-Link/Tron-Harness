# Tech Stack

## Summary

This project uses a **Rust + React monorepo architecture**.containing both the frontend and backend codebases in a single repository.  

The frontend focuses on component-first development and strong typing, while the backend provides a modular, high-performance Rust service with a pluggable storage layer. Backend storage may have multiple implementations behind repository boundaries; this version uses the SQLite implementation.

The final application is deployed as an integrated full-stack service: the frontend build output is bundled into the backend, and the backend serves both the API and the web UI.

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

## Build and Runtime Model

The project is designed to produce a single backend-driven runtime artifact.

### Development Mode

- Frontend can run independently using Vite dev server
- Backend can run independently using Cargo
- Backend storage is selected by backend configuration. `TRON_BACKEND_DATABASE_URL` points the current SQLite implementation at a file path or SQLite URI, and the default SQLite file lives under `$HOME/.tron/tron.sqlite3`.
- Frontend components use mock data by default
- API integration is isolated behind hooks
- Frontend data source selection is environment-driven:
  - `VITE_TRON_DATA_SOURCE=mock|api` sets the global default
  - `VITE_TRON_<DOMAIN>_DATA_SOURCE=mock|api` overrides one route domain, for example `VITE_TRON_VARIABLE_DATA_SOURCE=api`
  - `VITE_TRON_API_BASE_URL` configures the backend API origin when API mode is enabled
  - Page components must import route-domain data hooks from `src/domain/<domain>/hooks`; endpoint contracts stay isolated under `src/domain/<domain>/api`, and static fixtures stay isolated under `src/domain/<domain>/mock/data.ts`
  - Data hook names must describe result shape or access pattern, for example `useModelList`, `useTaskDetail`, `useAppList`, `useImportApplicationDefaultsBySource`; use `Page` for paginated results and `By<Condition>` for conditional reads instead of generic `Query` suffixes

### Production Mode

- Frontend is built into static files
- Frontend build output is placed inside the backend service
- Backend serves:
  - REST or application APIs
  - Frontend static assets
  - Web application entry point
- Backend persistence uses the configured SQLite repository implementation for this version.

After the backend service starts, users can access the application UI directly through the backend server.
