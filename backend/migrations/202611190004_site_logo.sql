ALTER TABLE site_settings ADD COLUMN logo_asset_id uuid REFERENCES operational_media(id);

-- Platform logo reuses the controlled operation artwork store. The reference
-- protects the file from orphan cleanup and is removed when the logo is cleared.
CREATE FUNCTION site_settings_logo_reference() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE old_asset uuid; new_asset uuid; object uuid;
BEGIN
 object := CASE WHEN TG_OP='DELETE' THEN OLD.id ELSE NEW.id END;
 IF TG_OP IN ('UPDATE','DELETE') THEN old_asset := OLD.logo_asset_id; END IF;
 IF TG_OP IN ('INSERT','UPDATE') THEN new_asset := NEW.logo_asset_id; END IF;
 PERFORM id FROM file_assets WHERE id=ANY(ARRAY[old_asset,new_asset]) ORDER BY id FOR UPDATE;
 IF new_asset IS NOT NULL THEN
  PERFORM 1 FROM file_assets f JOIN operational_media m ON m.id=f.id
   WHERE f.id=new_asset AND f.purpose='operation_artwork' AND f.state='ready'
   AND f.school_id IS NULL AND m.school_id IS NULL;
  IF NOT FOUND THEN
   RAISE EXCEPTION 'Invalid site logo asset' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
  END IF;
  INSERT INTO file_asset_references(asset_id,object_type,object_id)
   VALUES(new_asset,'site_settings',object) ON CONFLICT DO NOTHING;
  UPDATE file_assets SET ever_bound=true,gc_after=NULL WHERE id=new_asset;
  DELETE FROM file_asset_references
   WHERE object_type='site_settings' AND object_id=object AND asset_id<>new_asset;
 ELSE
  DELETE FROM file_asset_references
   WHERE object_type='site_settings' AND object_id=object;
 END IF;
 PERFORM linjian_refresh_file_gc(ARRAY[old_asset,new_asset]);
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;

CREATE TRIGGER site_settings_logo_reference
 AFTER INSERT OR UPDATE OR DELETE ON site_settings
 FOR EACH ROW EXECUTE FUNCTION site_settings_logo_reference();
