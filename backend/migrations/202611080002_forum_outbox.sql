-- Forum events are committed atomically with the post transaction. Imported history
-- is deliberately excluded by the snapshot importer; it never creates send intent.
CREATE TABLE forum_source_versions (
 school_id uuid NOT NULL REFERENCES schools(id), post_id uuid NOT NULL REFERENCES posts(id),
 sequence bigint NOT NULL CHECK(sequence > 0), available boolean NOT NULL,
 PRIMARY KEY(school_id,post_id)
);
CREATE TABLE forum_outbox (
 id uuid PRIMARY KEY, school_id uuid NOT NULL REFERENCES schools(id), post_id uuid NOT NULL REFERENCES posts(id),
 sequence bigint NOT NULL, payload jsonb NOT NULL, request_body text,
 attempts integer NOT NULL DEFAULT 0, next_attempt_at timestamptz NOT NULL DEFAULT now(),
 receipt_id text, receipt_status text, receipt_summary jsonb,
 last_error text, accepted_at timestamptz, created_at timestamptz NOT NULL DEFAULT now(),
 UNIQUE(school_id,post_id,sequence)
);
CREATE INDEX forum_outbox_due ON forum_outbox(next_attempt_at,created_at);
CREATE FUNCTION forum_enqueue_material(item posts, force_version boolean) RETURNS void LANGUAGE plpgsql AS $$
DECLARE eligible boolean; was_available boolean; seq bigint; event uuid;
BEGIN
 eligible := item.audit_status='approved' AND item.visibility='visible'
    AND EXISTS(SELECT 1 FROM schools WHERE id=item.school_id AND active);
 SELECT available INTO was_available FROM forum_source_versions WHERE school_id=item.school_id AND post_id=item.id;
 IF NOT eligible AND NOT coalesce(was_available,false) THEN RETURN; END IF;
 IF NOT force_version AND eligible IS NOT DISTINCT FROM was_available THEN RETURN; END IF;
 INSERT INTO forum_source_versions(school_id,post_id,sequence,available) VALUES(item.school_id,item.id,1,eligible)
 ON CONFLICT(school_id,post_id) DO UPDATE SET sequence=forum_source_versions.sequence+1,available=excluded.available
 RETURNING sequence INTO seq;
 event := gen_random_uuid();
 INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload) VALUES(event,item.school_id,item.id,seq,
 jsonb_build_object('event_id',event,'schema_version',1,
 'event_type',CASE WHEN eligible THEN 'content.upsert' ELSE 'content.withdrawn' END,
 'external_organization_id',item.school_id,'external_content_id',item.id,
 'version',seq::text,'sequence',seq,'occurred_at',now(),
 'title',CASE WHEN eligible THEN item.title ELSE NULL END,
 'content',CASE WHEN eligible THEN item.content ELSE NULL END));
END $$;
CREATE FUNCTION forum_enqueue_post() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.title,NEW.content,NEW.audit_status,NEW.visibility,NEW.images,NEW.tags)
    IS NOT DISTINCT FROM (OLD.title,OLD.content,OLD.audit_status,OLD.visibility,OLD.images,OLD.tags) THEN
  RETURN NEW;
 END IF;
 PERFORM forum_enqueue_material(NEW,true);
 RETURN NEW;
END $$;
CREATE TRIGGER forum_post_outbox AFTER INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_enqueue_post();
CREATE FUNCTION forum_enqueue_school() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE item posts;
BEGIN
 IF NEW.active IS DISTINCT FROM OLD.active THEN
  FOR item IN SELECT * FROM posts WHERE school_id=NEW.id ORDER BY id FOR UPDATE LOOP
   PERFORM forum_enqueue_material(item,false);
  END LOOP;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_school_outbox AFTER UPDATE OF active ON schools FOR EACH ROW EXECUTE FUNCTION forum_enqueue_school();
