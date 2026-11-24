DO $$
DECLARE
    catalog jsonb;
BEGIN
    SELECT jsonb_agg(
        CASE WHEN updated.description IS NULL THEN existing.permission
             ELSE jsonb_set(existing.permission, '{description}', to_jsonb(updated.description), false)
        END ORDER BY existing.ordinality
    )
    INTO catalog
    FROM jsonb_array_elements(forum_permission_catalog()) WITH ORDINALITY AS existing(permission, ordinality)
    LEFT JOIN (VALUES
        ('post.recommend', '允许预览并提交帖子推荐；帖子仍需满足发布要求、操作频率限制和接收方配置要求，系统会检查重复提交'),
        ('statistics.school.read', '查看本校统计和已生成且有权查看的报告；不包含全站汇总或跨校比较'),
        ('statistics.school.export', '将已生成的本校统计报告导出为 CSV，下载时会再次检查查看和导出权限')
    ) AS updated(permission_key, description)
        ON existing.permission->>'key' = updated.permission_key;

    EXECUTE format(
        'CREATE OR REPLACE FUNCTION forum_permission_catalog() RETURNS jsonb LANGUAGE sql IMMUTABLE AS %L',
        format('SELECT %L::jsonb', catalog::text)
    );
END;
$$;
