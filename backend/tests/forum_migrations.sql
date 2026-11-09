DO $$ BEGIN
 IF (SELECT application FROM application_metadata) <> 'linjian' THEN RAISE EXCEPTION 'wrong application'; END IF;
 IF EXISTS(SELECT 1 FROM information_schema.tables WHERE table_schema='public' AND (table_name LIKE 'mobile_%' OR table_name LIKE 'broadcast_%' OR table_name='wecom_groups')) THEN RAISE EXCEPTION 'sender schema leaked'; END IF;
END $$;
INSERT INTO schools(id,name) VALUES('00000000-0000-0000-0000-000000000001','学校');
INSERT INTO accounts(id,username,display_name) VALUES('00000000-0000-0000-0000-000000000002','member','成员');
INSERT INTO school_memberships VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002','member');
INSERT INTO posts(id,school_id,author_id,title,content) VALUES('00000000-0000-0000-0000-000000000003','00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002','标题','正文');
DO $$ BEGIN IF EXISTS(SELECT 1 FROM forum_outbox) THEN RAISE EXCEPTION 'pending post enqueued'; END IF; END $$;
BEGIN;
UPDATE posts SET audit_status='approved';
ROLLBACK;
DO $$ BEGIN IF EXISTS(SELECT 1 FROM forum_outbox) THEN RAISE EXCEPTION 'rolled back post enqueued'; END IF; END $$;
UPDATE posts SET audit_status='approved';
UPDATE posts SET browse=browse+1;
INSERT INTO post_reactions VALUES('00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000003','00000000-0000-0000-0000-000000000002','like');
DO $$ BEGIN IF (SELECT count(*) FROM forum_outbox)<>1 THEN RAISE EXCEPTION 'count/reaction caused push'; END IF; END $$;
UPDATE posts SET content='编辑正文',revision=revision+1;
UPDATE posts SET visibility='hidden';
DO $$ BEGIN
 IF (SELECT count(*) FROM forum_outbox)<>3 THEN RAISE EXCEPTION 'missing edit or withdrawal'; END IF;
 IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE sequence=3 AND payload->>'event_type'='content.withdrawn' AND payload->>'content' IS NULL) THEN RAISE EXCEPTION 'bad withdrawal payload'; END IF;
END $$;
UPDATE posts SET visibility='visible';
UPDATE schools SET active=false;
SELECT forum_reconcile_school('00000000-0000-0000-0000-000000000001');
DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE sequence=5 AND payload->>'event_type'='content.withdrawn') THEN RAISE EXCEPTION 'school disable did not withdraw'; END IF; END $$;
UPDATE schools SET active=true;
DO $$ BEGIN IF (SELECT count(*) FROM forum_outbox)<>5 THEN RAISE EXCEPTION 'school restore replayed history'; END IF; END $$;
-- Import suppresses historical events; future edits still enqueue.
ALTER TABLE posts DISABLE TRIGGER forum_post_outbox;
INSERT INTO posts(id,school_id,author_id,title,content,audit_status) VALUES('00000000-0000-0000-0000-000000000004','00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002','历史标题','历史正文','approved');
ALTER TABLE posts ENABLE TRIGGER forum_post_outbox;
DO $$ BEGIN IF (SELECT count(*) FROM forum_outbox)<>5 THEN RAISE EXCEPTION 'history import sent'; END IF; END $$;
UPDATE posts SET content='历史编辑' WHERE id='00000000-0000-0000-0000-000000000004';
DO $$ BEGIN IF (SELECT count(*) FROM forum_outbox)<>6 THEN RAISE EXCEPTION 'history edit missing'; END IF; END $$;
-- Real media bytes survive PostgreSQL JSON snapshot conversion.
INSERT INTO media_assets(id,school_id,owner_id,mime_type,size_bytes,content) VALUES('00000000-0000-0000-0000-000000000005','00000000-0000-0000-0000-000000000001','00000000-0000-0000-0000-000000000002','image/png',3,decode('0001ff','hex'));
DO $$ DECLARE original media_assets; copied media_assets;
BEGIN
 SELECT * INTO original FROM media_assets LIMIT 1;
 SELECT * INTO copied FROM jsonb_populate_record(NULL::media_assets,to_jsonb(original));
 IF copied IS DISTINCT FROM original THEN RAISE EXCEPTION 'media roundtrip mismatch'; END IF;
END $$;
DO $$ BEGIN
 IF (SELECT count(*) FROM sessions)<>0 THEN RAISE EXCEPTION 'unexpected migrated sessions'; END IF;
END $$;
