-- No legacy data conversion or parallel enqueue implementation.
CREATE TABLE school_member_capabilities (
 school_id uuid NOT NULL, account_id uuid NOT NULL,
 capability text NOT NULL CHECK(capability='post.recommend'),
 granted_by uuid NOT NULL REFERENCES accounts(id), created_at timestamptz NOT NULL DEFAULT now(),
 PRIMARY KEY(school_id,account_id,capability),
 FOREIGN KEY(school_id,account_id) REFERENCES school_memberships(school_id,account_id) ON DELETE CASCADE
);
CREATE FUNCTION forum_clear_member_capabilities() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.role<>'operator' THEN DELETE FROM school_member_capabilities WHERE school_id=NEW.school_id AND account_id=NEW.account_id; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_member_capabilities AFTER UPDATE OF role ON school_memberships FOR EACH ROW EXECUTE FUNCTION forum_clear_member_capabilities();
ALTER TABLE forum_outbox ADD COLUMN trigger_type text NOT NULL CHECK(trigger_type IN ('review_publication','manual_recommendation'));
CREATE TABLE forum_post_recommendations (
 id uuid PRIMARY KEY, school_id uuid NOT NULL REFERENCES schools(id), post_id uuid NOT NULL REFERENCES posts(id),
 actor_id uuid NOT NULL REFERENCES accounts(id), reason text NOT NULL CHECK(length(btrim(reason)) BETWEEN 1 AND 500),
 created_at timestamptz NOT NULL DEFAULT now(), publication_snapshot_id uuid NOT NULL REFERENCES forum_publication_snapshots(id),
 post_revision bigint NOT NULL, content_revision bigint NOT NULL, idempotency_key uuid NOT NULL,
 request_digest text NOT NULL, event_id uuid NOT NULL UNIQUE REFERENCES forum_outbox(id),
 UNIQUE(school_id,actor_id,idempotency_key)
);
CREATE INDEX forum_recommendation_history ON forum_outbox(school_id,post_id,created_at DESC,id DESC);
CREATE TABLE forum_source_terminations (
 event_id uuid PRIMARY KEY REFERENCES forum_outbox(id), actor_id uuid NOT NULL REFERENCES accounts(id),
 reason text NOT NULL CHECK(length(btrim(reason)) BETWEEN 1 AND 500), created_at timestamptz NOT NULL DEFAULT now()
);

-- One evidence predicate for preview and both creation triggers.
CREATE FUNCTION forum_post_has_publication_evidence(item posts) RETURNS boolean LANGUAGE sql STABLE AS $$
 SELECT EXISTS(SELECT 1 FROM forum_publication_snapshots s JOIN moderation_events m ON m.id=s.moderation_event_id
 WHERE s.id=item.approved_snapshot_id AND s.school_id=item.school_id AND s.post_id=item.id
 AND s.content_revision=item.content_revision AND s.approved_post_revision<=item.revision
 AND s.title=item.title AND s.content=item.content AND s.images=item.images AND s.tags=item.tags
 AND s.decision_source=m.decision_source AND m.school_id=item.school_id AND m.object_id=item.id AND m.object_kind='post' AND m.decision='approved'
 AND m.content_revision=item.content_revision AND m.review_digest IS NOT NULL
 AND ((s.decision_source='human' AND m.actor_id=s.moderator_id)
 OR (s.decision_source='automatic' AND m.actor_id IS NULL AND s.moderator_id IS NULL AND m.policy_revision=s.policy_revision AND m.policy_mode='auto')))
$$;

-- Sole creation primitive. Both review triggers and manual commands call this function.
CREATE FUNCTION forum_create_source_intent(item posts, trigger_kind text) RETURNS uuid LANGUAGE plpgsql AS $$
DECLARE seq bigint; b school_source_bindings; bv uuid; event uuid:=gen_random_uuid();
BEGIN
 IF trigger_kind NOT IN ('review_publication','manual_recommendation') THEN RAISE EXCEPTION 'invalid trigger'; END IF;
 PERFORM id FROM schools WHERE id=item.school_id FOR UPDATE;
 IF item.audit_status<>'approved' OR item.visibility<>'visible' OR item.approved_snapshot_id IS NULL
 OR NOT EXISTS(SELECT 1 FROM schools WHERE id=item.school_id AND active) THEN RAISE EXCEPTION 'publication not eligible'; END IF;
 IF NOT forum_post_has_publication_evidence(item) THEN RAISE EXCEPTION 'publication evidence mismatch'; END IF;
 INSERT INTO forum_source_versions(school_id,post_id,sequence,available) VALUES(item.school_id,item.id,1,true)
 ON CONFLICT(school_id,post_id) DO UPDATE SET sequence=forum_source_versions.sequence+1,available=true,delivery_intent=true RETURNING sequence INTO seq;
 SELECT sb.* INTO b FROM school_source_bindings sb JOIN source_active_configuration ac ON ac.connection_id=sb.connection_id WHERE sb.school_id=item.school_id FOR SHARE OF sb,ac;
 IF b.id IS NOT NULL THEN SELECT id INTO bv FROM school_source_binding_versions WHERE binding_id=b.id AND revision=b.revision; END IF;
 INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload,schema_version,signature_version,delivery_state,publication_snapshot_id,projection_version,trigger_type)
 VALUES(event,item.school_id,item.id,seq,jsonb_build_object('event_id',event,'schema_version',2,'event_type','content.upsert','occurred_at',now()),2,2,'waiting_binding',item.approved_snapshot_id,'message_v1',trigger_kind);
 INSERT INTO forum_publication_preparations(event_id,snapshot_id) VALUES(event,item.approved_snapshot_id);
 IF b.id IS NOT NULL THEN PERFORM forum_bind_prepared_source(event,b.connection_version,bv,b.route_id); END IF;
 IF b.id IS NOT NULL AND NOT b.active THEN UPDATE forum_outbox SET delivery_state='paused' WHERE id=event; END IF;
 RETURN event;
END $$;
CREATE OR REPLACE FUNCTION forum_enqueue_post() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.title,NEW.content,NEW.images,NEW.tags,NEW.audit_status,NEW.visibility,NEW.approved_snapshot_id) IS NOT DISTINCT FROM
 (OLD.title,OLD.content,OLD.images,OLD.tags,OLD.audit_status,OLD.visibility,OLD.approved_snapshot_id) THEN RETURN NEW; END IF;
 PERFORM id FROM schools WHERE id=NEW.school_id FOR UPDATE;
 -- Explicit invalidation, separate from creation. Never cancel a live delivery lease.
 UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=NEW.school_id AND post_id=NEW.id
 AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
 IF NEW.audit_status='approved' AND NEW.visibility='visible' AND EXISTS(SELECT 1 FROM schools WHERE id=NEW.school_id AND active) THEN
  PERFORM forum_create_source_intent(NEW,'review_publication');
 ELSE
  UPDATE forum_source_versions SET available=false WHERE school_id=NEW.school_id AND post_id=NEW.id;
 END IF;
 RETURN NEW;
END $$;
DROP FUNCTION forum_enqueue_material(posts,boolean);
DROP FUNCTION forum_source_event(posts,bigint,text,uuid,uuid,text);

-- One sequence identifies one event, independent of route binding.
CREATE UNIQUE INDEX forum_outbox_single_intent ON forum_outbox(school_id,post_id,sequence);
CREATE TRIGGER forum_recommendation_immutable BEFORE UPDATE OR DELETE ON forum_post_recommendations FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();
CREATE TRIGGER forum_termination_immutable BEFORE UPDATE OR DELETE ON forum_source_terminations FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();
CREATE FUNCTION forum_guard_source_trigger() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.trigger_type IS DISTINCT FROM OLD.trigger_type THEN RAISE EXCEPTION 'source trigger is immutable'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_source_trigger_immutable BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_source_trigger();

DROP FUNCTION forum_snapshot_source_v2();
DROP FUNCTION forum_freeze_source(uuid,uuid,uuid,text);
