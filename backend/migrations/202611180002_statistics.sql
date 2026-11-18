-- Statistics facts are independent of current account/school bindings.
CREATE TABLE statistics_registrations (
 account_id uuid PRIMARY KEY REFERENCES accounts(id), registered_at timestamptz NOT NULL,
 registration_school_id uuid REFERENCES schools(id),
 school_assignment text NOT NULL CHECK(school_assignment IN ('known_school','unassigned','unknown')),
 time_quality text NOT NULL CHECK(time_quality IN ('verified','unknown')),
 recorded_at timestamptz NOT NULL DEFAULT now(),
 CHECK((school_assignment='known_school')=(registration_school_id IS NOT NULL))
);
INSERT INTO statistics_registrations(account_id,registered_at,school_assignment,time_quality)
 SELECT account_id,created_at,'unknown','verified' FROM miniapp_users;
CREATE FUNCTION statistics_capture_registration() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 INSERT INTO statistics_registrations(account_id,registered_at,registration_school_id,school_assignment,time_quality)
 SELECT NEW.account_id,NEW.created_at,a.bound_school_id,
 CASE WHEN a.bound_school_id IS NULL THEN 'unassigned' ELSE 'known_school' END,'verified'
 FROM accounts a WHERE a.id=NEW.account_id;
 RETURN NEW;
END $$;
CREATE TRIGGER statistics_registration AFTER INSERT ON miniapp_users
 FOR EACH ROW EXECUTE FUNCTION statistics_capture_registration();
CREATE INDEX statistics_registration_time ON statistics_registrations(registered_at,account_id);
CREATE INDEX statistics_registration_unknown ON statistics_registrations(registered_at) WHERE school_assignment='unknown';
CREATE INDEX statistics_registration_school ON statistics_registrations(registration_school_id,registered_at,account_id);
CREATE TABLE statistics_activity_events (
 event_id uuid PRIMARY KEY, account_id uuid NOT NULL REFERENCES accounts(id),
 occurred_at timestamptz NOT NULL, school_id uuid REFERENCES schools(id), post_id uuid REFERENCES posts(id),
 kind text NOT NULL CHECK(kind IN ('login','feed_open','post_open','post_create','comment_create')),
 client_kind text NOT NULL DEFAULT 'miniapp' CHECK(client_kind='miniapp'),
 recorded_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX statistics_activity_time ON statistics_activity_events(occurred_at,account_id);
CREATE INDEX statistics_activity_school ON statistics_activity_events(school_id,occurred_at,account_id);
CREATE TABLE statistics_coverage (
 id uuid PRIMARY KEY, family text NOT NULL CHECK(family IN ('content','registration','activity')),
 start_at timestamptz NOT NULL, end_at timestamptz,
 status text NOT NULL CHECK(status IN ('complete','partial','unknown')),
 reason text NOT NULL, updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK(end_at IS NULL OR end_at>=start_at)
);
-- Only observations after installation are guaranteed; historical coverage must be verified explicitly.
INSERT INTO statistics_coverage VALUES
 (gen_random_uuid(),'content',now(),NULL,'complete','installation',now()),
 (gen_random_uuid(),'registration',now(),NULL,'complete','installation',now());
CREATE TABLE statistics_activity_runs (
 id uuid PRIMARY KEY, started_at timestamptz NOT NULL, sealed_at timestamptz NOT NULL,
 heartbeat_at timestamptz NOT NULL, closed_at timestamptz, failed boolean NOT NULL DEFAULT false
);
CREATE TABLE statistics_snapshots (
 id uuid PRIMARY KEY, owner_id uuid NOT NULL REFERENCES accounts(id), school_id uuid REFERENCES schools(id),
 as_of timestamptz NOT NULL, expires_at timestamptz NOT NULL,
 definition_version integer NOT NULL, result jsonb NOT NULL
);
CREATE INDEX statistics_snapshot_expiry ON statistics_snapshots(expires_at);
CREATE INDEX statistics_posts_time ON posts(school_id,created_at);
CREATE INDEX statistics_comments_time ON comments(school_id,created_at);
CREATE INDEX statistics_posts_author_time ON posts(author_id,created_at,school_id);
CREATE INDEX statistics_comments_author_time ON comments(author_id,created_at,school_id);
