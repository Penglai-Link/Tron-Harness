Always follow these spec when coding:  
- tron-frontend/docs/spec/DESIGN.md
- tron-frontend/docs/tech-stack.md
- tron-frontend/docs/spec/react-coding-spec.md
- tron-frontend/docs/spec/quality-gates.md

Browser verification note:
- OAuth2 login normalizes loopback URLs to `127.0.0.1`; after callback, keep browser checks on `127.0.0.1` because switching back to `localhost` loses auth cookies.
