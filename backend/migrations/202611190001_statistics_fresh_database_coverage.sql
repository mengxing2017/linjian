-- Linjian starts with an independent empty database. Content and registration
-- facts are complete from the outset; periods before installation contain zero.
-- Keep explicit partial/unknown intervals so real data gaps remain visible.
UPDATE statistics_coverage
SET start_at = '1970-01-01T00:00:00Z', reason = 'fresh_database', updated_at = now()
WHERE family IN ('content', 'registration') AND status = 'complete'
  AND reason = 'installation' AND end_at IS NULL;

-- Reports computed under the installation-time assumption must be refreshed.
DELETE FROM statistics_snapshots;
