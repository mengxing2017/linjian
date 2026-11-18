-- Unreleased project: remove unused integration, no identity or session conversion.
ALTER TABLE accounts DROP COLUMN wecom_member_id, DROP COLUMN wecom_binding_revision;
ALTER TABLE sessions DROP COLUMN wecom_authenticated, DROP COLUMN wecom_binding_revision, DROP COLUMN credential_generation;
DROP TABLE wecom_directory_sync_runs;
DROP TABLE wecom_members;
DROP TABLE wecom_departments;
DROP TABLE wecom_directory_state;
DROP TABLE oauth_states;
ALTER TABLE sessions DROP CONSTRAINT sessions_kind_check;
ALTER TABLE sessions ADD CONSTRAINT sessions_kind_check CHECK(kind IN ('admin_browser','miniapp'));
ALTER TABLE sessions ADD COLUMN login_origin text;
ALTER TABLE sessions ADD CONSTRAINT sessions_client_origin CHECK (
 (kind='admin_browser' AND login_origin IS NOT NULL AND length(login_origin)>0)
 OR (kind='miniapp' AND login_origin IS NULL)
);
-- Legacy content URL projections were removed by 202611160002.
ALTER TABLE source_connection_versions DROP COLUMN public_origin;
