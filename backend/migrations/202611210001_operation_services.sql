ALTER TABLE school_resources DROP CONSTRAINT school_resources_kind_check;
ALTER TABLE school_resources ADD CONSTRAINT school_resources_kind_check CHECK(kind IN ('banner','qrcode','service','legal','tag','blocked_word','shortcut'));
ALTER TABLE school_resources ADD CONSTRAINT operation_action_boundary CHECK (coalesce((
 (kind <> 'banner' OR action->>'kind' IN ('none','internal_page')) AND
 (kind <> 'qrcode' OR action->>'kind'='none') AND
 (kind <> 'shortcut' OR action->>'kind' IN ('none','internal_page','target_channel'))
),false));
CREATE TABLE operation_service_categories (
 id uuid PRIMARY KEY, school_id uuid REFERENCES schools(id), name text NOT NULL CHECK(char_length(btrim(name)) BETWEEN 1 AND 40),
 active boolean NOT NULL, sort_order integer NOT NULL, revision bigint NOT NULL DEFAULT 1
);
CREATE TABLE operation_partners (
 id uuid PRIMARY KEY, name text NOT NULL, contact text NOT NULL, active boolean NOT NULL, revision bigint NOT NULL DEFAULT 1
);
CREATE TABLE operation_cooperations (
 id uuid PRIMARY KEY, partner_id uuid NOT NULL REFERENCES operation_partners(id), name text NOT NULL,
 school_ids uuid[] NOT NULL DEFAULT '{}', model text NOT NULL CHECK(model IN ('service','sponsor','cps')), disclosure text NOT NULL, channel_code text NOT NULL,
 active boolean NOT NULL, starts_at timestamptz, ends_at timestamptz, revision bigint NOT NULL DEFAULT 1,
 CHECK(starts_at IS NULL OR ends_at IS NULL OR starts_at<ends_at)
);
CREATE TABLE operation_service_targets (
 id uuid PRIMARY KEY, revision bigint NOT NULL DEFAULT 1
);
CREATE TABLE operation_target_versions (
 id uuid PRIMARY KEY, target_id uuid NOT NULL REFERENCES operation_service_targets(id), revision bigint NOT NULL DEFAULT 1,
 version_number bigint NOT NULL, content_digest text NOT NULL, UNIQUE(target_id,version_number),
 config jsonb NOT NULL, state text NOT NULL CHECK(state IN ('draft','pending','approved','rejected','withdrawn')),
 creator_id uuid NOT NULL REFERENCES accounts(id), submitter_id uuid REFERENCES accounts(id), created_at timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX service_target_one_approved ON operation_target_versions(target_id) WHERE state='approved';
CREATE TABLE operation_target_reviews (
 id uuid PRIMARY KEY, version_id uuid NOT NULL REFERENCES operation_target_versions(id), actor_id uuid NOT NULL REFERENCES accounts(id),
 decision text NOT NULL CHECK(decision IN ('submit','approve','reject','withdraw')), reason text NOT NULL, created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TRIGGER service_reviews_immutable BEFORE UPDATE OR DELETE ON operation_target_reviews FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();
CREATE FUNCTION linjian_service_version_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF OLD.config IS DISTINCT FROM NEW.config OR OLD.target_id<>NEW.target_id OR OLD.creator_id<>NEW.creator_id OR OLD.version_number<>NEW.version_number OR OLD.content_digest<>NEW.content_digest OR OLD.created_at<>NEW.created_at THEN
  RAISE EXCEPTION 'Service version configurations are immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER service_version_immutable BEFORE UPDATE ON operation_target_versions FOR EACH ROW EXECUTE FUNCTION linjian_service_version_immutable();
ALTER TABLE school_resources
 ADD COLUMN subtitle text NOT NULL DEFAULT '', ADD COLUMN badge text NOT NULL DEFAULT '', ADD COLUMN featured boolean NOT NULL DEFAULT false,
 ADD COLUMN category_id uuid REFERENCES operation_service_categories(id), ADD COLUMN cooperation_id uuid REFERENCES operation_cooperations(id),
 ADD COLUMN target_version_id uuid REFERENCES operation_target_versions(id);
ALTER TABLE school_resources ADD CONSTRAINT banner_target_boundary CHECK(kind<>'banner' OR (target_version_id IS NULL AND cooperation_id IS NULL));
ALTER TABLE school_resources ADD CONSTRAINT shortcut_target_reference CHECK(coalesce((kind<>'shortcut' OR
 ((action->>'kind'='target_channel' AND target_version_id IS NOT NULL AND target_version_id::text=action->>'version_id') OR
 (action->>'kind' IN ('none','internal_page') AND target_version_id IS NULL))),false));
CREATE TABLE school_shortcut_overrides (
 id uuid PRIMARY KEY, school_id uuid NOT NULL REFERENCES schools(id), resource_id uuid NOT NULL REFERENCES school_resources(id),
 presentation jsonb NOT NULL, revision bigint NOT NULL DEFAULT 1, UNIQUE(school_id,resource_id)
);
CREATE TABLE operation_config_revisions (
 id uuid PRIMARY KEY, revision bigint NOT NULL DEFAULT 0
);
CREATE TABLE marketing_slots (
 id uuid PRIMARY KEY, name text NOT NULL, ad_unit_id text NOT NULL, format text NOT NULL CHECK(format IN ('custom','rewarded')), reward_scene text,
 qualification text NOT NULL, active boolean NOT NULL, revision bigint NOT NULL DEFAULT 1,
 CHECK((format='custom' AND reward_scene IS NULL) OR (format='rewarded' AND reward_scene='service_theme'))
);
CREATE TABLE school_marketing_settings (
 id uuid PRIMARY KEY, school_id uuid NOT NULL REFERENCES schools(id), slot_id uuid NOT NULL REFERENCES marketing_slots(id),
 enabled boolean NOT NULL, revision bigint NOT NULL DEFAULT 1, UNIQUE(school_id,slot_id)
);
CREATE FUNCTION linjian_service_file_references() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE payload jsonb; previous uuid[]; assets uuid[]; asset uuid; object uuid; scope uuid;
BEGIN
 payload:=CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 object:=(payload->>'id')::uuid; scope:=(payload->>'school_id')::uuid;
 SELECT coalesce(array_agg(asset_id),'{}') INTO previous FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object;
 IF TG_OP='DELETE' THEN assets:='{}';
 ELSIF TG_TABLE_NAME='school_shortcut_overrides' THEN assets:=array_remove(ARRAY[(payload->'presentation'->'image_ref'->>'id')::uuid],NULL);
 ELSE
  SELECT coalesce(array_agg((item->'action'->>'asset_id')::uuid),'{}') INTO assets
   FROM jsonb_array_elements(payload->'config'->'channels') item WHERE item->'action'->>'kind'='qr_code';
 END IF;
 PERFORM id FROM file_assets WHERE id=ANY(previous || assets) ORDER BY id FOR UPDATE;
 DELETE FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object AND NOT(asset_id=ANY(assets));
 FOREACH asset IN ARRAY assets LOOP
  PERFORM 1 FROM file_assets WHERE id=asset AND purpose='operation_artwork' AND state='ready' AND school_id IS NOT DISTINCT FROM scope
   AND ((NOT ever_bound AND expires_at>now()) OR asset=ANY(previous) OR EXISTS(SELECT 1 FROM file_asset_references WHERE asset_id=asset));
  IF NOT FOUND THEN RAISE EXCEPTION 'Invalid service file binding' USING ERRCODE='23514', CONSTRAINT='file_asset_binding'; END IF;
  INSERT INTO file_asset_references VALUES(asset,TG_TABLE_NAME,object) ON CONFLICT DO NOTHING;
  UPDATE file_assets SET ever_bound=true,gc_after=NULL WHERE id=asset;
 END LOOP;
 PERFORM linjian_refresh_file_gc(previous || assets);
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER file_references AFTER INSERT OR UPDATE OR DELETE ON school_shortcut_overrides FOR EACH ROW EXECUTE FUNCTION linjian_service_file_references();
CREATE TRIGGER file_references AFTER INSERT OR UPDATE OR DELETE ON operation_target_versions FOR EACH ROW EXECUTE FUNCTION linjian_service_file_references();
