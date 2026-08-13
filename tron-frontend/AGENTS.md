Always follow these spec when coding:  
- tron-frontend/docs/spec/DESIGN.md
- tron-frontend/docs/tech-stack.md
- tron-frontend/docs/spec/react-coding-spec.md
- tron-frontend/docs/spec/quality-gates.md
- tron-frontend/docs/spec/ui-components.md

UI component rule:
- Before building a page or domain component, check `src/components/ui` and reuse its primitives first.
- Compose and style existing primitives through their public APIs. Add a domain component only for domain-specific behavior.
- Add a new primitive only when the existing primitives cannot support the interaction without compromising accessibility or leaking implementation details.
- When adding or materially changing a primitive under `src/components/ui`, update `docs/spec/ui-components.md` in the same change.
- Keep the component catalog accurate: verify file paths, primary exports, usage guidance, accessibility behavior, internationalization considerations, and focused tests.

Browser verification note:
- OAuth2 login normalizes loopback URLs to `127.0.0.1`; after callback, keep browser checks on `127.0.0.1` because switching back to `localhost` loses auth cookies.
