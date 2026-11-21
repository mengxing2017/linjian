INSERT INTO application_metadata (singleton, schema_generation) VALUES (true, 1);
INSERT INTO site_settings (singleton) VALUES (true);
INSERT INTO miniapp_registration_settings (id, nickname_prefix) VALUES (1, '用户');
INSERT INTO source_active_configuration (singleton) VALUES (true);
INSERT INTO statistics_coverage (id, family, start_at, status, reason) VALUES
    (gen_random_uuid(), 'content', '1970-01-01T00:00:00Z', 'complete', 'fresh_database'),
    (gen_random_uuid(), 'registration', '1970-01-01T00:00:00Z', 'complete', 'fresh_database');
