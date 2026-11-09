-- Current configuration is public metadata only. Secrets remain process environment material.
ALTER TABLE source_connection_versions ADD COLUMN key_id text;
UPDATE source_connection_versions v SET key_id=c.key_id FROM source_connections c WHERE c.id=v.connection_id;
ALTER TABLE source_connection_versions ALTER COLUMN key_id SET NOT NULL;
CREATE FUNCTION forum_connection_version_key() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.key_id IS NULL THEN SELECT key_id INTO NEW.key_id FROM source_connections WHERE id=NEW.connection_id; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_connection_version_key BEFORE INSERT ON source_connection_versions FOR EACH ROW EXECUTE FUNCTION forum_connection_version_key();
CREATE TABLE source_active_configuration (
 singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
 connection_id uuid REFERENCES source_connections,
 updated_at timestamptz NOT NULL DEFAULT now()
);
INSERT INTO source_active_configuration(singleton,connection_id)
VALUES(true,(SELECT id FROM source_connections WHERE checked_at IS NOT NULL ORDER BY checked_at DESC,id DESC LIMIT 1));

ALTER TABLE school_source_bindings ADD COLUMN checked_at timestamptz,
 ADD COLUMN updated_at timestamptz NOT NULL DEFAULT now();
CREATE TABLE school_source_binding_versions (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), binding_id uuid NOT NULL,
 school_id uuid NOT NULL REFERENCES schools, revision bigint NOT NULL,
 connection_version uuid NOT NULL REFERENCES source_connection_versions,
 source_id uuid NOT NULL, origin text NOT NULL, route_id text NOT NULL, active boolean NOT NULL,
 checked_at timestamptz, created_at timestamptz NOT NULL DEFAULT now(), actor_id uuid REFERENCES accounts,
 baseline boolean NOT NULL DEFAULT false, UNIQUE(binding_id,revision)
);
INSERT INTO school_source_binding_versions(binding_id,school_id,revision,connection_version,source_id,origin,route_id,active,baseline)
SELECT id,school_id,revision,connection_version,source_id,origin,route_id,active,true FROM school_source_bindings;

ALTER TABLE forum_source_versions ADD COLUMN delivery_intent boolean NOT NULL DEFAULT true;
ALTER TABLE forum_outbox ADD COLUMN binding_version uuid REFERENCES school_source_binding_versions,
 ADD COLUMN delivery_state text NOT NULL DEFAULT 'ready',
 ADD COLUMN frozen_at timestamptz, ADD COLUMN lease_token uuid, ADD COLUMN lease_until timestamptz,
 ADD COLUMN superseded_by uuid REFERENCES forum_outbox,
 ADD COLUMN recovery_reason text;
UPDATE forum_outbox SET delivery_state=CASE WHEN blocked_reason IS NOT NULL THEN 'blocked'
 WHEN receipt_status IN ('ignored_stale','rejected') THEN 'settled'
 WHEN receipt_id IS NOT NULL THEN 'tracking_receipt' ELSE 'ready' END,
 frozen_at=CASE WHEN request_body IS NOT NULL THEN created_at END;
UPDATE forum_outbox SET delivery_state='blocked',blocked_reason='snapshot_incomplete',blocked_at=now()
WHERE schema_version=2 AND (request_body IS NULL OR body_sha256 IS NULL OR connection_version IS NULL OR source_id IS NULL OR route_id IS NULL);
ALTER TABLE forum_outbox ADD CONSTRAINT forum_outbox_complete_snapshot CHECK(schema_version<>2 OR request_body IS NULL OR (connection_version IS NOT NULL AND source_id IS NOT NULL AND route_id IS NOT NULL AND body_sha256 IS NOT NULL));
ALTER TABLE forum_outbox ADD CONSTRAINT forum_outbox_delivery_state CHECK(delivery_state IN
 ('waiting_binding','waiting_configuration','paused','ready','in_flight','retry_wait','blocked','tracking_receipt','settled','superseded'));
ALTER TABLE forum_outbox ADD COLUMN source_origin text;
UPDATE forum_outbox o SET source_origin=v.origin FROM source_connection_versions v WHERE v.id=o.connection_version;
DROP INDEX forum_outbox_version_route;
CREATE UNIQUE INDEX forum_outbox_bound_version ON forum_outbox(school_id,post_id,sequence,source_origin,source_id,route_id);
CREATE UNIQUE INDEX forum_outbox_unbound_version ON forum_outbox(school_id,post_id,sequence) WHERE connection_version IS NULL;
CREATE INDEX forum_outbox_schedulable ON forum_outbox(next_attempt_at,id) WHERE schema_version=2 AND delivery_state IN ('ready','retry_wait','tracking_receipt','in_flight');
CREATE INDEX forum_outbox_school_state ON forum_outbox(school_id,delivery_state,created_at);

CREATE TABLE forum_source_route_states (
 school_id uuid NOT NULL REFERENCES schools, post_id uuid NOT NULL REFERENCES posts,
 origin text NOT NULL, source_id uuid NOT NULL, route_id text NOT NULL,
 connection_version uuid NOT NULL REFERENCES source_connection_versions,
 binding_version uuid REFERENCES school_source_binding_versions,
 last_sequence bigint NOT NULL, last_event_type text NOT NULL,
 last_event_id uuid NOT NULL REFERENCES forum_outbox, receipt_id text, receipt_status text,
 PRIMARY KEY(school_id,post_id,origin,source_id,route_id)
);
-- Include uncertain requests: an absent receipt does not prove that upstream never received it.
INSERT INTO forum_source_route_states(school_id,post_id,origin,source_id,route_id,connection_version,last_sequence,last_event_type,last_event_id,receipt_id,receipt_status)
SELECT DISTINCT ON(o.school_id,o.post_id,v.origin,o.source_id,o.route_id)
 o.school_id,o.post_id,v.origin,o.source_id,o.route_id,o.connection_version,o.sequence,o.payload->>'event_type',o.id,o.receipt_id,o.receipt_status
FROM forum_outbox o JOIN source_connection_versions v ON v.id=o.connection_version
WHERE o.schema_version=2 ORDER BY o.school_id,o.post_id,v.origin,o.source_id,o.route_id,o.sequence DESC,o.created_at DESC,o.id DESC;
CREATE TABLE source_reconciliation_jobs (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(), school_id uuid NOT NULL REFERENCES schools,
 completed_at timestamptz, created_at timestamptz NOT NULL DEFAULT now(), processed bigint NOT NULL DEFAULT 0
);
CREATE INDEX source_reconciliation_pending ON source_reconciliation_jobs(school_id,created_at,id) WHERE completed_at IS NULL;
CREATE TABLE source_reconciliation_targets (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),job_id uuid NOT NULL REFERENCES source_reconciliation_jobs,
 post_id uuid NOT NULL REFERENCES posts, connection_version uuid REFERENCES source_connection_versions,
 binding_version uuid REFERENCES school_source_binding_versions, origin text,source_id uuid,route_id text,
 sequence bigint NOT NULL, completed_at timestamptz
);
CREATE INDEX source_reconciliation_targets_pending ON source_reconciliation_targets(job_id,id) WHERE completed_at IS NULL;
CREATE TABLE source_import_batches (
 id uuid PRIMARY KEY REFERENCES migration_runs, delivery_intent boolean NOT NULL DEFAULT false CHECK(NOT delivery_intent)
);
CREATE TABLE source_imported_posts (
 post_id uuid PRIMARY KEY REFERENCES posts, batch_id uuid NOT NULL REFERENCES source_import_batches
);

DROP TRIGGER forum_v2_snapshot ON forum_outbox;
CREATE FUNCTION forum_binding_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO school_source_binding_versions(binding_id,school_id,revision,connection_version,source_id,origin,route_id,active,checked_at,actor_id)
 VALUES(NEW.id,NEW.school_id,NEW.revision,NEW.connection_version,NEW.source_id,NEW.origin,NEW.route_id,NEW.active,NEW.checked_at,nullif(current_setting('linjian.source_actor_id',true),'')::uuid);
 UPDATE forum_outbox SET delivery_state=CASE WHEN NEW.active THEN CASE WHEN last_error IS NULL THEN 'ready' ELSE 'retry_wait' END ELSE 'paused' END
 WHERE school_id=NEW.school_id AND payload->>'event_type'='content.upsert' AND receipt_id IS NULL AND lease_token IS NULL AND delivery_state IN ('ready','retry_wait','paused');
 RETURN NEW;
END $$;
CREATE TRIGGER forum_binding_history AFTER INSERT OR UPDATE ON school_source_bindings FOR EACH ROW EXECUTE FUNCTION forum_binding_history();

-- One serializer for immediate and deferred binding. Assigning a deferred binding retains its event ID.
CREATE FUNCTION forum_freeze_source(event uuid, cv uuid, bv uuid, route text) RETURNS void LANGUAGE plpgsql AS $$
DECLARE o forum_outbox; v source_connection_versions; p jsonb; body text;
BEGIN
 SELECT * INTO STRICT o FROM forum_outbox WHERE id=event FOR UPDATE;
 IF o.request_body IS NOT NULL THEN RETURN; END IF;
 SELECT * INTO STRICT v FROM source_connection_versions WHERE id=cv;
 IF bv IS NOT NULL AND NOT EXISTS(SELECT 1 FROM school_source_binding_versions WHERE id=bv AND school_id=o.school_id AND connection_version=cv AND route_id=route) THEN
 RAISE EXCEPTION 'binding snapshot does not belong to this event';
 END IF;
 p := (o.payload - 'external_organization_id') || jsonb_build_object('schema_version',2,'route_id',route,
 'detail_url',v.public_origin || '/community/' || o.school_id || '/posts/' || o.post_id);
 body := jsonb_build_object('data',jsonb_build_object('type','source-events','id',event,'attributes',p))::text;
 UPDATE forum_outbox SET connection_version=cv,binding_version=bv,source_id=v.source_id,source_origin=v.origin,route_id=route,payload=p,
 request_body=body,body_sha256=encode(sha256(convert_to(body,'UTF8')),'hex'),frozen_at=now(),delivery_state='ready' WHERE id=event;
 INSERT INTO forum_source_route_states(school_id,post_id,origin,source_id,route_id,connection_version,binding_version,last_sequence,last_event_type,last_event_id)
 VALUES(o.school_id,o.post_id,v.origin,v.source_id,route,cv,bv,o.sequence,p->>'event_type',event)
 ON CONFLICT(school_id,post_id,origin,source_id,route_id) DO UPDATE SET
 connection_version=excluded.connection_version,binding_version=excluded.binding_version,last_sequence=excluded.last_sequence,
 last_event_type=excluded.last_event_type,last_event_id=excluded.last_event_id,receipt_id=NULL,receipt_status=NULL
 WHERE forum_source_route_states.last_sequence<excluded.last_sequence;
END $$;
CREATE FUNCTION forum_source_event(item posts, seq bigint, kind text, cv uuid, bv uuid, route text) RETURNS uuid LANGUAGE plpgsql AS $$
DECLARE event uuid := gen_random_uuid();
BEGIN
 INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload,schema_version,signature_version,delivery_state)
 VALUES(event,item.school_id,item.id,seq,jsonb_build_object('event_id',event,'schema_version',2,
 'event_type',kind,'external_content_id',item.id,'version',seq::text,'sequence',seq,'occurred_at',now(),
 'title',CASE WHEN kind='content.upsert' THEN item.title END,'content',CASE WHEN kind='content.upsert' THEN item.content END),2,2,'waiting_binding');
 IF cv IS NOT NULL THEN PERFORM forum_freeze_source(event,cv,bv,route); END IF;
 RETURN event;
END $$;
CREATE OR REPLACE FUNCTION forum_enqueue_material(item posts, force_version boolean) RETURNS void LANGUAGE plpgsql AS $$
DECLARE eligible boolean; was_available boolean; seq bigint; b school_source_bindings; bv uuid; r record; target boolean; event uuid;
BEGIN
 -- Shared lock serializes configuration publication and content snapshots, without locking other posts.
 PERFORM id FROM schools WHERE id=item.school_id FOR UPDATE;
 eligible := item.audit_status='approved' AND item.visibility='visible' AND (SELECT active FROM schools WHERE id=item.school_id);
 SELECT available INTO was_available FROM forum_source_versions WHERE school_id=item.school_id AND post_id=item.id;
 IF NOT eligible AND NOT coalesce(was_available,false) THEN RETURN; END IF;
 IF NOT force_version AND eligible IS NOT DISTINCT FROM was_available THEN RETURN; END IF;
 INSERT INTO forum_source_versions(school_id,post_id,sequence,available) VALUES(item.school_id,item.id,1,eligible)
 ON CONFLICT(school_id,post_id) DO UPDATE SET sequence=forum_source_versions.sequence+1,available=excluded.available,delivery_intent=true
 RETURNING sequence INTO seq;
 IF seq>9007199254740991 THEN RAISE EXCEPTION 'source sequence exhausted'; END IF;
 SELECT sb.* INTO b FROM school_source_bindings sb JOIN source_active_configuration ac ON ac.connection_id=sb.connection_id WHERE sb.school_id=item.school_id;
 target := b.id IS NOT NULL;
 IF target THEN SELECT id INTO bv FROM school_source_binding_versions WHERE binding_id=b.id AND revision=b.revision; END IF;
 FOR r IN SELECT * FROM forum_source_route_states WHERE school_id=item.school_id AND post_id=item.id AND last_event_type='content.upsert' ORDER BY origin,source_id,route_id FOR UPDATE LOOP
  IF NOT eligible OR NOT target OR (r.origin,r.source_id,r.route_id) IS DISTINCT FROM (b.origin,b.source_id,b.route_id) THEN
   event := forum_source_event(item,seq,'content.withdrawn',r.connection_version,r.binding_version,r.route_id);
  END IF;
 END LOOP;
 IF eligible THEN
  event := forum_source_event(item,seq,'content.upsert',b.connection_version,bv,b.route_id);
 ELSE
  -- An unbound update can otherwise resurrect unavailable content when binding is eventually configured.
  IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE school_id=item.school_id AND post_id=item.id AND sequence=seq) THEN
   event := forum_source_event(item,seq,'content.withdrawn',NULL,NULL,NULL);
  END IF;
  UPDATE forum_outbox SET delivery_state='superseded',superseded_by=CASE WHEN id<>event THEN event END WHERE school_id=item.school_id AND post_id=item.id AND connection_version IS NULL AND delivery_state IN ('waiting_binding','waiting_configuration');
 END IF;
 -- Preserve evidence but suppress never-attempted outdated upserts, including paused bound events.
 UPDATE forum_outbox SET delivery_state='superseded',superseded_by=event WHERE school_id=item.school_id AND post_id=item.id
 AND sequence<seq AND payload->>'event_type'='content.upsert' AND attempts=0 AND lease_token IS NULL AND receipt_id IS NULL AND delivery_state NOT IN ('settled','superseded');
 IF eligible AND target AND NOT b.active THEN UPDATE forum_outbox SET delivery_state='paused' WHERE id=event; END IF;
 IF eligible AND NOT target AND EXISTS(SELECT 1 FROM school_source_bindings WHERE school_id=item.school_id) THEN
 UPDATE forum_outbox SET delivery_state='waiting_configuration' WHERE id=event; END IF;
END $$;
CREATE OR REPLACE FUNCTION forum_enqueue_post() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 -- images/tags are not part of the current wire projection; interactions never create source opportunities.
 IF TG_OP='UPDATE' AND (NEW.title,NEW.content,NEW.audit_status,NEW.visibility) IS NOT DISTINCT FROM (OLD.title,OLD.content,OLD.audit_status,OLD.visibility) THEN RETURN NEW; END IF;
 PERFORM forum_enqueue_material(NEW,true);
 RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION forum_enqueue_school() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE job uuid;
BEGIN
 IF OLD.active AND NOT NEW.active THEN
 INSERT INTO source_reconciliation_jobs(school_id) VALUES(NEW.id) RETURNING id INTO job;
 -- Capture sequence cutoffs while the school row is locked, not transaction timestamps.
 -- A subsequent reenable/edit may start in an older transaction and still be a newer content version.
 INSERT INTO source_reconciliation_targets(job_id,post_id,connection_version,binding_version,origin,source_id,route_id,sequence)
 SELECT job,post_id,connection_version,binding_version,origin,source_id,route_id,last_sequence FROM forum_source_route_states
 WHERE school_id=NEW.id AND last_event_type='content.upsert';
 INSERT INTO source_reconciliation_targets(job_id,post_id,sequence)
 SELECT job,post_id,max(sequence) FROM forum_outbox WHERE school_id=NEW.id AND connection_version IS NULL AND delivery_state IN ('waiting_binding','waiting_configuration') GROUP BY post_id;
 END IF;
 RETURN NEW;
END $$;
-- Re-enabling never replays history. Every target has an exact content-version cutoff.
CREATE FUNCTION forum_reconcile_school(school uuid, batch_size integer DEFAULT 50) RETURNS integer LANGUAGE plpgsql AS $$
DECLARE r record; item posts; seq bigint; count integer := 0; job uuid; current_seq bigint; current_kind text;
BEGIN
 PERFORM id FROM schools WHERE id=school FOR UPDATE;
 SELECT id INTO job FROM source_reconciliation_jobs WHERE school_id=school AND completed_at IS NULL ORDER BY created_at,id LIMIT 1 FOR UPDATE;
 IF job IS NULL THEN RETURN 0; END IF;
 FOR r IN SELECT * FROM source_reconciliation_targets WHERE job_id=job AND completed_at IS NULL ORDER BY id LIMIT batch_size FOR UPDATE LOOP
  SELECT * INTO STRICT item FROM posts WHERE id=r.post_id;
  IF r.connection_version IS NOT NULL THEN
   SELECT last_sequence,last_event_type INTO current_seq,current_kind FROM forum_source_route_states WHERE school_id=school AND post_id=r.post_id AND origin=r.origin AND source_id=r.source_id AND route_id=r.route_id FOR UPDATE;
   IF current_seq<=r.sequence AND current_kind='content.upsert' THEN
    UPDATE forum_source_versions SET sequence=sequence+1,available=CASE WHEN (SELECT active FROM schools WHERE id=school) THEN available ELSE false END WHERE school_id=school AND post_id=r.post_id RETURNING sequence INTO seq;
    PERFORM forum_source_event(item,seq,'content.withdrawn',r.connection_version,r.binding_version,r.route_id);
   END IF;
  ELSE
   SELECT sequence INTO current_seq FROM forum_source_versions WHERE school_id=school AND post_id=r.post_id;
   IF current_seq<=r.sequence THEN
    UPDATE forum_source_versions SET sequence=sequence+1,available=false WHERE school_id=school AND post_id=r.post_id RETURNING sequence INTO seq;
    PERFORM forum_source_event(item,seq,'content.withdrawn',NULL,NULL,NULL);
    UPDATE forum_outbox SET delivery_state='superseded' WHERE post_id=r.post_id AND connection_version IS NULL AND sequence<=seq AND delivery_state IN ('waiting_binding','waiting_configuration');
   END IF;
  END IF;
  UPDATE forum_outbox SET delivery_state='superseded' WHERE school_id=school AND post_id=r.post_id AND sequence<=r.sequence AND attempts=0 AND lease_token IS NULL AND receipt_id IS NULL AND payload->>'event_type'='content.upsert' AND delivery_state NOT IN ('settled','superseded');
  UPDATE source_reconciliation_targets SET completed_at=now() WHERE id=r.id;
  count := count+1;
 END LOOP;
 IF NOT EXISTS(SELECT 1 FROM source_reconciliation_targets WHERE job_id=job AND completed_at IS NULL) THEN UPDATE source_reconciliation_jobs SET completed_at=now() WHERE id=job; END IF;
 UPDATE source_reconciliation_jobs SET processed=processed+count WHERE id=job;
 RETURN count;
END $$;

CREATE FUNCTION forum_guard_frozen_source() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.request_body IS NOT NULL AND (NEW.id,NEW.school_id,NEW.post_id,NEW.sequence,NEW.payload,NEW.connection_version,NEW.binding_version,NEW.source_id,NEW.source_origin,NEW.route_id,NEW.schema_version,NEW.signature_version,NEW.request_body,NEW.body_sha256,NEW.frozen_at)
 IS DISTINCT FROM (OLD.id,OLD.school_id,OLD.post_id,OLD.sequence,OLD.payload,OLD.connection_version,OLD.binding_version,OLD.source_id,OLD.source_origin,OLD.route_id,OLD.schema_version,OLD.signature_version,OLD.request_body,OLD.body_sha256,OLD.frozen_at) THEN
 RAISE EXCEPTION 'frozen source identity is immutable';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_guard_frozen_source BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_frozen_source();
CREATE FUNCTION forum_guard_source_version() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'historical source versions are immutable'; END $$;
CREATE TRIGGER forum_guard_source_version BEFORE UPDATE OR DELETE ON source_connection_versions FOR EACH ROW EXECUTE FUNCTION forum_guard_source_version();
CREATE TRIGGER forum_guard_binding_version BEFORE UPDATE OR DELETE ON school_source_binding_versions FOR EACH ROW EXECUTE FUNCTION forum_guard_source_version();

INSERT INTO source_reconciliation_jobs(school_id) SELECT s.id FROM schools s WHERE NOT s.active AND EXISTS(SELECT 1 FROM forum_source_route_states r WHERE r.school_id=s.id AND r.last_event_type='content.upsert');
INSERT INTO source_reconciliation_targets(job_id,post_id,connection_version,binding_version,origin,source_id,route_id,sequence)
SELECT j.id,r.post_id,r.connection_version,r.binding_version,r.origin,r.source_id,r.route_id,r.last_sequence
FROM source_reconciliation_jobs j JOIN forum_source_route_states r ON r.school_id=j.school_id WHERE r.last_event_type='content.upsert';
