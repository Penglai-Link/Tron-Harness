# Backend instructions

Consult `docs/tech-stack.md` for the architecture overview and `docs/spec/rust-coding-spec.md` for Rust implementation rules relevant to the change. Run `make be-check` and focused backend tests.

- Unit tests may use `#[cfg(test)]` beside the code; cross-module/API integration tests belong in `tests/`.
- Resolve filesystem and command execution from the selected application/worktree scope. A missing configured working directory is an error; never fall back to the backend process directory. `default_branch` is configuration, not proof of the checked-out Git branch.
- Agent command tools use a command string when `args` is omitted; explicit `args` are literal argv. Ordinary commands capture output and become task-ID-backed after 10 seconds; PTYs are for interactive or long-running processes and remain backend-owned, independent of frontend sockets or tabs.
- Resolve workspace and chat-attachment files through the shared `fileRef`; keep file exports scoped to project paths.
- `chat_ask_user` answers remain pending until the prior run releases the session; persist the answer summary in the continuation transcript.
- When creating a Person-AI Rust SDK client, set `X-Product-Code` at client construction so regular and SSE requests inherit it.
- `cargo fmt` may reformat unrelated files; review the diff and retain only changes in scope.
