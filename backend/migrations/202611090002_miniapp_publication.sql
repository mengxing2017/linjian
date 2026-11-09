-- Reviewed miniapp publications: durable intent before external link preparation.
ALTER TABLE posts ADD COLUMN created_via text NOT NULL DEFAULT 'legacy' CHECK(created_via IN ('legacy','web','miniapp')),
 ADD COLUMN content_revision bigint NOT NULL DEFAULT 1 CHECK(content_revision>0),
 ADD COLUMN approved_snapshot_id uuid,
 ADD COLUMN source_projection text NOT NULL DEFAULT 'legacy_full_v2' CHECK(source_projection IN ('legacy_full_v2','miniapp_summary_v1'));
CREATE TABLE forum_publication_snapshots (
 id uuid PRIMARY KEY, school_id uuid NOT NULL, post_id uuid NOT NULL, author_id uuid NOT NULL REFERENCES accounts,
 content_revision bigint NOT NULL, approved_post_revision bigint NOT NULL,
 moderation_event_id uuid NOT NULL REFERENCES moderation_events, moderator_id uuid NOT NULL REFERENCES accounts,
 approved_at timestamptz NOT NULL DEFAULT now(), title text NOT NULL, content text NOT NULL,
 images jsonb NOT NULL, tags jsonb NOT NULL, content_format text NOT NULL DEFAULT 'plain' CHECK(content_format='plain'),
 content_digest text NOT NULL, summary text NOT NULL, summary_algorithm text NOT NULL CHECK(summary_algorithm='plain-v1'),
 preview_digest text NOT NULL, link_policy jsonb NOT NULL,
 FOREIGN KEY(school_id,post_id) REFERENCES posts(school_id,id), UNIQUE(post_id,approved_post_revision), UNIQUE(school_id,post_id,id)
);
ALTER TABLE posts ADD FOREIGN KEY(school_id,id,approved_snapshot_id) REFERENCES forum_publication_snapshots(school_id,post_id,id);
CREATE TABLE miniapp_link_assets (
 id uuid PRIMARY KEY, cache_key text NOT NULL, app_id text NOT NULL, policy jsonb NOT NULL,
 page_path text NOT NULL, canonical_query text NOT NULL, page_title text NOT NULL,
 link_type text NOT NULL CHECK(link_type IN ('short_link','url_link')), permanent boolean NOT NULL,
 link text NOT NULL CHECK(length(link) BETWEEN 1 AND 4096), generated_at timestamptz NOT NULL,
 expires_at timestamptz, expiry_basis text NOT NULL DEFAULT 'provider_policy',
 CHECK(permanent=(expires_at IS NULL))
);
CREATE INDEX miniapp_link_assets_cache ON miniapp_link_assets(cache_key,generated_at DESC);
CREATE TABLE miniapp_link_leases (cache_key text PRIMARY KEY, lease_token uuid NOT NULL, lease_until timestamptz NOT NULL);
CREATE TABLE miniapp_token_cache (
 credential_scope text PRIMARY KEY, token text, expires_at timestamptz, generation uuid,
 lease_token uuid, lease_until timestamptz
);
ALTER TABLE forum_outbox ADD COLUMN publication_snapshot_id uuid REFERENCES forum_publication_snapshots,
 ADD COLUMN projection_version text NOT NULL DEFAULT 'legacy_full_v2';
ALTER TABLE forum_outbox DROP CONSTRAINT forum_outbox_delivery_state;
ALTER TABLE forum_outbox ADD CONSTRAINT forum_outbox_delivery_state CHECK(delivery_state IN
 ('waiting_binding','waiting_configuration','paused','preparing_content','ready','in_flight','retry_wait','blocked','tracking_receipt','settled','superseded'));
ALTER TABLE forum_outbox ADD CONSTRAINT forum_prepared_before_delivery CHECK
 (projection_version<>'miniapp_summary_v1' OR payload->>'event_type'<>'content.upsert' OR delivery_state NOT IN ('ready','in_flight','retry_wait','tracking_receipt','settled') OR request_body IS NOT NULL);
CREATE TABLE forum_publication_preparations (
 event_id uuid PRIMARY KEY REFERENCES forum_outbox, snapshot_id uuid NOT NULL REFERENCES forum_publication_snapshots,
 state text NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','generating','retry_wait','blocked','completed','superseded')),
 revision bigint NOT NULL DEFAULT 1, attempts integer NOT NULL DEFAULT 0,
 next_attempt_at timestamptz NOT NULL DEFAULT now(), last_error_code text,
 lease_token uuid, lease_until timestamptz, link_asset_id uuid REFERENCES miniapp_link_assets,
 created_at timestamptz NOT NULL DEFAULT now(), completed_at timestamptz
);
CREATE INDEX forum_preparation_due ON forum_publication_preparations(next_attempt_at,event_id) WHERE state IN ('pending','retry_wait','generating');
CREATE FUNCTION forum_guard_publication_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'publication evidence is immutable'; END $$;
CREATE TRIGGER forum_snapshot_immutable BEFORE UPDATE OR DELETE ON forum_publication_snapshots FOR EACH ROW EXECUTE FUNCTION forum_guard_publication_snapshot();
CREATE TRIGGER miniapp_link_immutable BEFORE UPDATE OR DELETE ON miniapp_link_assets FOR EACH ROW EXECUTE FUNCTION forum_guard_publication_snapshot();
CREATE FUNCTION forum_guard_post_publication() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' THEN
  IF (NEW.created_via,NEW.source_projection) IS DISTINCT FROM (OLD.created_via,OLD.source_projection) THEN RAISE EXCEPTION 'post provenance is immutable'; END IF;
  IF (NEW.title,NEW.content,NEW.images,NEW.tags) IS DISTINCT FROM (OLD.title,OLD.content,OLD.images,OLD.tags) THEN
   NEW.content_revision:=OLD.content_revision+1;
   NEW.approved_snapshot_id:=NULL;
   IF NEW.source_projection='miniapp_summary_v1' THEN NEW.audit_status:='pending'; END IF;
  ELSE NEW.content_revision:=OLD.content_revision; END IF;
 END IF;
 IF NEW.audit_status<>'approved' THEN NEW.approved_snapshot_id:=NULL; END IF;
 IF NEW.source_projection='miniapp_summary_v1' AND NEW.audit_status='approved' AND NOT EXISTS(
  SELECT 1 FROM forum_publication_snapshots p JOIN moderation_events m ON m.id=p.moderation_event_id
  WHERE p.id=NEW.approved_snapshot_id AND p.school_id=NEW.school_id AND p.post_id=NEW.id
   AND p.content_revision=NEW.content_revision AND p.title=NEW.title AND p.content=NEW.content AND p.images=NEW.images AND p.tags=NEW.tags
   AND m.school_id=NEW.school_id AND m.object_id=NEW.id AND m.object_kind='post' AND m.decision='approved' AND m.actor_id=p.moderator_id
 ) THEN RAISE EXCEPTION 'approved miniapp post requires matching human review evidence'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_post_publication_guard BEFORE INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_guard_post_publication();

-- Identity assignment is distinct from byte freezing; preparing routes also need withdrawals.
CREATE FUNCTION forum_bind_prepared_source(event uuid, cv uuid, bv uuid, route text) RETURNS void LANGUAGE plpgsql AS $$
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
 INSERT INTO forum_source_route_states(school_id,post_id,origin,source_id,route_id,connection_version,binding_version,last_sequence,last_event_type,last_event_id)
 VALUES(o.school_id,o.post_id,v.origin,v.source_id,route,cv,bv,o.sequence,'content.upsert',event)
 ON CONFLICT(school_id,post_id,origin,source_id,route_id) DO UPDATE SET connection_version=excluded.connection_version,binding_version=excluded.binding_version,
 last_sequence=excluded.last_sequence,last_event_type=excluded.last_event_type,last_event_id=excluded.last_event_id,receipt_id=NULL,receipt_status=NULL
 WHERE forum_source_route_states.last_sequence<excluded.last_sequence;
END $$;
ALTER FUNCTION forum_freeze_source(uuid,uuid,uuid,text) RENAME TO forum_freeze_legacy_source;
CREATE FUNCTION forum_freeze_source(event uuid, cv uuid, bv uuid, route text) RETURNS void LANGUAGE plpgsql AS $$
BEGIN
 IF EXISTS(SELECT 1 FROM forum_outbox WHERE id=event AND projection_version='miniapp_summary_v1' AND payload->>'event_type'='content.upsert') THEN
  PERFORM forum_bind_prepared_source(event,cv,bv,route);
 ELSE PERFORM forum_freeze_legacy_source(event,cv,bv,route); END IF;
END $$;
ALTER FUNCTION forum_source_event(posts,bigint,text,uuid,uuid,text) RENAME TO forum_legacy_source_event;
CREATE FUNCTION forum_source_event(item posts, seq bigint, kind text, cv uuid, bv uuid, route text) RETURNS uuid LANGUAGE plpgsql AS $$
DECLARE event uuid:=gen_random_uuid(); snap forum_publication_snapshots;
BEGIN
 IF item.source_projection<>'miniapp_summary_v1' OR kind='content.withdrawn' THEN
  RETURN forum_legacy_source_event(item,seq,kind,cv,bv,route);
 END IF;
 SELECT * INTO STRICT snap FROM forum_publication_snapshots WHERE id=item.approved_snapshot_id;
 INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload,schema_version,signature_version,delivery_state,publication_snapshot_id,projection_version)
 VALUES(event,item.school_id,item.id,seq,jsonb_build_object('event_id',event,'schema_version',2,'event_type',kind,
 'external_content_id',item.id,'version',seq::text,'sequence',seq,'occurred_at',now(),'title',snap.title,'content',snap.summary),2,2,'waiting_binding',snap.id,'miniapp_summary_v1');
 INSERT INTO forum_publication_preparations(event_id,snapshot_id) VALUES(event,snap.id);
 IF cv IS NOT NULL THEN PERFORM forum_bind_prepared_source(event,cv,bv,route); END IF;
 RETURN event;
END $$;
CREATE OR REPLACE FUNCTION forum_enqueue_post() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.title,NEW.content,NEW.audit_status,NEW.visibility,NEW.approved_snapshot_id) IS NOT DISTINCT FROM
 (OLD.title,OLD.content,OLD.audit_status,OLD.visibility,OLD.approved_snapshot_id) THEN RETURN NEW; END IF;
 PERFORM forum_enqueue_material(NEW,true);
 RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION forum_binding_history() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO school_source_binding_versions(binding_id,school_id,revision,connection_version,source_id,origin,route_id,active,checked_at,actor_id)
 VALUES(NEW.id,NEW.school_id,NEW.revision,NEW.connection_version,NEW.source_id,NEW.origin,NEW.route_id,NEW.active,NEW.checked_at,nullif(current_setting('linjian.source_actor_id',true),'')::uuid);
 UPDATE forum_outbox SET delivery_state=CASE WHEN NOT NEW.active THEN 'paused' WHEN request_body IS NULL THEN 'preparing_content' WHEN last_error IS NULL THEN 'ready' ELSE 'retry_wait' END
 WHERE school_id=NEW.school_id AND payload->>'event_type'='content.upsert' AND receipt_id IS NULL AND lease_token IS NULL AND delivery_state IN ('ready','retry_wait','paused','preparing_content');
 RETURN NEW;
END $$;
CREATE FUNCTION forum_invalidate_preparation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.delivery_state='superseded' THEN
  UPDATE forum_publication_preparations SET state='superseded',revision=revision+1,lease_token=NULL,lease_until=NULL,last_error_code='superseded'
  WHERE event_id=NEW.id AND state NOT IN ('completed','superseded');
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_preparation_invalidation AFTER UPDATE OF delivery_state ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_invalidate_preparation();
CREATE FUNCTION forum_guard_bound_preparation() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.projection_version='miniapp_summary_v1' AND (NEW.projection_version,NEW.publication_snapshot_id) IS DISTINCT FROM (OLD.projection_version,OLD.publication_snapshot_id) THEN RAISE EXCEPTION 'publication intent is immutable'; END IF;
 IF OLD.projection_version='miniapp_summary_v1' AND OLD.connection_version IS NOT NULL AND
 (NEW.connection_version,NEW.binding_version,NEW.source_id,NEW.source_origin,NEW.route_id) IS DISTINCT FROM
 (OLD.connection_version,OLD.binding_version,OLD.source_id,OLD.source_origin,OLD.route_id) THEN RAISE EXCEPTION 'publication binding is immutable'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_bound_publication_guard BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_bound_preparation();
ALTER TABLE forum_publication_preparations ADD COLUMN resolved_policy jsonb;
-- All qualification tests run under the school lock; never acquire a post lock here.
CREATE FUNCTION forum_publication_is_current(event uuid) RETURNS boolean LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM forum_outbox o JOIN forum_publication_snapshots snap ON snap.id=o.publication_snapshot_id
 JOIN posts p ON p.id=o.post_id AND p.school_id=o.school_id JOIN schools s ON s.id=o.school_id
 WHERE o.id=event AND o.delivery_state<>'superseded' AND s.active AND p.audit_status='approved' AND p.visibility='visible'
 AND p.approved_snapshot_id=snap.id AND p.content_revision=snap.content_revision
 AND (o.connection_version IS NULL OR EXISTS(SELECT 1 FROM forum_source_route_states r WHERE r.school_id=o.school_id AND r.post_id=o.post_id
 AND r.origin=o.source_origin AND r.source_id=o.source_id AND r.route_id=o.route_id AND r.last_event_id=o.id)))
$$;
CREATE FUNCTION forum_finalize_prepared_source(event uuid, lease uuid, asset uuid) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE o forum_outbox; prep forum_publication_preparations; snap forum_publication_snapshots; a miniapp_link_assets; v source_connection_versions; p jsonb; body text;
BEGIN
 SELECT * INTO STRICT o FROM forum_outbox WHERE id=event FOR UPDATE;
 SELECT * INTO STRICT prep FROM forum_publication_preparations WHERE event_id=event FOR UPDATE;
 IF prep.state<>'generating' OR prep.lease_token IS DISTINCT FROM lease OR prep.lease_until<=now() OR o.request_body IS NOT NULL THEN RETURN false; END IF;
 IF NOT forum_publication_is_current(event) THEN
  UPDATE forum_outbox SET delivery_state='superseded' WHERE id=event;
  RETURN false;
 END IF;
 IF NOT EXISTS(SELECT 1 FROM school_source_bindings b WHERE b.school_id=o.school_id AND b.active) THEN
  UPDATE forum_publication_preparations SET state='pending',lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event;
  RETURN false;
 END IF;
 SELECT * INTO STRICT snap FROM forum_publication_snapshots WHERE id=prep.snapshot_id;
 SELECT * INTO STRICT a FROM miniapp_link_assets WHERE id=asset;
 IF a.app_id IS DISTINCT FROM prep.resolved_policy->>'app_id' OR a.policy IS DISTINCT FROM prep.resolved_policy
 OR a.page_path<>'pages/index/detail' OR a.canonical_query<>('school='||o.school_id||'&id='||o.post_id) THEN RAISE EXCEPTION 'link does not belong to this publication'; END IF;
 IF a.expires_at IS NOT NULL AND a.expires_at<=now()+make_interval(secs=>(prep.resolved_policy->>'min_remaining')::integer) THEN
  UPDATE forum_publication_preparations SET state='retry_wait',next_attempt_at=now()+interval '5 seconds',last_error_code='link_near_expiry',lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event;
  RETURN false;
 END IF;
 SELECT * INTO STRICT v FROM source_connection_versions WHERE id=o.connection_version;
 p:=o.payload||jsonb_build_object('route_id',o.route_id,'title',snap.title,'content',snap.summary||E'\n查看详情：'||a.link,
 'detail_url',v.public_origin||'/community/'||o.school_id||'/posts/'||o.post_id);
 body:=jsonb_build_object('data',jsonb_build_object('type','source-events','id',event,'attributes',p))::text;
 UPDATE forum_outbox SET payload=p,request_body=body,body_sha256=encode(sha256(convert_to(body,'UTF8')),'hex'),frozen_at=now(),delivery_state='ready' WHERE id=event;
 UPDATE forum_publication_preparations SET state='completed',link_asset_id=asset,completed_at=now(),last_error_code=NULL,lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event;
 RETURN true;
END $$;
