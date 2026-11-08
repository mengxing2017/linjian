BEGIN;
DO $$ BEGIN
 IF EXISTS(SELECT 1 FROM information_schema.columns WHERE table_schema=current_schema() AND table_name='source_connections' AND column_name IN ('secret','encrypted_secret','encryption_version')) THEN RAISE EXCEPTION 'credential storage remains'; END IF;
END $$;
INSERT INTO schools(id,name) VALUES('20000000-0000-0000-0000-000000000001','school1'),('20000000-0000-0000-0000-000000000002','school2');
INSERT INTO accounts(id,username,display_name) VALUES('20000000-0000-0000-0000-000000000003','source-test','test');
INSERT INTO source_connections(id,name,source_id,origin,key_id) VALUES('20000000-0000-0000-0000-000000000011','connection','20000000-0000-0000-0000-000000000101','https://keeper.example.com','k1');
-- Historical metadata is not a selectable second live upstream.
INSERT INTO source_connections(id,name,source_id,origin,key_id) VALUES('20000000-0000-0000-0000-000000000012','historical','20000000-0000-0000-0000-000000000102','https://old-keeper.example.com','old-key');
DO $$ BEGIN
 IF pg_temp.configured_connection('https://keeper.example.com','k1')->>'id'<>'20000000-0000-0000-0000-000000000011' THEN RAISE EXCEPTION 'configured upstream not resolved'; END IF;
 IF pg_temp.configured_connection('https://keeper.example.com','old-key') IS NOT NULL OR pg_temp.configured_connection('https://old-keeper.example.com','k1') IS NOT NULL THEN RAISE EXCEPTION 'other connection selected'; END IF;
END $$;
INSERT INTO source_connection_versions(id,connection_id,revision,source_id,origin,public_origin) VALUES('20000000-0000-0000-0000-000000000021','20000000-0000-0000-0000-000000000011',1,'20000000-0000-0000-0000-000000000101','https://keeper.example.com','https://forum.example.com');
INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id) VALUES('20000000-0000-0000-0000-000000000031','20000000-0000-0000-0000-000000000001','20000000-0000-0000-0000-000000000011','20000000-0000-0000-0000-000000000021','20000000-0000-0000-0000-000000000101','https://keeper.example.com','m1');
DO $$ BEGIN
 BEGIN INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id) VALUES('20000000-0000-0000-0000-000000000032','20000000-0000-0000-0000-000000000002','20000000-0000-0000-0000-000000000011','20000000-0000-0000-0000-000000000021','20000000-0000-0000-0000-000000000101','https://keeper.example.com','m1'); RAISE EXCEPTION 'route shared across schools'; EXCEPTION WHEN unique_violation THEN NULL; END;
END $$;
-- A different route still cannot create a second record for the same school, even disabled.
DO $$ BEGIN
 BEGIN INSERT INTO school_source_bindings(id,school_id,connection_id,connection_version,source_id,origin,route_id,active) VALUES('20000000-0000-0000-0000-000000000033','20000000-0000-0000-0000-000000000001','20000000-0000-0000-0000-000000000011','20000000-0000-0000-0000-000000000021','20000000-0000-0000-0000-000000000101','https://keeper.example.com','m3',false); RAISE EXCEPTION 'second school route accepted'; EXCEPTION WHEN unique_violation THEN NULL; END;
END $$;
-- This helper is built from the real repository upsert SQL by the isolated runner.
SELECT pg_temp.test_binding('20000000-0000-0000-0000-000000000032','20000000-0000-0000-0000-000000000002','20000000-0000-0000-0000-000000000011','20000000-0000-0000-0000-000000000021','20000000-0000-0000-0000-000000000101','https://keeper.example.com','m2',true);
DO $$ BEGIN IF (SELECT count(*) FROM school_source_bindings WHERE connection_id='20000000-0000-0000-0000-000000000011')<>2 THEN RAISE EXCEPTION 'connection cannot serve two schools'; END IF; END $$;
INSERT INTO posts(id,school_id,author_id,title,content,audit_status) VALUES('20000000-0000-0000-0000-000000000041','20000000-0000-0000-0000-000000000001','20000000-0000-0000-0000-000000000003','标题','正文','approved');
DO $$ BEGIN
 IF (SELECT count(*) FROM forum_outbox WHERE post_id='20000000-0000-0000-0000-000000000041')<>1 THEN RAISE EXCEPTION 'v2 missing'; END IF;
 IF EXISTS(SELECT 1 FROM forum_outbox WHERE post_id='20000000-0000-0000-0000-000000000041' AND (schema_version<>2 OR request_body IS NULL OR payload ? 'external_organization_id' OR payload ? 'school_id' OR route_id<>'m1' OR body_sha256<>encode(sha256(convert_to(request_body,'UTF8')),'hex'))) THEN RAISE EXCEPTION 'invalid snapshot'; END IF;
END $$;
INSERT INTO posts(id,school_id,author_id,title,content,audit_status) VALUES('20000000-0000-0000-0000-000000000042','20000000-0000-0000-0000-000000000002','20000000-0000-0000-0000-000000000003','标题二','正文二','approved');
DO $$ BEGIN
 IF (SELECT count(*) FROM forum_outbox WHERE post_id='20000000-0000-0000-0000-000000000042')<>1 OR NOT EXISTS(SELECT 1 FROM forum_outbox WHERE post_id='20000000-0000-0000-0000-000000000042' AND route_id='m2') THEN RAISE EXCEPTION 'school two delivery wrong'; END IF;
END $$;
SELECT pg_temp.test_binding('20000000-0000-0000-0000-000000000099','20000000-0000-0000-0000-000000000001','20000000-0000-0000-0000-000000000011','20000000-0000-0000-0000-000000000021','20000000-0000-0000-0000-000000000101','https://keeper.example.com','m9',true);
DO $$ BEGIN
 IF (SELECT count(*) FROM school_source_bindings WHERE school_id='20000000-0000-0000-0000-000000000001')<>1 OR NOT EXISTS(SELECT 1 FROM school_source_bindings WHERE id='20000000-0000-0000-0000-000000000031' AND route_id='m9' AND revision=2) THEN RAISE EXCEPTION 'school route was appended instead of updated'; END IF;
 IF NOT EXISTS(SELECT 1 FROM school_source_bindings WHERE school_id='20000000-0000-0000-0000-000000000002' AND route_id='m2') THEN RAISE EXCEPTION 'other school route changed'; END IF;
 BEGIN PERFORM pg_temp.test_binding('20000000-0000-0000-0000-000000000099','20000000-0000-0000-0000-000000000001','20000000-0000-0000-0000-000000000011','20000000-0000-0000-0000-000000000021','20000000-0000-0000-0000-000000000101','https://keeper.example.com','m2',true); RAISE EXCEPTION 'route stolen from other school'; EXCEPTION WHEN unique_violation THEN NULL; END;
END $$;
DO $$ BEGIN IF (SELECT route_id FROM forum_outbox WHERE post_id='20000000-0000-0000-0000-000000000041')<>'m1' THEN RAISE EXCEPTION 'frozen route changed'; END IF; END $$;
UPDATE schools SET active=false WHERE id='20000000-0000-0000-0000-000000000001';
DO $$ BEGIN IF NOT EXISTS(SELECT 1 FROM forum_outbox WHERE payload->>'event_type'='content.withdrawn' AND schema_version=2 AND payload->>'content' IS NULL) THEN RAISE EXCEPTION 'withdrawal missing'; END IF; END $$;
ROLLBACK;
