# Agent instructions

## Workflow

- Use the Makefile targets for build, test, and development commands. Run checks relevant to the task; do not run cleanup unless the task needs it and the target scope is clear.
- If a default port is occupied, choose an available port and keep frontend and backend configuration aligned.
- Follow the nearest `AGENTS.md` for frontend or backend work, and consult its linked specifications as relevant to the change.

## Shared contracts

- Backend JSON responses use `ApiResponse<T>` from `tron-backend/src/response.rs` (`code`, `message`, `data`). Follow each endpoint's declared pagination shape rather than assuming one universal DTO.
- Follow `$eliteforge-git-feature-oriented-lite-spec` for Git workflows.
