DO $$
BEGIN
 IF EXISTS(SELECT 1 FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname IN ('forum_legacy_source_event','forum_freeze_legacy_source','forum_reconcile_school','forum_snapshot_source_v2','forum_freeze_source','forum_enqueue_material','forum_source_event')) THEN RAISE EXCEPTION 'obsolete source protocol functions'; END IF;
 IF to_regclass('forum_source_route_states') IS NOT NULL OR to_regclass('source_reconciliation_jobs') IS NOT NULL THEN RAISE EXCEPTION 'obsolete withdrawal state'; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_constraint WHERE conrelid='forum_outbox'::regclass AND conname='forum_received_receipt') THEN RAISE EXCEPTION 'missing receipt constraint'; END IF;
 IF NOT EXISTS(SELECT 1 FROM pg_proc WHERE pronamespace=current_schema()::regnamespace AND proname='forum_finalize_prepared_source' AND pronargs=5) THEN RAISE EXCEPTION 'missing complete-message finalizer'; END IF;
END $$;
