UPDATE miniapp_school_change_documents d
 SET purge_after = LEAST(COALESCE(d.purge_after, r.closed_at), r.closed_at),
     retry_at = LEAST(d.retry_at, now())
 FROM miniapp_school_change_requests r
 WHERE d.request_id = r.id AND r.status = 'approved'
   AND d.state IN ('ready', 'uploading', 'deleting');
