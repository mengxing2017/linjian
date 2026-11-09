-- Presentation metadata is independent from reviewed content and publication snapshots.
ALTER TABLE posts DROP CONSTRAINT posts_visibility_check;
ALTER TABLE posts ADD CONSTRAINT posts_visibility_check CHECK(visibility IN ('visible','hidden','offline','deleted'));
CREATE SEQUENCE post_sort_order_seq;
ALTER TABLE posts ADD COLUMN is_top boolean NOT NULL DEFAULT false,
 ADD COLUMN sort_order bigint NOT NULL DEFAULT 0;
WITH ranked AS (
 SELECT id,row_number() OVER (ORDER BY school_id,top_sort,created_at,id DESC) AS weight FROM posts
) UPDATE posts p SET is_top=(p.top_sort>0 AND p.visibility<>'deleted'),sort_order=r.weight,top_sort=CASE WHEN p.visibility='deleted' THEN 0 ELSE p.top_sort END FROM ranked r WHERE r.id=p.id;
SELECT setval('post_sort_order_seq',greatest(coalesce((SELECT max(sort_order) FROM posts),0),1));
ALTER TABLE posts ALTER COLUMN sort_order SET DEFAULT nextval('post_sort_order_seq');
CREATE TABLE post_order_versions(school_id uuid PRIMARY KEY REFERENCES schools(id),revision bigint NOT NULL DEFAULT 1);
INSERT INTO post_order_versions(school_id) SELECT id FROM schools;
CREATE FUNCTION forum_post_order_metadata() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  NEW.is_top:=(NEW.is_top OR NEW.top_sort>0) AND NEW.visibility<>'deleted';
 ELSIF NEW.top_sort IS DISTINCT FROM OLD.top_sort AND NEW.is_top IS NOT DISTINCT FROM OLD.is_top THEN
  NEW.is_top:=NEW.top_sort>0;
 END IF;
 IF NEW.visibility='deleted' THEN NEW.is_top:=false; END IF;
 NEW.top_sort:=CASE WHEN NEW.is_top THEN greatest(NEW.top_sort,1) ELSE 0 END;
 IF TG_OP='UPDATE' AND NEW.is_top IS DISTINCT FROM OLD.is_top THEN NEW.sort_order:=nextval('post_sort_order_seq'); END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_post_order_metadata BEFORE INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_post_order_metadata();
CREATE FUNCTION forum_post_order_version() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (TG_OP='INSERT' AND NEW.visibility<>'deleted') OR (TG_OP='UPDATE' AND
  ((NEW.visibility='deleted') IS DISTINCT FROM (OLD.visibility='deleted') OR
   NEW.is_top IS DISTINCT FROM OLD.is_top OR NEW.sort_order IS DISTINCT FROM OLD.sort_order)) THEN
  INSERT INTO post_order_versions(school_id,revision) VALUES(NEW.school_id,2)
   ON CONFLICT(school_id) DO UPDATE SET revision=post_order_versions.revision+1;
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER forum_post_order_version AFTER INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_post_order_version();
CREATE INDEX post_management_order ON posts(school_id,is_top DESC,sort_order DESC,created_at DESC,id) WHERE visibility<>'deleted';
CREATE INDEX post_public_order ON posts(school_id,is_top DESC,sort_order DESC,created_at DESC,id) WHERE visibility='visible' AND audit_status='approved';
CREATE TABLE post_import_jobs(
 id uuid PRIMARY KEY,school_id uuid NOT NULL REFERENCES schools(id),actor_id uuid NOT NULL REFERENCES accounts(id),
 revision bigint NOT NULL DEFAULT 1,digest text NOT NULL,rows jsonb NOT NULL,errors jsonb NOT NULL,
 state text NOT NULL DEFAULT 'preview' CHECK(state IN('preview','committed','expired')),
 result jsonb NOT NULL DEFAULT '[]',idempotency_key uuid,created_at timestamptz NOT NULL DEFAULT now(),
 expires_at timestamptz NOT NULL DEFAULT now()+interval '30 minutes',
 UNIQUE(actor_id,school_id,idempotency_key)
);
CREATE INDEX post_import_expiration ON post_import_jobs(expires_at) WHERE state='preview';
