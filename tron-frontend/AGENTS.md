# Frontend instructions

Consult the relevant project guidance when a change touches its area:

- `docs/spec/DESIGN.md` for visual and interaction design.
- `docs/tech-stack.md` and `docs/spec/react-coding-spec.md` for architecture and React conventions.
- `docs/spec/quality-gates.md` for the appropriate checks; normally run `make fe-check` and focused tests.
- `docs/spec/ui-components.md` when using or changing shared UI primitives.

- Before building a page or domain component, check `src/components/ui` and reuse its primitives first.
- Compose and style existing primitives through their public APIs. Add a domain component only for domain-specific behavior.
- Add a new primitive only when the existing primitives cannot support the interaction without compromising accessibility or leaking implementation details.
- When adding or materially changing a primitive under `src/components/ui`, update `docs/spec/ui-components.md` in the same change.
- Keep the component catalog accurate: verify file paths, primary exports, usage guidance, accessibility behavior, internationalization considerations, and focused tests.

- When changing the default sidebar hierarchy, update both `shellNavigation.ts` and the built-in SQLite menu migration; API-backed navigation uses the persisted tree.
- Chat API, SSE, and attachment requests must use the visible document locale in `Accept-Language`, including requests sent through `@tron/chat-internal-adapter`.
