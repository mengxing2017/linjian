school_id=$1 AND account_id=$2
AND (
    n.kind NOT IN ('comment','moderation')
    OR EXISTS (
        SELECT 1 FROM posts p
        WHERE p.id=n.object_id AND p.school_id=n.school_id
          AND (p.author_id=$2 OR (p.audit_status='approved' AND p.visibility='visible'))
    )
    OR EXISTS (
        SELECT 1 FROM comments c JOIN posts p ON p.id=c.post_id
        WHERE c.id=n.object_id AND c.school_id=n.school_id
          AND NOT c.deleted AND c.visibility='visible'
          AND p.audit_status='approved' AND p.visibility='visible'
          AND (c.audit_status='approved' OR c.author_id=$2)
    )
)
