# Feature Evolution Log: OpenAPI HMAC Auth Foundation

## Summary

- feature_slug: openapi-hmac-auth-foundation
- status: done

## Change History

- 2026-06-11: implemented the OpenAPI HMAC auth foundation on `feature/clouds3n/openapi-open-app-management/base`, including open-app persistence, signed-request verification, explicit acting-user and capability authorization, and current agent-chat route mapping.

## Downstream Artifact Migration

- No downstream workspace migration was required because the dependent `openapi-open-app-management` slice now reuses the same open-app domain and service foundation on the base branch.

## Residual Risks

- Follow-up features still need operator CRUD endpoints, settings UI, and adapter-package updates that consume this foundation without reintroducing bearer fallback.
