# Business statistics

`domain` defines Shanghai calendar windows, exact metrics and coverage. `north/local` applies management capabilities, bounds reader concurrency, and formats snapshot CSV. `north/remote` adapts HTTP without SQL. `south/port` separates report/snapshot access and activity recording; PostgreSQL adapters own queries, fresh authorization, registration facts, coverage and persisted snapshots.

Registration facts are captured in the miniapp registration transaction. Subsequent account school changes do not move them. Historical registrations retain their timestamps but have unknown school assignment. Active access is collected asynchronously through a bounded queue, using server-authenticated identities and school/resource checks. Only miniapp login, page access and successful content creation count; admin requests do not.

Reports use one repeatable-read transaction, then persist only aggregate results for ten minutes. Exports and school pagination read that original result and recheck current account/school permissions. Platform administrators can view all schools, including hidden schools; school admins/operators cannot query platform data or other schools. The ordinary management session and Origin/CSRF rules remain in place.

The worker records sealed time ranges and failed intervals. It retries bounded batches, preserves crash/overflow gaps, and starts a new interval after recovery. Verified client deployment is required before claiming complete access coverage. No statistics operation increments browse, creates Outbox, or gates forum readiness.

See `docs/guide/business-statistics.md` and `report/论坛业务统计实施方案.md` for configuration, definitions and validation. Database tests run through `make test-integration` with an explicit isolated `LINJIAN_TEST_DATABASE_URL`; `backend/examples/test_database.rs` provisions a fresh temporary server for that target.
