CREATE OR REPLACE FUNCTION linjian_public_post(item posts) RETURNS jsonb LANGUAGE sql STABLE AS $$
 SELECT jsonb_build_object('id',item.id,'school_id',item.school_id,'author_id',item.author_id,'title',item.title,'content',item.content,'images',item.images,'tags',item.tags,'audit_status',item.audit_status,'visibility',item.visibility,'browse',item.browse,'revision',item.revision,'display_published_at',item.display_published_at)
$$;
