-- Uses its own transaction so it is runnable both by Rust and the standalone SQL runner.
BEGIN;
INSERT INTO schools(id,name) VALUES('40000000-0000-0000-0000-000000000001','动态甲校'),('40000000-0000-0000-0000-000000000002','动态乙校');
INSERT INTO accounts(id,username,display_name) VALUES('40000000-0000-0000-0000-000000000003','dynamic-test','动态');
INSERT INTO source_connections(id,name,source_id,origin,key_id,checked_at) VALUES('40000000-0000-0000-0000-000000000011','test','40000000-0000-0000-0000-000000000101','https://dynamic.example.com','dynamic',now());
INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin,public_origin) VALUES('40000000-0000-0000-0000-000000000021','40000000-0000-0000-0000-000000000011',1,'40000000-0000-0000-0000-000000000101','https://dynamic.example.com','https://forum.example.com');
UPDATE source_active_configuration SET connection_id='40000000-0000-0000-0000-000000000011';
INSERT INTO posts(id,school_id,author_id,title,content,audit_status) VALUES('40000000-0000-0000-0000-000000000041','40000000-0000-0000-0000-000000000001','40000000-0000-0000-0000-000000000003','未绑定标题','正文','approved');
CREATE TEMP TABLE initial_identity ON COMMIT DROP AS SELECT id FROM forum_outbox WHERE post_id='40000000-0000-0000-0000-000000000041';
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE id=(SELECT id FROM initial_identity) AND schema_version=2 AND delivery_state='waiting_binding' AND request_body IS NULL AND blocked_reason IS NULL) THEN RAISE EXCEPTION 'unbound event must be current v2 intent'; END IF;
END $$;
INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id) VALUES('40000000-0000-0000-0000-000000000031','40000000-0000-0000-0000-000000000001','40000000-0000-0000-0000-000000000011','40000000-0000-0000-0000-000000000021','40000000-0000-0000-0000-000000000101','https://dynamic.example.com','route-a');
SELECT forum_freeze_source((SELECT id FROM initial_identity),'40000000-0000-0000-0000-000000000021',(SELECT id FROM school_source_binding_versions WHERE binding_id='40000000-0000-0000-0000-000000000031'),'route-a');
CREATE TEMP TABLE original_frozen ON COMMIT DROP AS SELECT id,request_body,body_sha256 FROM forum_outbox WHERE id=(SELECT id FROM initial_identity);
UPDATE school_source_bindings SET active=false,revision=revision+1 WHERE id='40000000-0000-0000-0000-000000000031';
UPDATE posts SET content='暂停期间编辑' WHERE id='40000000-0000-0000-0000-000000000041';
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE post_id='40000000-0000-0000-0000-000000000041' AND sequence=2 AND route_id='route-a' AND delivery_state='paused' AND request_body IS NOT NULL) THEN RAISE EXCEPTION 'pause lost new binding snapshot'; END IF;
 IF (SELECT count(*) FROM school_source_binding_versions WHERE binding_id='40000000-0000-0000-0000-000000000031')<>2 THEN RAISE EXCEPTION 'missing binding history'; END IF;
END $$;
-- An uncertain request must remain represented in history before any receipt is known.
UPDATE forum_outbox SET attempts=1,delivery_state='retry_wait' WHERE post_id='40000000-0000-0000-0000-000000000041' AND sequence=2;
UPDATE school_source_bindings SET active=true,route_id='route-b',revision=revision+1 WHERE id='40000000-0000-0000-0000-000000000031';
UPDATE posts SET content='切换路由正文' WHERE id='40000000-0000-0000-0000-000000000041';
DO $$ BEGIN
 IF (SELECT count(*) FROM forum_outbox WHERE post_id='40000000-0000-0000-0000-000000000041' AND sequence=3)<>2 THEN RAISE EXCEPTION 'route change must withdraw old and upsert new'; END IF;
 IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE post_id='40000000-0000-0000-0000-000000000041' AND sequence=3 AND route_id='route-a' AND payload->>'event_type'='content.withdrawn') THEN RAISE EXCEPTION 'uncertain old route not withdrawn'; END IF;
 IF EXISTS(SELECT 1 FROM forum_outbox o JOIN original_frozen f USING(id) WHERE o.request_body<>f.request_body OR o.body_sha256<>f.body_sha256) THEN RAISE EXCEPTION 'frozen body changed'; END IF;
 BEGIN UPDATE forum_outbox SET request_body='tampered' WHERE id=(SELECT id FROM original_frozen); RAISE EXCEPTION 'mutable body'; EXCEPTION WHEN raise_exception THEN IF SQLERRM<>'frozen source identity is immutable' THEN RAISE; END IF; END;
END $$;
-- Interaction and non-projected fields do not create a new source opportunity.
CREATE TEMP TABLE counts ON COMMIT DROP AS SELECT count(*) AS total FROM forum_outbox;
UPDATE posts SET browse=browse+1,tags='["tag"]',images='["image"]' WHERE id='40000000-0000-0000-0000-000000000041';
DO $$ BEGIN IF (SELECT count(*) FROM forum_outbox)<>(SELECT total FROM counts) THEN RAISE EXCEPTION 'interaction caused push'; END IF; END $$;
-- Pause ordinary submissions, but still propagate school disable to all historical identities.
UPDATE school_source_bindings SET active=false,revision=revision+1 WHERE id='40000000-0000-0000-0000-000000000031';
UPDATE schools SET active=false WHERE id='40000000-0000-0000-0000-000000000001';
SELECT forum_reconcile_school('40000000-0000-0000-0000-000000000001',1);
SELECT forum_reconcile_school('40000000-0000-0000-0000-000000000001',1);
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM forum_source_route_states WHERE school_id='40000000-0000-0000-0000-000000000001' AND last_event_type='content.upsert') THEN RAISE EXCEPTION 'school disable left active historical route'; END IF;
 IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE route_id='route-b' AND payload->>'event_type'='content.withdrawn' AND delivery_state='ready') THEN RAISE EXCEPTION 'withdrawal was paused'; END IF;
END $$;
UPDATE counts SET total=(SELECT count(*) FROM forum_outbox);
SELECT forum_reconcile_school('40000000-0000-0000-0000-000000000001');
UPDATE schools SET active=true WHERE id='40000000-0000-0000-0000-000000000001';
DO $$ BEGIN IF (SELECT count(*) FROM forum_outbox)<>(SELECT total FROM counts) THEN RAISE EXCEPTION 'duplicate reconciliation or reenable replay'; END IF; END $$;
-- Return to route-a: sequence must advance above its previous withdrawal.
UPDATE school_source_bindings SET route_id='route-a',active=true,revision=revision+1 WHERE id='40000000-0000-0000-0000-000000000031';
UPDATE posts SET content='恢复后新正文' WHERE id='40000000-0000-0000-0000-000000000041';
DO $$ BEGIN
 IF NOT EXISTS(SELECT 1 FROM forum_source_route_states WHERE route_id='route-a' AND last_sequence>3 AND last_event_type='content.upsert') THEN RAISE EXCEPTION 'route return did not advance'; END IF;
END $$;
-- A delayed stop task must not withdraw a higher-version edit after reenable, even in the same transaction.
UPDATE schools SET active=false WHERE id='40000000-0000-0000-0000-000000000001';
UPDATE schools SET active=true WHERE id='40000000-0000-0000-0000-000000000001';
UPDATE posts SET content='停校任务延迟期间重新发布' WHERE id='40000000-0000-0000-0000-000000000041';
SELECT forum_reconcile_school('40000000-0000-0000-0000-000000000001');
DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM forum_source_route_states WHERE route_id='route-a' AND last_event_type='content.upsert') THEN RAISE EXCEPTION 'old stop task withdrew a newer publication'; END IF; END $$;
-- Missing current global config must not assign an old school's connection to a new event.
UPDATE source_active_configuration SET connection_id=NULL;
UPDATE posts SET content='新全局配置待验证' WHERE id='40000000-0000-0000-0000-000000000041';
DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE post_id='40000000-0000-0000-0000-000000000041' AND delivery_state='waiting_configuration' AND connection_version IS NULL) THEN RAISE EXCEPTION 'old global identity borrowed'; END IF; END $$;
-- Duplicate current route remains occupied while paused.
DO $$ BEGIN
 BEGIN INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id,active) VALUES(gen_random_uuid(),'40000000-0000-0000-0000-000000000002','40000000-0000-0000-0000-000000000011','40000000-0000-0000-0000-000000000021','40000000-0000-0000-0000-000000000101','https://dynamic.example.com','route-a',false); RAISE EXCEPTION 'duplicate route accepted'; EXCEPTION WHEN unique_violation THEN NULL; END;
END $$;
ROLLBACK;
