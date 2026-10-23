# Marivect implementation rules

Read README.md, docs/STATUS.md and docs/AI_INSTRUCTIONS_REMOTE_MONITORING.md first. The latter is the product specification; the current repository is only its skeleton.

- Preserve Docker/Compose workflow. Modular Rust monolith with separate API/worker processes, PostgreSQL only initially.
- Never infer tenant identity from an unverified client tenant header or payload. Service credentials and human memberships are separate identities.
- Use transaction-scoped tenant context on the SAME connection. All tenant tables need RLS and composite foreign keys. Runtime must never own tables or bypass RLS.
- No fake Campbell/PakBus/ChirpStack transport, command acknowledgement, or placeholder success. Disabled functionality stays absent until implemented.
- Keep raw and normalized protocols separate. Adapter boundaries must not leak vendor field names into the core.
- Validate source, units and timestamps; commit before success; stable idempotency and late-data safe latest updates are required.
- Keep secrets out of git/logs. Do not commit generated .env or demo tokens.
- For implemented behavior run meaningful isolation tests, fmt/Clippy/tests and frontend typecheck/build. Report unavailable checks honestly.
- Complete human auth/RBAC before exposing any tenant management endpoints.
