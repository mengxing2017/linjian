-- Run against the schema before the configuration-only credential migration.
BEGIN;
INSERT INTO schools(id,name) VALUES('30000000-0000-0000-0000-000000000001','upgrade-school');
INSERT INTO source_connections(id,name,source_id,origin,key_id,encrypted_secret)
VALUES('30000000-0000-0000-0000-000000000011','existing','30000000-0000-0000-0000-000000000101','https://keeper.example.com','old-key','old-ciphertext');
INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin,public_origin)
VALUES('30000000-0000-0000-0000-000000000021','30000000-0000-0000-0000-000000000011',1,'30000000-0000-0000-0000-000000000101','https://keeper.example.com','https://forum.example.com');
INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id)
VALUES('30000000-0000-0000-0000-000000000031','30000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000011','30000000-0000-0000-0000-000000000021','30000000-0000-0000-0000-000000000101','https://keeper.example.com','upgrade-route');
-- APPLY CONFIGURATION MIGRATION HERE
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema=current_schema() AND table_name='source_connections' AND column_name IN ('secret','encrypted_secret','encryption_version')) THEN RAISE EXCEPTION 'old credential columns remain'; END IF;
 IF NOT EXISTS(SELECT 1 FROM source_connections WHERE id='30000000-0000-0000-0000-000000000011' AND key_id='old-key') THEN RAISE EXCEPTION 'connection identity lost'; END IF;
 IF NOT EXISTS(SELECT 1 FROM school_source_bindings b JOIN source_connection_versions v ON v.id=b.connection_version WHERE b.id='30000000-0000-0000-0000-000000000031' AND v.connection_id='30000000-0000-0000-0000-000000000011' AND b.route_id='upgrade-route') THEN RAISE EXCEPTION 'version or binding lost'; END IF;
END $$;
ROLLBACK;
