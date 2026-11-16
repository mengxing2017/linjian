-- Caller authentication does not depend on any downstream content address.
-- Keep historical origins and frozen request bytes unchanged.
ALTER TABLE source_connection_versions ALTER COLUMN public_origin DROP NOT NULL;

-- The forum content projection reuses the existing browser/public address.
-- It is independent of caller credentials and does not validate or probe URLs.
CREATE TABLE forum_publication_configuration (
 singleton boolean PRIMARY KEY DEFAULT true CHECK(singleton),
 public_origin text NOT NULL DEFAULT 'http://localhost:5173'
);
INSERT INTO forum_publication_configuration(singleton) VALUES(true);

-- Content-domain projection for legacy forum posts. No URL validation, network access or sender policy.
CREATE FUNCTION forum_legacy_content_payload(payload jsonb, historical_origin text, school uuid, post uuid)
RETURNS jsonb LANGUAGE sql STABLE AS $$
 SELECT payload || jsonb_build_object('detail_url',coalesce(historical_origin,
  (SELECT public_origin FROM forum_publication_configuration WHERE singleton)) || '/community/' || school || '/posts/' || post)
$$;

CREATE OR REPLACE FUNCTION forum_freeze_legacy_source(event uuid, cv uuid, bv uuid, route text) RETURNS void LANGUAGE plpgsql AS $$
DECLARE o forum_outbox; v source_connection_versions; p jsonb; body text;
BEGIN
 SELECT * INTO STRICT o FROM forum_outbox WHERE id=event FOR UPDATE;
 IF o.request_body IS NOT NULL THEN RETURN; END IF;
 SELECT * INTO STRICT v FROM source_connection_versions WHERE id=cv;
 IF bv IS NOT NULL AND NOT EXISTS(SELECT 1 FROM school_source_binding_versions WHERE id=bv AND school_id=o.school_id AND connection_version=cv AND route_id=route) THEN
 RAISE EXCEPTION 'binding snapshot does not belong to this event';
 END IF;
 p := forum_legacy_content_payload(o.payload - 'external_organization_id',v.public_origin,o.school_id,o.post_id)
  || jsonb_build_object('schema_version',2,'route_id',route);
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

CREATE OR REPLACE FUNCTION forum_finalize_prepared_source(event uuid, lease uuid, asset uuid) RETURNS boolean LANGUAGE plpgsql AS $$
DECLARE o forum_outbox; prep forum_publication_preparations; snap forum_publication_snapshots; a miniapp_link_assets; p jsonb; body text;
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
 p:=o.payload||jsonb_build_object('route_id',o.route_id,'title',snap.title,'content',snap.summary||E'\n查看详情：'||a.link,
 'detail_url',a.link);
 body:=jsonb_build_object('data',jsonb_build_object('type','source-events','id',event,'attributes',p))::text;
 UPDATE forum_outbox SET payload=p,request_body=body,body_sha256=encode(sha256(convert_to(body,'UTF8')),'hex'),frozen_at=now(),delivery_state='ready' WHERE id=event;
 UPDATE forum_publication_preparations SET state='completed',link_asset_id=asset,completed_at=now(),last_error_code=NULL,lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event;
 RETURN true;
END $$;
