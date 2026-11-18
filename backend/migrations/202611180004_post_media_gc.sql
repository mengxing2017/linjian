-- A released asset gets its own grace period, independent of upload expiry.
ALTER TABLE file_assets ADD COLUMN ever_bound boolean NOT NULL DEFAULT false,
 ADD COLUMN gc_after timestamptz;
CREATE INDEX file_gc_due ON file_assets(store_id,gc_after,id) WHERE state IN ('ready','uploading');
CREATE INDEX file_gc_retry ON file_assets(store_id,retry_at,id) WHERE state='pending_delete';
CREATE INDEX file_lease_expiry ON file_asset_leases(expires_at,asset_id);
CREATE INDEX media_token_asset_expiry ON media_access_tokens(asset_id,expires_at);
CREATE INDEX product_preview_expiry ON product_previews(expires_at,id);

CREATE FUNCTION linjian_media_grace_hours() RETURNS integer LANGUAGE plpgsql AS $$
DECLARE hours integer := coalesce(nullif(current_setting('linjian.media_orphan_grace_hours',true),''),'24')::integer;
BEGIN
 IF hours NOT BETWEEN 1 AND 168 THEN RAISE EXCEPTION 'Invalid media grace hours'; END IF;
 RETURN hours;
END $$;
CREATE FUNCTION linjian_file_protected(asset uuid) RETURNS boolean LANGUAGE sql AS $$
 SELECT EXISTS(SELECT 1 FROM file_asset_references WHERE asset_id=asset)
 OR EXISTS(SELECT 1 FROM file_asset_leases WHERE asset_id=asset AND expires_at>now())
 OR EXISTS(SELECT 1 FROM media_access_tokens WHERE asset_id=asset AND expires_at>now())
$$;
CREATE FUNCTION linjian_refresh_file_gc(assets uuid[]) RETURNS void LANGUAGE plpgsql AS $$
DECLARE asset uuid;
BEGIN
 FOR asset IN SELECT id FROM file_assets WHERE id=ANY(assets) ORDER BY id FOR UPDATE LOOP
  UPDATE file_assets SET gc_after=CASE WHEN linjian_file_protected(asset) THEN NULL
   ELSE coalesce(gc_after,now()+make_interval(hours=>linjian_media_grace_hours())) END
   WHERE id=asset AND state IN ('ready','uploading');
 END LOOP;
END $$;
CREATE FUNCTION linjian_file_history_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.ever_bound AND NOT NEW.ever_bound THEN
  RAISE EXCEPTION 'File binding history cannot be reset' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER file_history_guard BEFORE UPDATE ON file_assets FOR EACH ROW EXECUTE FUNCTION linjian_file_history_guard();

CREATE OR REPLACE FUNCTION linjian_file_references() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v jsonb; ids uuid[] := '{}'; previous uuid[]; asset uuid; object uuid; scope uuid; expected text;
BEGIN
 v := CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 object := coalesce((v->>'id')::uuid,(v->>'account_id')::uuid,'00000000-0000-0000-0000-000000000001'::uuid);
 SELECT coalesce(array_agg(asset_id),'{}') INTO previous FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object;

 scope := (v->>'school_id')::uuid;
 IF TG_OP='DELETE' OR (TG_TABLE_NAME='posts' AND v->>'visibility'='deleted') THEN
  ids := '{}';
 ELSIF TG_TABLE_NAME IN ('posts','forum_publication_snapshots','cases') THEN
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
 -- Lock the union before changing references, including assets only being released.
 PERFORM id FROM file_assets WHERE id=ANY(previous || ids) ORDER BY id FOR UPDATE;
 DELETE FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object AND NOT(asset_id=ANY(ids));
 FOR asset IN SELECT DISTINCT unnest(ids) ORDER BY 1 LOOP
  PERFORM 1 FROM file_assets WHERE id=asset AND purpose=expected AND state='ready'
   AND school_id IS NOT DISTINCT FROM scope AND ((NOT ever_bound AND expires_at>now()) OR asset=ANY(previous)
    OR EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=asset)
    OR EXISTS(SELECT 1 FROM file_asset_leases l WHERE l.asset_id=asset AND l.expires_at>now()));
  IF NOT FOUND THEN RAISE EXCEPTION 'Invalid file asset binding' USING ERRCODE='23514', CONSTRAINT='file_asset_binding'; END IF;
  INSERT INTO file_asset_references VALUES(asset,TG_TABLE_NAME,object) ON CONFLICT DO NOTHING;
  UPDATE file_assets SET ever_bound=true,gc_after=NULL WHERE id=asset;
 END LOOP;
 PERFORM linjian_refresh_file_gc(previous || ids);
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;

-- Lease/token writers share the same asset lock as bindings and cleanup.
CREATE FUNCTION linjian_file_protection_guard() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE asset uuid; f file_assets;
BEGIN
 asset := CASE WHEN TG_OP='DELETE' THEN OLD.asset_id ELSE NEW.asset_id END;
 IF TG_OP='UPDATE' AND NEW.asset_id<>OLD.asset_id THEN
  RAISE EXCEPTION 'Protection asset cannot change' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
 END IF;
 SELECT * INTO STRICT f FROM file_assets WHERE id=asset FOR UPDATE;
 IF TG_OP<>'DELETE' AND NEW.expires_at>now() THEN
  IF f.state<>'ready' THEN
   RAISE EXCEPTION 'File no longer available' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
  END IF;
  IF TG_TABLE_NAME='file_asset_leases' THEN
   IF f.owner_id<>NEW.owner_id OR f.purpose<>'post_image' THEN
    RAISE EXCEPTION 'Invalid draft lease' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
   END IF;
  ELSE
   IF f.school_id IS DISTINCT FROM NEW.school_id OR f.purpose<>'post_image' OR NOT EXISTS(
    SELECT 1 FROM posts p JOIN schools s ON s.id=p.school_id WHERE p.id=NEW.post_id AND p.school_id=NEW.school_id
     AND s.active AND p.visibility='visible' AND p.audit_status='approved'
     AND p.images @> jsonb_build_array('/api/v1/schools/'||p.school_id||'/media/'||asset)) THEN
    RAISE EXCEPTION 'Invalid media token' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
   END IF;
  END IF;
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;
CREATE FUNCTION linjian_file_protection_changed() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN PERFORM linjian_refresh_file_gc(ARRAY[OLD.asset_id]); RETURN OLD; END IF;
 PERFORM linjian_refresh_file_gc(ARRAY[NEW.asset_id]); RETURN NEW;
END $$;
CREATE TRIGGER file_lease_guard BEFORE INSERT OR UPDATE OR DELETE ON file_asset_leases FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_guard();
CREATE TRIGGER file_lease_changed AFTER INSERT OR UPDATE OR DELETE ON file_asset_leases FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_changed();
CREATE TRIGGER media_token_guard BEFORE INSERT OR UPDATE OR DELETE ON media_access_tokens FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_guard();
CREATE TRIGGER media_token_changed AFTER INSERT OR UPDATE OR DELETE ON media_access_tokens FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_changed();

-- Capture history before removing stale soft-deleted-post references.
UPDATE file_assets f SET ever_bound=true WHERE EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=f.id)
 OR EXISTS(SELECT 1 FROM posts p WHERE p.school_id=f.school_id AND p.images @> jsonb_build_array('/api/v1/schools/'||p.school_id||'/media/'||f.id));
DELETE FROM file_asset_references r USING posts p WHERE r.object_type='posts' AND r.object_id=p.id AND p.visibility='deleted';
-- No disk I/O in migration. Existing orphans get a full grace period.
UPDATE file_assets SET gc_after=CASE WHEN linjian_file_protected(id) THEN NULL
 ELSE now()+make_interval(hours=>linjian_media_grace_hours()) END WHERE state IN ('ready','uploading');
