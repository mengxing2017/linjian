CREATE TABLE legal_document_catalog (id boolean PRIMARY KEY DEFAULT true CHECK (id), revision bigint NOT NULL DEFAULT 0);
INSERT INTO legal_document_catalog DEFAULT VALUES;
CREATE TABLE legal_documents (
    kind text PRIMARY KEY CHECK (kind IN ('user_agreement','privacy_policy')),
    revision bigint NOT NULL DEFAULT 0 CHECK (revision >= 0),
    content text CHECK (char_length(content) BETWEEN 1 AND 50000),
    updated_at timestamptz NOT NULL DEFAULT now(),
    updated_by uuid REFERENCES accounts(id)
);
INSERT INTO legal_documents(kind) VALUES ('user_agreement'),('privacy_policy');
CREATE TABLE legal_document_revisions (
    kind text NOT NULL REFERENCES legal_documents(kind),
    revision bigint NOT NULL,
    content text,
    updated_at timestamptz NOT NULL DEFAULT now(),
    updated_by uuid NOT NULL REFERENCES accounts(id),
    PRIMARY KEY(kind,revision)
);
CREATE TABLE miniapp_onboarding (
    account_id uuid PRIMARY KEY REFERENCES accounts(id),
    completed_at timestamptz
);
CREATE FUNCTION protect_legal_document_revision() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'legal document revisions are immutable';
END;
$$;
CREATE TRIGGER legal_document_revision_immutable BEFORE UPDATE OR DELETE ON legal_document_revisions
FOR EACH ROW EXECUTE FUNCTION protect_legal_document_revision();
CREATE TABLE miniapp_legal_acceptances (
    account_id uuid NOT NULL REFERENCES accounts(id),
    kind text NOT NULL,
    revision bigint NOT NULL,
    accepted_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY(account_id,kind,revision),
    FOREIGN KEY(kind,revision) REFERENCES legal_document_revisions(kind,revision)
);
CREATE TABLE miniapp_onboarding_attempts (
    token_hash text PRIMARY KEY,
    app_id text NOT NULL,
    openid text NOT NULL,
    account_id uuid REFERENCES accounts(id),
    expires_at timestamptz NOT NULL DEFAULT now()+interval '10 minutes',
    cancelled boolean NOT NULL DEFAULT false,
    completed boolean NOT NULL DEFAULT false,
    completion_key text,
    completion_school uuid REFERENCES schools(id),
    result bytea,
    result_expires_at timestamptz
);
CREATE INDEX miniapp_onboarding_expiry ON miniapp_onboarding_attempts(expires_at);
DELETE FROM sessions WHERE kind='miniapp';
