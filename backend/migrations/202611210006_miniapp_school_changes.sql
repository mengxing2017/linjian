CREATE TABLE miniapp_school_change_requests (
 id uuid PRIMARY KEY,
 account_id uuid NOT NULL REFERENCES miniapp_users(account_id),
 from_school_id uuid NOT NULL REFERENCES schools,
 to_school_id uuid NOT NULL REFERENCES schools,
 declared_student_name text,
 declared_student_number text,
 reason text,
 status text NOT NULL DEFAULT 'pending' CHECK(status IN ('pending','approved','rejected','cancelled','invalidated')),
 revision bigint NOT NULL DEFAULT 1 CHECK(revision>0),
 submission_key uuid NOT NULL,
 submission_digest text NOT NULL,
 consent_version text NOT NULL,
 consent_notice text NOT NULL,
 review_guidance text NOT NULL,
 consented_at timestamptz NOT NULL DEFAULT now(),
 submitted_at timestamptz NOT NULL DEFAULT now(),
 review_due_at timestamptz NOT NULL,
 reviewed_by uuid REFERENCES accounts,
 reviewed_at timestamptz,
 closed_at timestamptz,
 result_reason_code text,
 result_reason text,
 document_retention_seconds bigint NOT NULL CHECK(document_retention_seconds>0),
 personal_retention_seconds bigint NOT NULL CHECK(personal_retention_seconds>0),
 audit_retention_seconds bigint NOT NULL CHECK(audit_retention_seconds>0),
 personal_data_purged_at timestamptz,
 UNIQUE(account_id,id),
 UNIQUE(account_id,submission_key),
 CHECK(from_school_id<>to_school_id),
 CHECK((status='pending')=(closed_at IS NULL)),
 CHECK((status IN ('approved','rejected'))=(reviewed_by IS NOT NULL AND reviewed_at IS NOT NULL)),
 CHECK((reviewed_by IS NULL)=(reviewed_at IS NULL)),
 CHECK(reviewed_by IS NULL OR reviewed_by<>account_id)
);
CREATE UNIQUE INDEX miniapp_school_change_pending ON miniapp_school_change_requests(account_id) WHERE status='pending';
CREATE INDEX miniapp_school_change_recent ON miniapp_school_change_requests(account_id,submitted_at DESC,id DESC);
CREATE INDEX miniapp_school_change_queue ON miniapp_school_change_requests(status,submitted_at,id);
CREATE TABLE miniapp_school_change_documents (
 id uuid PRIMARY KEY,
 owner_id uuid NOT NULL REFERENCES miniapp_users(account_id),
 request_id uuid,
 state text NOT NULL CHECK(state IN ('uploading','ready','deleting','deleted')),
 purpose text NOT NULL DEFAULT 'school_change' CHECK(purpose='school_change'),
 mime text NOT NULL,
 size_bytes bigint NOT NULL CHECK(size_bytes>0),
 sha256 text NOT NULL,
 store_id uuid NOT NULL,
 created_at timestamptz NOT NULL DEFAULT now(),
 expires_at timestamptz,
 purge_after timestamptz,
 deleted_at timestamptz,
 attempts integer NOT NULL DEFAULT 0,
 retry_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(owner_id,request_id) REFERENCES miniapp_school_change_requests(account_id,id),
 CHECK(request_id IS NULL OR expires_at IS NULL)
);
CREATE INDEX miniapp_school_change_document_owner ON miniapp_school_change_documents(owner_id,created_at DESC);
CREATE INDEX miniapp_school_change_document_request ON miniapp_school_change_documents(request_id);
CREATE INDEX miniapp_school_change_document_cleanup ON miniapp_school_change_documents(state,retry_at);
CREATE TABLE miniapp_school_change_events (
 id uuid PRIMARY KEY,
 request_id uuid REFERENCES miniapp_school_change_requests,
 document_id uuid REFERENCES miniapp_school_change_documents,
 actor_id uuid REFERENCES accounts,
 action text NOT NULL,
 reason_code text,
 created_at timestamptz NOT NULL DEFAULT now(),
 purge_after timestamptz NOT NULL
);
CREATE INDEX miniapp_school_change_events_request ON miniapp_school_change_events(request_id,created_at,id);
CREATE INDEX miniapp_school_change_events_purge ON miniapp_school_change_events(purge_after);
