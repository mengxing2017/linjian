ALTER TABLE miniapp_school_change_requests
 ADD COLUMN student_relationship text CHECK(student_relationship IN ('enrolled','new_student'));

CREATE INDEX miniapp_school_change_review_deadline
 ON miniapp_school_change_requests(review_due_at,id) WHERE status='pending';

ALTER TABLE miniapp_school_change_requests
 DROP CONSTRAINT miniapp_school_change_requests_check2,
 ADD CONSTRAINT miniapp_school_change_review_outcome CHECK (
   (status IN ('approved','rejected') AND reviewed_by IS NOT NULL AND reviewed_at IS NOT NULL AND result_reason_code IS DISTINCT FROM 'review_timeout')
   OR (status='rejected' AND reviewed_by IS NULL AND reviewed_at IS NULL AND result_reason_code IS NOT DISTINCT FROM 'review_timeout')
   OR (status NOT IN ('approved','rejected') AND reviewed_by IS NULL AND reviewed_at IS NULL AND result_reason_code IS DISTINCT FROM 'review_timeout')
 );
