# Feature Spec: Integrated Frontend Runtime

## Summary

The production runtime should serve the built React frontend and Rust API from the same backend process and port. The frontend remains mounted at the root path, while backend API routes remain reserved under `/api`.

## Scope

- Add a Makefile workflow that builds the frontend directly into the backend frontend resource directory without changing existing frontend data-source choices.
- Let the Rust backend serve built frontend assets and return `index.html` for client-side routes.
- Keep `/api` paths owned by backend API routing so unknown API routes return the uniform JSON response envelope instead of frontend HTML.
- Preserve the existing split development workflow for Vite and Cargo.

## Routing Model

- `/api/...` routes are backend API routes.
- `/assets/...` and other existing build files are served from the backend frontend resource directory.
- `/`, `/login`, `/dashboard`, and other non-API client routes return the frontend entry point.
- Missing frontend asset paths that look like files return 404 instead of the SPA entry point.

## Build Model

- `make fe-build` keeps the current standalone frontend output behavior.
- `make app-build` builds frontend assets into `tron-backend/resources/frontend` with same-origin API configuration, while preserving whatever data-source settings the frontend already had. Mock modules remain mock unless an existing environment file or shell setting switches them to API.
- `make app-run` builds those frontend resources and starts the backend.
- `make app-clean` removes generated integrated frontend assets while preserving the resource directory.

## Compatibility

Existing backend API handlers, frontend standalone development, and Vite dev proxy behavior remain available. The integrated mode adds a deployment-oriented workflow without requiring a separate frontend port. It does not promote frontend modules from mock to API as a side effect of deployment packaging.
