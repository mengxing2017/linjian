CREATE TABLE site_settings (
 id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
 singleton boolean NOT NULL DEFAULT true UNIQUE CHECK(singleton),
 revision bigint NOT NULL DEFAULT 0 CHECK(revision>=0),
 site_name text,
 title_subtitle text,
 login_intro text,
 icp_number text,
 police_number text,
 police_url text,
 copyright_owner text,
 copyright_start_year integer,
 copyright_end_mode text CHECK(copyright_end_mode IN ('auto','fixed')),
 copyright_end_year integer,
 copyright_notice text,
 updated_by uuid REFERENCES accounts(id),
 updated_at timestamptz NOT NULL DEFAULT now(),
 CHECK(site_name IS NULL OR char_length(btrim(site_name)) BETWEEN 1 AND 30),
 CHECK(title_subtitle IS NULL OR char_length(btrim(title_subtitle)) BETWEEN 1 AND 40),
 CHECK(login_intro IS NULL OR char_length(btrim(login_intro)) BETWEEN 1 AND 100),
 CHECK(icp_number IS NULL OR char_length(btrim(icp_number)) BETWEEN 1 AND 64),
 CHECK(police_number IS NULL OR char_length(btrim(police_number)) BETWEEN 1 AND 64),
 CHECK(police_url IS NULL OR (char_length(police_url) BETWEEN 1 AND 2048 AND police_url LIKE 'https://%')),
 CHECK((police_number IS NULL)=(police_url IS NULL)),
 CHECK(copyright_owner IS NULL OR char_length(btrim(copyright_owner)) BETWEEN 1 AND 100),
 CHECK(copyright_notice IS NULL OR char_length(btrim(copyright_notice)) BETWEEN 1 AND 200),
 CHECK(copyright_start_year IS NULL OR copyright_start_year BETWEEN 1900 AND 9999),
 CHECK(copyright_end_year IS NULL OR copyright_end_year BETWEEN 1900 AND 9999),
 CHECK(
  (copyright_owner IS NULL AND copyright_start_year IS NULL AND copyright_end_mode IS NULL
   AND copyright_end_year IS NULL AND copyright_notice IS NULL)
  OR
  (copyright_owner IS NOT NULL AND copyright_start_year IS NOT NULL AND copyright_end_mode IS NOT NULL)
 ),
 CHECK(copyright_end_mode IS DISTINCT FROM 'fixed' OR copyright_end_year IS NOT NULL),
 CHECK(copyright_end_mode IS DISTINCT FROM 'auto' OR copyright_end_year IS NULL),
 CHECK(copyright_end_year IS NULL OR copyright_start_year IS NULL OR copyright_end_year>=copyright_start_year)
);

INSERT INTO site_settings(singleton) VALUES(true);

CREATE INDEX IF NOT EXISTS site_audit_history
 ON audit_events(action,object_id,created_at DESC);
