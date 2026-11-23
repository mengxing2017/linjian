CREATE TABLE miniapp_phone_bindings (
    account_id uuid PRIMARY KEY REFERENCES miniapp_users(account_id) ON DELETE CASCADE,
    status text NOT NULL CHECK (status IN ('verified','revoked')),
    phone_ciphertext bytea,
    key_version text,
    country_code text,
    phone_last4 text,
    app_id text,
    verified_at timestamptz,
    notice_version text,
    revision bigint NOT NULL CHECK (revision > 0),
    updated_at timestamptz NOT NULL DEFAULT now(),
    CHECK (status <> 'verified' OR (
        phone_ciphertext IS NOT NULL AND octet_length(phone_ciphertext) > 28
        AND key_version IS NOT NULL AND country_code IS NOT NULL AND country_code = '86'
        AND phone_last4 IS NOT NULL AND phone_last4 ~ '^[0-9]{4}$' AND app_id IS NOT NULL
        AND verified_at IS NOT NULL AND notice_version IS NOT NULL
    )),
    CHECK (status <> 'revoked' OR (
        phone_ciphertext IS NULL AND key_version IS NULL AND country_code IS NULL
        AND phone_last4 IS NULL AND app_id IS NULL AND verified_at IS NULL AND notice_version IS NULL
    ))
);

CREATE TABLE miniapp_phone_binding_requests (
    account_id uuid NOT NULL REFERENCES miniapp_users(account_id) ON DELETE CASCADE,
    request_id uuid NOT NULL,
    fingerprint text NOT NULL,
    status text NOT NULL CHECK (status IN ('processing','completed','failed','uncertain')),
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (account_id,request_id)
);
CREATE INDEX miniapp_phone_binding_requests_created ON miniapp_phone_binding_requests(created_at);
