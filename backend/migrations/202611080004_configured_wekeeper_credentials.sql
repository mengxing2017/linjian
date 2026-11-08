-- Signing material lives only in backend deployment configuration.
-- Keep connection IDs/versions and historical outbox ownership unchanged.
ALTER TABLE source_connections DROP COLUMN encrypted_secret, DROP COLUMN encryption_version;
