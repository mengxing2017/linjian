DO $$
BEGIN
 IF (SELECT count(*) FROM application_metadata WHERE singleton AND application='linjian' AND schema_generation=1) <> 1 THEN RAISE EXCEPTION 'missing application metadata'; END IF;
 IF (SELECT count(*) FROM site_settings WHERE singleton AND revision=0) <> 1 THEN RAISE EXCEPTION 'missing site defaults'; END IF;
 IF (SELECT count(*) FROM miniapp_registration_settings WHERE id=1 AND nickname_prefix='用户' AND revision=0) <> 1 THEN RAISE EXCEPTION 'missing registration defaults'; END IF;
 IF (SELECT count(*) FROM source_active_configuration WHERE singleton AND connection_id IS NULL) <> 1 THEN RAISE EXCEPTION 'missing source defaults'; END IF;
 IF (SELECT count(*) FROM statistics_coverage WHERE family IN ('content','registration') AND start_at='1970-01-01T00:00:00Z' AND end_at IS NULL AND status='complete' AND reason='fresh_database') <> 2 THEN RAISE EXCEPTION 'invalid fresh database coverage'; END IF;
 IF EXISTS(SELECT 1 FROM pg_tables WHERE schemaname=current_schema() AND tablename IN ('blocked_word_legacy_snapshot','school_source_binding_archive')) THEN RAISE EXCEPTION 'obsolete migration archive'; END IF;
 IF EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema=current_schema() AND table_name IN ('accounts','sessions') AND column_name LIKE 'wecom_%') THEN RAISE EXCEPTION 'obsolete account or session columns'; END IF;
 IF EXISTS(SELECT 1 FROM pg_constraint WHERE connamespace=current_schema()::regnamespace AND NOT convalidated) THEN RAISE EXCEPTION 'unvalidated baseline constraint'; END IF;
 IF EXISTS(SELECT 1 FROM accounts) OR EXISTS(SELECT 1 FROM schools) OR EXISTS(SELECT 1 FROM posts) OR EXISTS(SELECT 1 FROM forum_outbox) THEN RAISE EXCEPTION 'unexpected business seed data'; END IF;
END $$;
