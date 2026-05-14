# Rust Coding Spec

## DDD Hexagonal Implementation Rules

Backend workers must read and follow this specification before implementing
backend source changes. `tron-backend/docs/tech-stack.md` owns the stack and
architecture overview; this file owns enforceable Rust implementation rules.

For DDD + Hexagonal Architecture, dependency direction is fixed:

```text
adapters/inbound -> application -> domain
adapters/outbound -> application ports
```

- `domain` owns pure business model, invariants, value objects, domain
  policies, domain events, and domain errors.
- `application` owns use cases, application services, input/output DTOs for use
  cases, transaction orchestration, and ports.
- `adapters/inbound` owns HTTP/Axum routes, handlers, request DTOs, response
  DTOs, and error mapping.
- `adapters/outbound` owns concrete SQLite, bcrypt, ID generator, clock, file,
  and external service implementations.
- `app.rs` is the composition root. It wires configuration, outbound adapters,
  application services, inbound routes, and application state.

## Bounded Context Layout

Use modern Rust module layout. Do not create new `mod.rs` files for bounded
contexts or HTTP route groups.

Use this shape for bounded contexts:

```text
src/
  domain.rs
  domain/
    user.rs
    user/
      aggregate.rs
      profile.rs
      identity.rs
      lifecycle.rs
      assignment.rs
      assignment/
        role_assignment.rs
        space_assignment.rs
      password.rs
      password/
        state.rs
        hash.rs
        reset.rs
      query.rs
      query/
        list_users.rs
        user_statistics.rs

  application.rs
  application/
    user.rs
    user/
      use_case.rs
      port.rs
      service.rs
      dto.rs
      error.rs

  adapters.rs
  adapters/
    inbound/
      http/
        admin/
          users.rs
          users/
            routes.rs
            handler.rs
            request.rs
            response.rs
            error_mapper.rs
    outbound/
      persistence/
        sqlite/
          user_repository.rs
      crypto/
        bcrypt_password_hasher.rs
```

Files such as `domain/user.rs`, `domain/user/password.rs`, and
`domain/user/assignment.rs` are module facades. They should contain `mod`
declarations, controlled `pub use` exports, and at most small shared constants or
type aliases. They must not become large implementation buckets.

Folders are not passive Java-style package folders. In Rust, source folders are
part of the module tree when they are connected by `mod`. Use private submodules
for internal organization and facade `pub use` exports to keep public APIs
stable. Callers should depend on bounded-context exports such as
`crate::domain::user::UserCode`, not deep internal paths.

## Splitting Rules

Split by domain concept, not by broad DDD buckets.

- Prefer concept modules such as `profile`, `identity`, `lifecycle`,
  `assignment`, `password`, and `query`.
- Avoid catch-all files such as `entity.rs`, `query.rs`, or `policy.rs` once
  they contain unrelated concepts or multiple reasons to change.
- Introduce deeper private submodules only when a concept grows beyond a small
  cohesive file set. For example, `password/reset.rs` may own `reset/*` only
  after reset behavior has policy, result, event, and temporary-password pieces.
- Keep leaf files focused on one business concept or adapter responsibility.

## Prohibitions

- Do not put business rules in HTTP handlers.
- Do not let `domain` depend on `axum`, `rusqlite`, HTTP status codes, API
  envelopes, or concrete infrastructure libraries.
- Do not let outbound adapters define application or domain contracts. They
  implement ports defined by `application`.
- Do not store an entire bounded context in one large
  `domain/<context>/mod.rs`, `domain/<context>.rs`, or catch-all
  `entity.rs`/`query.rs` file.
- Do not store an entire HTTP route group in one large `http/.../mod.rs` or
  `http/.../<route>.rs` file.

## Storage And Runtime

- Storage must be behind application ports so SQLite can be replaced or extended
  without changing domain logic.
- SQLite is the current lightweight deployment storage implementation.
- Backend APIs and frontend static assets are served from one runtime service.
