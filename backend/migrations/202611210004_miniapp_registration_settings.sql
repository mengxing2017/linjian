CREATE TABLE miniapp_registration_settings (
 id smallint PRIMARY KEY CHECK(id=1),
 nickname_prefix text NOT NULL CHECK(nickname_prefix=btrim(nickname_prefix) AND char_length(nickname_prefix) BETWEEN 1 AND 91),
 revision bigint NOT NULL DEFAULT 0 CHECK(revision>=0),
 updated_at timestamptz NOT NULL DEFAULT now(),
 updated_by uuid REFERENCES accounts ON DELETE SET NULL
);
INSERT INTO miniapp_registration_settings(id,nickname_prefix) VALUES(1,'用户');
CREATE INDEX miniapp_users_nickname_lookup ON miniapp_users(nickname);
