-- Breaking storage change. Do not silently discard existing binary assets.
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM media_assets) OR EXISTS(SELECT 1 FROM operational_media) OR EXISTS(SELECT 1 FROM miniapp_avatar_assets) THEN
  RAISE EXCEPTION 'File storage upgrade requires an empty media library; export and restore files explicitly before upgrading. No binary assets have been deleted.';
 END IF;
END $$;
CREATE TABLE file_assets (
 id uuid PRIMARY KEY, school_id uuid REFERENCES schools, owner_id uuid NOT NULL REFERENCES accounts,
 purpose text NOT NULL CHECK(purpose IN ('post_image','feedback_evidence','operation_artwork','avatar')),
 store_id uuid NOT NULL, mime_type text NOT NULL CHECK(mime_type IN ('image/png','image/jpeg','image/gif','image/webp')),
 size_bytes bigint NOT NULL CHECK(size_bytes BETWEEN 1 AND 5242880),
 width integer NOT NULL CHECK(width BETWEEN 1 AND 8192), height integer NOT NULL CHECK(height BETWEEN 1 AND 8192),
 sha256 text NOT NULL CHECK(sha256 ~ '^[0-9a-f]{64}$'),
 state text NOT NULL CHECK(state IN ('uploading','ready','pending_delete','deleted')),
 idempotency_key uuid NOT NULL, created_at timestamptz NOT NULL DEFAULT now(), expires_at timestamptz NOT NULL,
 retry_at timestamptz NOT NULL DEFAULT now(), attempts integer NOT NULL DEFAULT 0,
 CHECK(width::bigint*height<=24000000), CHECK(purpose IN ('avatar','operation_artwork') OR school_id IS NOT NULL),
 CHECK(purpose<>'avatar' OR school_id IS NULL)
);
CREATE UNIQUE INDEX file_upload_idempotency ON file_assets(owner_id,coalesce(school_id,'00000000-0000-0000-0000-000000000000'::uuid),purpose,idempotency_key);
CREATE INDEX file_cleanup ON file_assets(state,expires_at,retry_at);
CREATE TABLE file_asset_references (
 asset_id uuid NOT NULL REFERENCES file_assets, object_type text NOT NULL, object_id uuid NOT NULL,
 PRIMARY KEY(asset_id,object_type,object_id)
);
CREATE TABLE file_asset_leases (
 asset_id uuid NOT NULL REFERENCES file_assets, owner_id uuid NOT NULL REFERENCES accounts,
 draft_id uuid NOT NULL, expires_at timestamptz NOT NULL,
 PRIMARY KEY(asset_id,owner_id,draft_id)
);
ALTER TABLE media_assets DROP COLUMN content;
ALTER TABLE media_assets ADD FOREIGN KEY(id) REFERENCES file_assets(id);
ALTER TABLE operational_media DROP COLUMN content;
ALTER TABLE operational_media ADD FOREIGN KEY(id) REFERENCES file_assets(id);
ALTER TABLE miniapp_avatar_assets DROP COLUMN content;
ALTER TABLE miniapp_avatar_assets ADD FOREIGN KEY(id) REFERENCES file_assets(id);
-- Tokens must identify the approved post rather than authorizing arbitrary assets.
DELETE FROM media_access_tokens;
ALTER TABLE media_access_tokens ADD COLUMN post_id uuid NOT NULL REFERENCES posts(id);

-- References are registered in the very transaction that writes each business
-- record, including imports, automatic approvals and immutable snapshots.
CREATE FUNCTION linjian_file_references() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v jsonb; ids uuid[] := '{}'; previous uuid[]; asset uuid; object uuid; scope uuid; expected text;
BEGIN
 v := CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 object := coalesce((v->>'id')::uuid,(v->>'account_id')::uuid,'00000000-0000-0000-0000-000000000001'::uuid);
 SELECT coalesce(array_agg(asset_id),'{}') INTO previous FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object;
 DELETE FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 scope := (v->>'school_id')::uuid;
 IF TG_TABLE_NAME IN ('posts','forum_publication_snapshots','cases') THEN
  expected := CASE WHEN TG_TABLE_NAME='cases' THEN 'feedback_evidence' ELSE 'post_image' END;
  SELECT coalesce(array_agg(DISTINCT split_part(x,'/',7)::uuid),'{}') INTO ids
   FROM jsonb_array_elements_text(coalesce(v->'images','[]')) x;
 ELSIF TG_TABLE_NAME='schools' THEN
  scope := object; expected := 'operation_artwork';
  ids := array_remove(ARRAY[(v->>'logo_asset_id')::uuid,(v->>'header_bg_asset_id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='miniapp_users' THEN
  expected := 'avatar'; ids := array_remove(ARRAY[(v->>'avatar_asset_id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='customer_config' THEN
  expected := 'operation_artwork';
  ids := array_remove(ARRAY[(v->>'business_asset_id')::uuid,(v->>'join_asset_id')::uuid,(v->>'service_asset_id')::uuid,(v->>'fallback_asset_id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='school_resources' THEN
  expected := 'operation_artwork'; ids := array_remove(ARRAY[(v->'image_ref'->>'id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='resource_publications' THEN
  expected := 'operation_artwork'; ids := array_remove(ARRAY[(v->'snapshot'->'image_ref'->>'id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='product_previews' AND v->>'kind'='resource' THEN
  expected := 'operation_artwork'; ids := array_remove(ARRAY[(v->'payload'->'image_ref'->>'id')::uuid],NULL);
 END IF;
 FOR asset IN SELECT DISTINCT unnest(ids) ORDER BY 1 LOOP
  PERFORM 1 FROM file_assets WHERE id=asset AND purpose=expected AND state='ready'
   AND school_id IS NOT DISTINCT FROM scope AND (expires_at>now() OR asset=ANY(previous) OR EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=asset) OR EXISTS(SELECT 1 FROM file_asset_leases l WHERE l.asset_id=asset AND l.expires_at>now())) FOR UPDATE;
  IF NOT FOUND THEN RAISE EXCEPTION 'Invalid file asset binding' USING ERRCODE='23514'; END IF;
  INSERT INTO file_asset_references VALUES(asset,TG_TABLE_NAME,object);
 END LOOP;
 RETURN NEW;
END $$;
DO $$ DECLARE t text; BEGIN
 FOREACH t IN ARRAY ARRAY['posts','forum_publication_snapshots','cases','schools','miniapp_users','school_resources','customer_config','resource_publications','product_previews'] LOOP
  EXECUTE format('CREATE TRIGGER file_references AFTER INSERT OR UPDATE OR DELETE ON %I FOR EACH ROW EXECUTE FUNCTION linjian_file_references()',t);
 END LOOP;
END $$;

-- Empty-target snapshot restore includes cyclic business/media references.
DO $$ DECLARE r record; BEGIN
 FOR r IN SELECT conrelid::regclass AS tab,conname FROM pg_constraint WHERE contype='f' AND connamespace=current_schema()::regnamespace LOOP
  EXECUTE format('ALTER TABLE %s ALTER CONSTRAINT %I DEFERRABLE INITIALLY IMMEDIATE',r.tab,r.conname);
 END LOOP;
END $$;
