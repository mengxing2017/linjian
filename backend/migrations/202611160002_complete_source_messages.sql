-- Single source-message protocol. No historical request conversion or compatibility mode.
ALTER TABLE posts DROP CONSTRAINT posts_source_projection_check;
ALTER TABLE posts ALTER COLUMN source_projection SET DEFAULT 'message_v1';
ALTER TABLE posts ADD CONSTRAINT posts_source_projection_check CHECK(source_projection='message_v1');
ALTER TABLE forum_outbox ALTER COLUMN projection_version SET DEFAULT 'message_v1';
ALTER TABLE forum_outbox ADD CONSTRAINT forum_message_projection CHECK(projection_version='message_v1');
ALTER TABLE forum_outbox ADD COLUMN message_version text;
ALTER TABLE forum_outbox DROP COLUMN superseded_by;
ALTER TABLE forum_outbox DROP CONSTRAINT forum_outbox_delivery_state;
ALTER TABLE forum_outbox ADD CONSTRAINT forum_outbox_delivery_state CHECK(delivery_state IN
 ('waiting_binding','waiting_configuration','paused','preparing_content','ready','in_flight','retry_wait','blocked','accepted','cancelled'));
ALTER TABLE forum_outbox DROP CONSTRAINT forum_prepared_before_delivery;
ALTER TABLE forum_outbox ADD CONSTRAINT forum_prepared_before_delivery CHECK
 (delivery_state NOT IN ('ready','in_flight','retry_wait','accepted') OR request_body IS NOT NULL);
ALTER TABLE forum_outbox ADD CONSTRAINT forum_received_receipt CHECK(receipt_status IS NULL OR receipt_status='received');
DROP INDEX forum_outbox_schedulable;
CREATE INDEX forum_outbox_schedulable ON forum_outbox(next_attempt_at,id) WHERE delivery_state IN ('ready','retry_wait','in_flight');
CREATE OR REPLACE FUNCTION forum_guard_post_publication() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' THEN
  IF (NEW.created_via,NEW.source_projection) IS DISTINCT FROM (OLD.created_via,OLD.source_projection) THEN RAISE EXCEPTION 'post provenance is immutable'; END IF;
  IF (NEW.title,NEW.content,NEW.images,NEW.tags) IS DISTINCT FROM (OLD.title,OLD.content,OLD.images,OLD.tags) THEN
   NEW.content_revision:=OLD.content_revision+1;
   NEW.approved_snapshot_id:=NULL;
   IF NEW.source_projection='message_v1' THEN NEW.audit_status:='pending'; END IF;
  ELSE NEW.content_revision:=OLD.content_revision; END IF;
 END IF;
 IF NEW.audit_status<>'approved' THEN NEW.approved_snapshot_id:=NULL; END IF;
 IF NEW.source_projection='message_v1' AND NEW.audit_status='approved' AND NOT EXISTS(
  SELECT 1 FROM forum_publication_snapshots p JOIN moderation_events m ON m.id=p.moderation_event_id
  WHERE p.id=NEW.approved_snapshot_id AND p.school_id=NEW.school_id AND p.post_id=NEW.id
   AND p.content_revision=NEW.content_revision AND p.title=NEW.title AND p.content=NEW.content AND p.images=NEW.images AND p.tags=NEW.tags
   AND p.approved_post_revision<=NEW.revision
   AND (p.approved_post_revision=NEW.revision OR (TG_OP='UPDATE' AND OLD.approved_snapshot_id=NEW.approved_snapshot_id))
   AND m.school_id=NEW.school_id AND m.object_id=NEW.id AND m.object_kind='post' AND m.decision='approved'
   AND p.decision_source=m.decision_source
   AND ((p.decision_source='legacy' AND m.actor_id=p.moderator_id)
     OR (p.decision_source='human' AND m.actor_id=p.moderator_id AND m.content_revision=NEW.content_revision AND m.review_digest IS NOT NULL)
     OR (p.decision_source='automatic' AND m.actor_id IS NULL AND p.moderator_id IS NULL
       AND m.content_revision=NEW.content_revision AND m.policy_revision=p.policy_revision AND m.policy_mode='auto'))
 ) THEN RAISE EXCEPTION 'approved post requires matching publication evidence'; END IF;
 RETURN NEW;
END $$;


CREATE OR REPLACE FUNCTION forum_freeze_source(event uuid, cv uuid, bv uuid, route text) RETURNS void LANGUAGE plpgsql AS $$
BEGIN PERFORM forum_bind_prepared_source(event,cv,bv,route); END $$;

CREATE OR REPLACE FUNCTION forum_bind_prepared_source(event uuid, cv uuid, bv uuid, route text) RETURNS void LANGUAGE plpgsql AS $$
DECLARE o forum_outbox; v source_connection_versions;
BEGIN
 SELECT * INTO STRICT o FROM forum_outbox WHERE id=event FOR UPDATE;
 IF o.connection_version IS NOT NULL THEN
  IF (o.connection_version,o.binding_version,o.route_id) IS DISTINCT FROM (cv,bv,route) THEN RAISE EXCEPTION 'source binding is immutable'; END IF;
  RETURN;
 END IF;
 SELECT * INTO STRICT v FROM source_connection_versions WHERE id=cv;
 IF NOT EXISTS(SELECT 1 FROM school_source_binding_versions WHERE id=bv AND school_id=o.school_id AND connection_version=cv AND route_id=route) THEN RAISE EXCEPTION 'invalid school binding'; END IF;
 UPDATE forum_outbox SET connection_version=cv,binding_version=bv,source_id=v.source_id,source_origin=v.origin,route_id=route,
 delivery_state='preparing_content' WHERE id=event;
END $$;

CREATE OR REPLACE FUNCTION forum_source_event(item posts, seq bigint, kind text, cv uuid, bv uuid, route text) RETURNS uuid LANGUAGE plpgsql AS $$
DECLARE event uuid:=gen_random_uuid();
BEGIN
 IF kind<>'content.upsert' THEN RAISE EXCEPTION 'unsupported source event'; END IF;
 INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload,schema_version,signature_version,delivery_state,publication_snapshot_id,projection_version)
 VALUES(event,item.school_id,item.id,seq,jsonb_build_object('event_id',event,'schema_version',2,'event_type','content.upsert','occurred_at',now()),2,2,'waiting_binding',item.approved_snapshot_id,'message_v1');
 INSERT INTO forum_publication_preparations(event_id,snapshot_id) VALUES(event,item.approved_snapshot_id);
 IF cv IS NOT NULL THEN PERFORM forum_bind_prepared_source(event,cv,bv,route); END IF;
 RETURN event;
END $$;

CREATE OR REPLACE FUNCTION forum_enqueue_material(item posts, force_version boolean) RETURNS void LANGUAGE plpgsql AS $$
DECLARE eligible boolean; seq bigint; b school_source_bindings; bv uuid;
BEGIN
 PERFORM id FROM schools WHERE id=item.school_id FOR UPDATE;
 eligible:=item.audit_status='approved' AND item.visibility='visible' AND EXISTS(SELECT 1 FROM schools WHERE id=item.school_id AND active);
 -- Claims use the same school lock. Only requests never attempted can be cancelled.
 UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=item.school_id AND post_id=item.id
 AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
 IF NOT eligible THEN
  UPDATE forum_source_versions SET available=false WHERE school_id=item.school_id AND post_id=item.id;
  RETURN;
 END IF;
 INSERT INTO forum_source_versions(school_id,post_id,sequence,available) VALUES(item.school_id,item.id,1,true)
 ON CONFLICT(school_id,post_id) DO UPDATE SET sequence=forum_source_versions.sequence+1,available=true,delivery_intent=true RETURNING sequence INTO seq;
 SELECT sb.* INTO b FROM school_source_bindings sb JOIN source_active_configuration ac ON ac.connection_id=sb.connection_id WHERE sb.school_id=item.school_id;
 IF b.id IS NOT NULL THEN SELECT id INTO bv FROM school_source_binding_versions WHERE binding_id=b.id AND revision=b.revision; END IF;
 PERFORM forum_source_event(item,seq,'content.upsert',b.connection_version,bv,b.route_id);
 IF b.id IS NOT NULL AND NOT b.active THEN UPDATE forum_outbox SET delivery_state='paused' WHERE school_id=item.school_id AND post_id=item.id AND sequence=seq; END IF;
END $$;

CREATE OR REPLACE FUNCTION forum_enqueue_school() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.active AND NOT NEW.active THEN
  UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=NEW.id AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
  UPDATE forum_source_versions SET available=false WHERE school_id=NEW.id;
 END IF;
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION forum_binding_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO school_source_binding_versions(binding_id,school_id,revision,connection_version,source_id,origin,route_id,active,checked_at,actor_id)
 VALUES(NEW.id,NEW.school_id,NEW.revision,NEW.connection_version,NEW.source_id,NEW.origin,NEW.route_id,NEW.active,NEW.checked_at,nullif(current_setting('linjian.source_actor_id',true),'')::uuid);
 IF TG_OP='UPDATE' AND (OLD.origin,OLD.source_id,OLD.route_id) IS DISTINCT FROM (NEW.origin,NEW.source_id,NEW.route_id) THEN
  UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=NEW.school_id AND connection_version IS NOT NULL AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
 END IF;
 UPDATE forum_outbox SET delivery_state=CASE WHEN NOT NEW.active THEN 'paused' WHEN request_body IS NULL THEN 'preparing_content' WHEN last_error IS NULL THEN 'ready' ELSE 'retry_wait' END
 WHERE school_id=NEW.school_id AND connection_version IS NOT NULL AND receipt_id IS NULL AND attempts=0 AND lease_token IS NULL AND delivery_state IN ('ready','retry_wait','paused','preparing_content');
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION forum_invalidate_preparation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.delivery_state='cancelled' THEN
  UPDATE forum_publication_preparations SET state='superseded',revision=revision+1,lease_token=NULL,lease_until=NULL,last_error_code='cancelled'
  WHERE event_id=NEW.id AND state NOT IN ('completed','superseded');
 END IF;
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION forum_guard_bound_preparation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (NEW.projection_version,NEW.publication_snapshot_id) IS DISTINCT FROM (OLD.projection_version,OLD.publication_snapshot_id) THEN RAISE EXCEPTION 'publication intent is immutable'; END IF;
 IF OLD.connection_version IS NOT NULL AND (NEW.connection_version,NEW.binding_version,NEW.source_id,NEW.source_origin,NEW.route_id) IS DISTINCT FROM (OLD.connection_version,OLD.binding_version,OLD.source_id,OLD.source_origin,OLD.route_id) THEN RAISE EXCEPTION 'publication binding is immutable'; END IF;
 RETURN NEW;
END $$;

CREATE OR REPLACE FUNCTION forum_publication_is_current(event uuid) RETURNS boolean LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM forum_outbox o JOIN forum_publication_snapshots snap ON snap.id=o.publication_snapshot_id
 JOIN posts p ON p.id=o.post_id AND p.school_id=o.school_id JOIN schools s ON s.id=o.school_id
 WHERE o.id=event AND o.delivery_state<>'cancelled' AND s.active AND p.audit_status='approved' AND p.visibility='visible'
 AND p.approved_snapshot_id=snap.id AND p.content_revision=snap.content_revision)
$$;

-- The application provides the already constructed, serialized and signed-protocol-ready bytes.
DROP FUNCTION forum_finalize_prepared_source(uuid,uuid,uuid);
CREATE FUNCTION forum_finalize_prepared_source(event uuid, lease uuid, asset uuid, body text, version text) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE o forum_outbox; prep forum_publication_preparations; a miniapp_link_assets; attrs jsonb;
BEGIN
 SELECT * INTO STRICT o FROM forum_outbox WHERE id=event FOR UPDATE;
 SELECT * INTO STRICT prep FROM forum_publication_preparations WHERE event_id=event FOR UPDATE;
 IF prep.state<>'generating' OR prep.lease_token IS DISTINCT FROM lease OR prep.lease_until<=now() OR o.request_body IS NOT NULL THEN RETURN false; END IF;
 IF NOT forum_publication_is_current(event) THEN UPDATE forum_outbox SET delivery_state='cancelled' WHERE id=event; RETURN false; END IF;
 IF NOT EXISTS(SELECT 1 FROM school_source_bindings b WHERE b.school_id=o.school_id AND b.active) THEN
  UPDATE forum_publication_preparations SET state='pending',lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event; RETURN false;
 END IF;
 IF asset IS NOT NULL THEN
  SELECT * INTO STRICT a FROM miniapp_link_assets WHERE id=asset;
  IF a.app_id IS DISTINCT FROM prep.resolved_policy->>'app_id' OR a.policy IS DISTINCT FROM prep.resolved_policy
  OR a.page_path<>'pages/index/detail' OR a.canonical_query<>('school='||o.school_id||'&id='||o.post_id) THEN RAISE EXCEPTION 'link does not belong to this publication'; END IF;
  IF a.expires_at IS NOT NULL AND a.expires_at<=now()+make_interval(secs=>(prep.resolved_policy->>'min_remaining')::integer) THEN
   UPDATE forum_publication_preparations SET state='retry_wait',next_attempt_at=now()+interval '5 seconds',last_error_code='link_near_expiry',lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event; RETURN false;
  END IF;
 ELSIF EXISTS(SELECT 1 FROM posts WHERE id=o.post_id AND created_via='miniapp') THEN RAISE EXCEPTION 'miniapp publication requires a link';
 END IF;
 attrs:=body::jsonb->'data'->'attributes';
 IF body::jsonb->'data'->>'type' IS DISTINCT FROM 'source-events'
 OR body::jsonb->'data'->>'id' IS DISTINCT FROM event::text
 OR attrs->>'event_id' IS DISTINCT FROM event::text OR attrs->>'route_id' IS DISTINCT FROM o.route_id
 OR attrs->'schema_version' IS DISTINCT FROM '2'::jsonb
 OR (attrs->>'occurred_at')::timestamptz IS DISTINCT FROM (o.payload->>'occurred_at')::timestamptz
 OR attrs->>'event_type' IS DISTINCT FROM 'content.upsert'
 OR jsonb_typeof(attrs->'content') IS DISTINCT FROM 'string'
 OR coalesce(length(btrim(attrs->>'content')),0)=0 OR length(attrs->>'content')>200
 OR (SELECT array_agg(k ORDER BY k) FROM jsonb_object_keys(attrs) k) IS DISTINCT FROM ARRAY['content','event_id','event_type','occurred_at','route_id','schema_version']
 OR version IS DISTINCT FROM 'summary-20-v1' THEN RAISE EXCEPTION 'invalid frozen message'; END IF;
 UPDATE forum_outbox SET payload=attrs,request_body=body,body_sha256=encode(sha256(convert_to(body,'UTF8')),'hex'),message_version=version,frozen_at=now(),delivery_state='ready' WHERE id=event;
 UPDATE forum_publication_preparations SET state='completed',link_asset_id=asset,completed_at=now(),last_error_code=NULL,lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event;
 RETURN true;
END $$;

DROP FUNCTION forum_reconcile_school(uuid,integer);
DROP FUNCTION forum_freeze_legacy_source(uuid,uuid,uuid,text);
DROP FUNCTION forum_legacy_source_event(posts,bigint,text,uuid,uuid,text);
DROP FUNCTION forum_legacy_content_payload(jsonb,text,uuid,uuid);
DROP TABLE source_reconciliation_targets;
DROP TABLE source_reconciliation_jobs;
DROP TABLE forum_source_route_states;
DROP TABLE forum_publication_configuration;

CREATE FUNCTION forum_guard_message_version() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.request_body IS NOT NULL AND NEW.message_version IS DISTINCT FROM OLD.message_version THEN RAISE EXCEPTION 'frozen message version is immutable'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_guard_message_version BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_message_version();

-- Current snapshot export/import restores posts and their evidence in one deferred-FK transaction.
DO $$ DECLARE constraint_name text;
BEGIN
 SELECT conname INTO constraint_name FROM pg_constraint WHERE conrelid='posts'::regclass AND confrelid='forum_publication_snapshots'::regclass;
 EXECUTE format('ALTER TABLE posts ALTER CONSTRAINT %I DEFERRABLE INITIALLY DEFERRED',constraint_name);
END $$;
