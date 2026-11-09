-- Explicit policy decisions; historical rows retain their original evidence.
ALTER TABLE moderation_events ALTER COLUMN actor_id DROP NOT NULL,
 ADD COLUMN decision_source text NOT NULL DEFAULT 'legacy',
 ADD COLUMN content_revision bigint,
 ADD COLUMN policy_revision bigint,
 ADD COLUMN policy_mode text,
 ADD COLUMN review_digest text;
ALTER TABLE moderation_events ALTER COLUMN decision_source SET DEFAULT 'human';
ALTER TABLE moderation_events ADD CONSTRAINT moderation_evidence CHECK (
 decision_source='legacy' OR
 (decision_source='human' AND actor_id IS NOT NULL AND coalesce(content_revision>0,false)) OR
 (decision_source='automatic' AND actor_id IS NULL AND decision='approved'
  AND coalesce(content_revision>0 AND policy_revision>0 AND policy_mode='auto',false) AND review_digest IS NOT NULL)
);
ALTER TABLE forum_publication_snapshots ALTER COLUMN moderator_id DROP NOT NULL,
 ADD COLUMN decision_source text NOT NULL DEFAULT 'legacy',
 ADD COLUMN policy_revision bigint;
ALTER TABLE forum_publication_snapshots ALTER COLUMN decision_source SET DEFAULT 'human';
ALTER TABLE forum_publication_snapshots ADD CONSTRAINT publication_decision_source CHECK (
 decision_source='legacy' OR (decision_source='human' AND moderator_id IS NOT NULL) OR
 (decision_source='automatic' AND moderator_id IS NULL AND coalesce(policy_revision>0,false))
);

CREATE FUNCTION forum_guard_decision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'moderation evidence is immutable'; END IF;
 IF NEW.decision_source='legacy' THEN RAISE EXCEPTION 'new legacy decisions are forbidden'; END IF;
 IF NEW.object_kind NOT IN ('post','comment') OR NEW.decision NOT IN ('pending','approved','rejected') THEN RAISE EXCEPTION 'invalid moderation decision'; END IF;
 IF NEW.decision_source='human' AND NOT EXISTS(
  SELECT 1 FROM accounts a JOIN schools s ON s.id=NEW.school_id AND s.active
  WHERE a.id=NEW.actor_id AND a.active AND (a.platform_admin OR EXISTS(
   SELECT 1 FROM school_memberships m WHERE m.account_id=a.id AND m.school_id=s.id AND m.role IN ('admin','operator')))
 ) THEN RAISE EXCEPTION 'human decision requires a school moderator'; END IF;
 IF NEW.decision_source='automatic' AND NOT EXISTS(
  SELECT 1 FROM schools WHERE id=NEW.school_id AND active AND revision=NEW.policy_revision
   AND config->>(CASE WHEN NEW.object_kind='post' THEN 'post_audit_mode' ELSE 'comment_audit_mode' END)='auto'
 ) THEN RAISE EXCEPTION 'automatic decision requires active school policy'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_decision_guard BEFORE INSERT OR UPDATE OR DELETE ON moderation_events
 FOR EACH ROW EXECUTE FUNCTION forum_guard_decision();

CREATE OR REPLACE FUNCTION forum_guard_post_publication() RETURNS trigger LANGUAGE plpgsql AS $$
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
   AND p.approved_post_revision<=NEW.revision
   AND (p.approved_post_revision=NEW.revision OR (TG_OP='UPDATE' AND OLD.approved_snapshot_id=NEW.approved_snapshot_id))
   AND m.school_id=NEW.school_id AND m.object_id=NEW.id AND m.object_kind='post' AND m.decision='approved'
   AND p.decision_source=m.decision_source
   AND ((p.decision_source='legacy' AND m.actor_id=p.moderator_id)
     OR (p.decision_source='human' AND m.actor_id=p.moderator_id AND m.content_revision=NEW.content_revision AND m.review_digest IS NOT NULL)
     OR (p.decision_source='automatic' AND m.actor_id IS NULL AND p.moderator_id IS NULL
       AND m.content_revision=NEW.content_revision AND m.policy_revision=p.policy_revision AND m.policy_mode='auto'))
 ) THEN RAISE EXCEPTION 'approved miniapp post requires matching publication evidence'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_decision_details(school uuid, object uuid, kind text, version bigint, status text)
RETURNS jsonb LANGUAGE sql STABLE AS $$
 SELECT jsonb_build_object('decision_source',coalesce(m.decision_source,CASE WHEN status<>'pending' THEN 'legacy' END),
  'decided_at',m.created_at,'moderator_id',m.actor_id)
 FROM (SELECT 1) dummy LEFT JOIN LATERAL (
  SELECT * FROM moderation_events WHERE school_id=school AND object_id=object AND object_kind=kind
   AND (content_revision=version OR (content_revision IS NULL AND version=1)) AND decision=status
  ORDER BY created_at DESC,id DESC LIMIT 1
 ) m ON true
$$;
ALTER TABLE schools ALTER COLUMN config SET DEFAULT '{"post_audit_mode":"manual","comment_audit_mode":"manual"}'::jsonb;

CREATE FUNCTION forum_guard_new_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.decision_source='legacy' THEN RAISE EXCEPTION 'new legacy snapshots are forbidden'; END IF;
 IF NOT EXISTS(
  SELECT 1 FROM posts p JOIN moderation_events m ON m.id=NEW.moderation_event_id JOIN schools s ON s.id=p.school_id AND s.active
  WHERE p.id=NEW.post_id AND p.school_id=NEW.school_id AND p.author_id=NEW.author_id
   AND p.content_revision=NEW.content_revision AND p.revision+1=NEW.approved_post_revision
   AND p.title=NEW.title AND p.content=NEW.content AND p.images=NEW.images AND p.tags=NEW.tags
   AND m.school_id=p.school_id AND m.object_id=p.id AND m.object_kind='post' AND m.decision='approved'
   AND m.content_revision=p.content_revision AND m.decision_source=NEW.decision_source
   AND ((NEW.decision_source='human' AND m.actor_id=NEW.moderator_id AND m.review_digest IS NOT NULL)
    OR (NEW.decision_source='automatic' AND m.actor_id IS NULL AND NEW.moderator_id IS NULL
     AND m.policy_revision=NEW.policy_revision AND s.revision=NEW.policy_revision AND s.config->>'post_audit_mode'='auto'))
 ) THEN RAISE EXCEPTION 'snapshot requires matching current decision and content'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_new_snapshot_guard BEFORE INSERT ON forum_publication_snapshots FOR EACH ROW EXECUTE FUNCTION forum_guard_new_snapshot();

CREATE INDEX moderation_object_history ON moderation_events(school_id,object_kind,object_id,created_at DESC,id DESC);
