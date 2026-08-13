## Backend Tech-Stack

The backend is implemented in **Rust** and follows a **domain-driven** architecture, with a focus on performance, safety, and modularity.

### Stack

- **Rust**: primary backend language
- **Cargo**: package management, build automation, testing, and workspace management
- **Storage Repository Boundary**: backend domains should depend on repository abstractions so storage can have multiple implementations over time
- **SQLite**: current version's storage implementation; configured with `TRON_BACKEND_DATABASE_URL`; defaults to `$HOME/.tron/tron.sqlite3`
- **Config File**: use yaml
- **utoipa**: code-first OpenAPI 3.1 generation; HTTP request/response DTOs and handlers are annotated incrementally by bounded context
- **Scalar**: interactive OpenAPI preview, served by the backend at `/api/docs`; the YAML document is available at `/api/openapi.yaml`
- **OpenAPI Export**: `make openapi` generates the repository-root `openapi.yaml` for CI validation and external consumers
