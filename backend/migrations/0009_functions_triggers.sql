CREATE FUNCTION forum_bind_prepared_source(event uuid, cv uuid, bv uuid, route text) RETURNS void
    LANGUAGE plpgsql
    AS $$
DECLARE o forum_outbox; v source_connection_versions;
BEGIN
 SELECT * INTO STRICT o FROM forum_outbox WHERE id=event FOR UPDATE;
 IF o.connection_version IS NOT NULL THEN
  IF (o.connection_version,o.binding_version,o.route_id) IS DISTINCT FROM (cv,bv,route) THEN RAISE EXCEPTION 'source binding is immutable'; END IF;
  RETURN;
 END IF;
 SELECT * INTO STRICT v FROM source_connection_versions WHERE id=cv;
 IF NOT EXISTS(SELECT 1 FROM school_source_binding_versions WHERE id=bv AND school_id=o.school_id AND connection_version=cv AND route_id=route) THEN RAISE EXCEPTION 'invalid school binding'; END IF;
 UPDATE forum_outbox SET connection_version=cv,binding_version=bv,source_id=v.source_id,source_origin=v.origin,route_id=route,
 delivery_state='preparing_content' WHERE id=event;
END $$;

CREATE FUNCTION forum_binding_history() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 INSERT INTO school_source_binding_versions(binding_id,school_id,revision,connection_version,source_id,origin,route_id,active,checked_at,actor_id)
 VALUES(NEW.id,NEW.school_id,NEW.revision,NEW.connection_version,NEW.source_id,NEW.origin,NEW.route_id,NEW.active,NEW.checked_at,nullif(current_setting('linjian.source_actor_id',true),'')::uuid);
 IF TG_OP='UPDATE' AND (OLD.origin,OLD.source_id,OLD.route_id) IS DISTINCT FROM (NEW.origin,NEW.source_id,NEW.route_id) THEN
  UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=NEW.school_id AND connection_version IS NOT NULL AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
 END IF;
 UPDATE forum_outbox SET delivery_state=CASE WHEN NOT NEW.active THEN 'paused' WHEN request_body IS NULL THEN 'preparing_content' WHEN last_error IS NULL THEN 'ready' ELSE 'retry_wait' END
 WHERE school_id=NEW.school_id AND connection_version IS NOT NULL AND receipt_id IS NULL AND attempts=0 AND lease_token IS NULL AND delivery_state IN ('ready','retry_wait','paused','preparing_content');
 RETURN NEW;
END $$;

CREATE FUNCTION forum_connection_version_key() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF NEW.key_id IS NULL THEN SELECT key_id INTO NEW.key_id FROM source_connections WHERE id=NEW.connection_id; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_create_school_authorization_guard() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 INSERT INTO school_authorization_guards(school_id) VALUES(NEW.id);
 RETURN NEW;
END $$;


SET default_tablespace = '';

SET default_table_access_method = heap;

CREATE FUNCTION forum_decision_details(school uuid, object uuid, kind text, version bigint, status text) RETURNS jsonb
    LANGUAGE sql STABLE
    AS $$
 SELECT jsonb_build_object('decision_source',coalesce(m.decision_source,CASE WHEN status<>'pending' THEN 'legacy' END),
  'decided_at',m.created_at,'moderator_id',m.actor_id)
 FROM (SELECT 1) dummy LEFT JOIN LATERAL (
  SELECT * FROM moderation_events WHERE school_id=school AND object_id=object AND object_kind=kind
   AND (content_revision=version OR (content_revision IS NULL AND version=1)) AND decision=status
  ORDER BY created_at DESC,id DESC LIMIT 1
 ) m ON true
$$;

CREATE FUNCTION forum_enqueue_school() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF OLD.active AND NOT NEW.active THEN
  UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=NEW.id AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
  UPDATE forum_source_versions SET available=false WHERE school_id=NEW.id;
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_events_immutable() RETURNS trigger
    LANGUAGE plpgsql
    AS $$ BEGIN RAISE EXCEPTION 'immutable forum event'; END $$;

CREATE FUNCTION forum_guard_bound_preparation() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF (NEW.projection_version,NEW.publication_snapshot_id) IS DISTINCT FROM (OLD.projection_version,OLD.publication_snapshot_id) THEN RAISE EXCEPTION 'publication intent is immutable'; END IF;
 IF OLD.connection_version IS NOT NULL AND (NEW.connection_version,NEW.binding_version,NEW.source_id,NEW.source_origin,NEW.route_id) IS DISTINCT FROM (OLD.connection_version,OLD.binding_version,OLD.source_id,OLD.source_origin,OLD.route_id) THEN RAISE EXCEPTION 'publication binding is immutable'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_guard_decision() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN RAISE EXCEPTION 'moderation evidence is immutable'; END IF;
 IF NEW.decision_source='legacy' THEN RAISE EXCEPTION 'new legacy decisions are forbidden'; END IF;
 IF NEW.object_kind NOT IN ('post','comment') OR NEW.decision NOT IN ('pending','approved','rejected') THEN RAISE EXCEPTION 'invalid moderation decision'; END IF;
 IF NEW.decision_source='human' AND NOT EXISTS(
  SELECT 1 FROM accounts a JOIN schools s ON s.id=NEW.school_id AND s.active
  WHERE a.id=NEW.actor_id AND a.active AND (a.platform_admin OR EXISTS(
   SELECT 1 FROM school_memberships m WHERE m.account_id=a.id AND m.school_id=s.id AND m.role IN ('admin','operator')))
 ) THEN RAISE EXCEPTION 'human decision requires a school moderator'; END IF;
 IF NEW.decision_source='automatic' AND NOT EXISTS(
  SELECT 1 FROM schools WHERE id=NEW.school_id AND active AND revision=NEW.policy_revision
   AND config->>(CASE WHEN NEW.object_kind='post' THEN 'post_audit_mode' ELSE 'comment_audit_mode' END)='auto'
 ) THEN RAISE EXCEPTION 'automatic decision requires active school policy'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_guard_frozen_source() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF OLD.request_body IS NOT NULL AND (NEW.id,NEW.school_id,NEW.post_id,NEW.sequence,NEW.payload,NEW.connection_version,NEW.binding_version,NEW.source_id,NEW.source_origin,NEW.route_id,NEW.schema_version,NEW.signature_version,NEW.request_body,NEW.body_sha256,NEW.frozen_at)
 IS DISTINCT FROM (OLD.id,OLD.school_id,OLD.post_id,OLD.sequence,OLD.payload,OLD.connection_version,OLD.binding_version,OLD.source_id,OLD.source_origin,OLD.route_id,OLD.schema_version,OLD.signature_version,OLD.request_body,OLD.body_sha256,OLD.frozen_at) THEN
 RAISE EXCEPTION 'frozen source identity is immutable';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_guard_message_version() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF OLD.request_body IS NOT NULL AND NEW.message_version IS DISTINCT FROM OLD.message_version THEN RAISE EXCEPTION 'frozen message version is immutable'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_guard_new_snapshot() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF NEW.decision_source='legacy' THEN RAISE EXCEPTION 'new legacy snapshots are forbidden'; END IF;
 IF NOT EXISTS(
  SELECT 1 FROM posts p JOIN moderation_events m ON m.id=NEW.moderation_event_id JOIN schools s ON s.id=p.school_id AND s.active
  WHERE p.id=NEW.post_id AND p.school_id=NEW.school_id AND p.author_id=NEW.author_id
   AND p.content_revision=NEW.content_revision AND p.revision+1=NEW.approved_post_revision
   AND p.title=NEW.title AND p.content=NEW.content AND p.images=NEW.images AND p.tags=NEW.tags
   AND m.school_id=p.school_id AND m.object_id=p.id AND m.object_kind='post' AND m.decision='approved'
   AND m.content_revision=p.content_revision AND m.decision_source=NEW.decision_source
   AND ((NEW.decision_source='human' AND m.actor_id=NEW.moderator_id AND m.review_digest IS NOT NULL)
    OR (NEW.decision_source='automatic' AND m.actor_id IS NULL AND NEW.moderator_id IS NULL
     AND m.policy_revision=NEW.policy_revision AND s.revision=NEW.policy_revision AND s.config->>'post_audit_mode'='auto'))
 ) THEN RAISE EXCEPTION 'snapshot requires matching current decision and content'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_guard_post_publication() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF TG_OP='UPDATE' THEN
  IF (NEW.created_via,NEW.source_projection) IS DISTINCT FROM (OLD.created_via,OLD.source_projection) THEN RAISE EXCEPTION 'post provenance is immutable'; END IF;
  IF (NEW.title,NEW.content,NEW.images,NEW.tags) IS DISTINCT FROM (OLD.title,OLD.content,OLD.images,OLD.tags) THEN
   NEW.content_revision:=OLD.content_revision+1;
   NEW.approved_snapshot_id:=NULL;
   IF NEW.source_projection='message_v1' THEN NEW.audit_status:='pending'; END IF;
  ELSE NEW.content_revision:=OLD.content_revision; END IF;
 END IF;
 IF NEW.audit_status<>'approved' THEN NEW.approved_snapshot_id:=NULL; END IF;
 IF NEW.source_projection='message_v1' AND NEW.audit_status='approved' AND NOT EXISTS(
  SELECT 1 FROM forum_publication_snapshots p JOIN moderation_events m ON m.id=p.moderation_event_id
  WHERE p.id=NEW.approved_snapshot_id AND p.school_id=NEW.school_id AND p.post_id=NEW.id
   AND p.content_revision=NEW.content_revision AND p.title=NEW.title AND p.content=NEW.content AND p.images=NEW.images AND p.tags=NEW.tags
   AND p.approved_post_revision<=NEW.revision
   AND (p.approved_post_revision=NEW.revision OR (TG_OP='UPDATE' AND OLD.approved_snapshot_id=NEW.approved_snapshot_id))
   AND m.school_id=NEW.school_id AND m.object_id=NEW.id AND m.object_kind='post' AND m.decision='approved'
   AND p.decision_source=m.decision_source
   AND ((p.decision_source='legacy' AND m.actor_id=p.moderator_id)
     OR (p.decision_source='human' AND m.actor_id=p.moderator_id AND m.content_revision=NEW.content_revision AND m.review_digest IS NOT NULL)
     OR (p.decision_source='automatic' AND m.actor_id IS NULL AND p.moderator_id IS NULL
       AND m.content_revision=NEW.content_revision AND m.policy_revision=p.policy_revision AND m.policy_mode='auto'))
 ) THEN RAISE EXCEPTION 'approved post requires matching publication evidence'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_guard_publication_snapshot() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN RAISE EXCEPTION 'publication evidence is immutable'; END $$;

CREATE FUNCTION forum_guard_source_trigger() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF NEW.trigger_type IS DISTINCT FROM OLD.trigger_type THEN RAISE EXCEPTION 'source trigger is immutable'; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_guard_source_version() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN RAISE EXCEPTION 'historical source versions are immutable'; END $$;

CREATE FUNCTION forum_invalidate_preparation() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF NEW.delivery_state='cancelled' THEN
  UPDATE forum_publication_preparations SET state='superseded',revision=revision+1,lease_token=NULL,lease_until=NULL,last_error_code='cancelled'
  WHERE event_id=NEW.id AND state NOT IN ('completed','superseded');
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_permission_catalog() RETURNS jsonb
    LANGUAGE sql IMMUTABLE
    AS $$
 SELECT '[{"id":"P01","key":"content.post.read","name":"查看帖子","description":"管理列表、筛选、详情、待审/隐藏内容、审核记录及授权附件；推荐记录只读，不含重试/终止 Outbox","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P02","key":"content.post.review","name":"审核帖子","description":"通过、驳回帖子；不含修改正文或上下架","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P03","key":"content.post.edit","name":"编辑帖子","description":"修改已有本校帖子的正文、标题、图片与标签；不含平台代发/导入","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P04","key":"content.post.visibility","name":"帖子上架/下架","description":"改变帖子展示状态；不替代审核，不包含删除","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P05","key":"content.post.pin","name":"帖子置顶与排序","description":"置顶、取消置顶及帖子管理排序","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P06","key":"content.post.delete","name":"删除帖子","description":"删除已有帖子；不授予恢复或清库能力","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":true},{"id":"P07","key":"post.recommend","name":"推荐/重推帖子","description":"推荐预览、确认提交；仍校验发布证据、频控、幂等和来源就绪条件","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["content.post.read"],"high_risk":true},{"id":"P08","key":"content.comment.read","name":"查看评论","description":"评论管理列表、筛选、正文、审核记录及所属帖子最小上下文","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P09","key":"content.comment.review","name":"审核评论","description":"通过、驳回评论","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":false},{"id":"P10","key":"content.comment.visibility","name":"评论显示/隐藏","description":"隐藏、恢复展示；不等于审核或删除","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":false},{"id":"P11","key":"content.comment.delete","name":"删除评论","description":"管理员删除本校评论；运营获授权也可执行；不影响社区删除本人评论规则","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":true},{"id":"P12","key":"content.case.read","name":"查看举报与反馈工单","description":"case 列表、详情、联系信息及证据；不附赠目标帖子完整管理读取","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P13","key":"content.case.resolve","name":"处理举报与反馈工单","description":"resolved/dismissed 与处理说明；不包含删除工单、直接删帖或封禁","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.case.read"],"high_risk":false},{"id":"P14","key":"operation.qrcode.read","name":"查看群二维码","description":"本校二维码列表、详情和管理图片","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P15","key":"operation.qrcode.create","name":"新增群二维码","description":"新建本校二维码，含该业务必需上传；不含平台默认二维码","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":false},{"id":"P16","key":"operation.qrcode.update","name":"编辑群二维码","description":"修改、启停、展示时机与排序字段","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":false},{"id":"P17","key":"operation.qrcode.delete","name":"删除群二维码","description":"删除本校二维码","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":true},{"id":"P18","key":"operation.banner.read","name":"查看首页广告位","description":"本校广告列表、详情、图片和已有曝光/点击汇总","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P19","key":"operation.banner.create","name":"新增首页广告位","description":"新建本校广告和必需上传；只可选择本校已发布可见帖作为落地目标","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":false},{"id":"P20","key":"operation.banner.update","name":"编辑首页广告位","description":"修改、启停、投放时间和排序字段","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":false},{"id":"P21","key":"operation.banner.delete","name":"删除首页广告位","description":"删除本校广告","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":true},{"id":"P22","key":"operation.feedback.read","name":"查看投诉反馈","description":"运营投诉列表、详情及授权图片；与 case 工单区分","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P23","key":"operation.feedback.delete","name":"删除投诉反馈","description":"删除运营投诉记录；不提供 case 删除或用户封禁","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.feedback.read"],"high_risk":true},{"id":"P24","key":"resource.school.read","name":"查看学校展示资源","description":"服务/法律等现有学校资源与管理素材；不授予平台资源编辑","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P25","key":"resource.school.write","name":"配置并发布学校展示资源","description":"现有预览/确认流程中的新增、修改、启停、排序与发布；含所需素材上传，不虚构独立删除/草稿流程","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["resource.school.read"],"high_risk":true},{"id":"P26","key":"school.home_menu.read","name":"查看首页菜单/话题","description":"标签菜单、筛选项及现有配置读取","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P27","key":"school.home_menu.write","name":"新增/编辑首页菜单","description":"标签名称、展示状态、启用状态；不包含删除、批量排序或导入","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P28","key":"school.home_menu.delete","name":"删除首页菜单项","description":"删除本校菜单标签","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":true},{"id":"P29","key":"school.home_menu.reorder","name":"调整首页菜单顺序","description":"现有菜单排序提交","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P30","key":"school.home_menu.settings.update","name":"修改首页菜单设置","description":"菜单模块现有独立设置，不等于学校审核策略","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P31","key":"school.home_menu.import","name":"导入首页菜单","description":"本项目现有导入预览及确认；不连接 wxrobot 数据库或服务","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read","school.home_menu.write"],"high_risk":false},{"id":"P32","key":"school.blocked_word.read","name":"查看屏蔽词","description":"本校词库及对本校生效的平台规则只读","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P33","key":"school.blocked_word.write","name":"新增/编辑屏蔽词","description":"本校词条、备注、启停；不改变平台词库","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":false},{"id":"P34","key":"school.blocked_word.delete","name":"删除屏蔽词","description":"删除本校词条","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":true},{"id":"P35","key":"school.blocked_word.reorder","name":"调整屏蔽词顺序","description":"本校现有排序流程","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":false},{"id":"P36","key":"school.blocked_word.import","name":"导入屏蔽词","description":"词库导入预览与确认","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read","school.blocked_word.write"],"high_risk":false},{"id":"P37","key":"school.profile.read","name":"查看学校后台资料","description":"本校后台资料；公开目录另走公开/社区规则","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P38","key":"school.profile.update","name":"修改学校展示资料","description":"学校名称、地区、图标、主题等学校可改字段；不含创建、启停、删除学校","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.profile.read"],"high_risk":false},{"id":"P39","key":"school.review_settings.update","name":"修改帖子/评论审核策略","description":"手动/自动审核设置，字段白名单严格分离","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.profile.read"],"high_risk":true},{"id":"P40","key":"school.user.read","name":"查看本校用户","description":"用户列表、详情、已有状态历史和授权头像；不包含平台范围搜索","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P41","key":"school.user.status.update","name":"封禁/解封本校用户","description":"本校用户状态修改；不含手机号纠正、学校纠正、后台账号启停","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.user.read"],"high_risk":true},{"id":"P42","key":"school.member.read","name":"查看本校后台成员与权限","description":"本校后台成员及业务权限，不含全局候选账号、密码和跨校授权","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P43","key":"school.member.recommendation.assign","name":"为运营配置推荐权限","description":"本校管理员的受限推荐授权，不能改其他权限或给本人授权","group":"学校与管理","applicable_roles":["admin"],"default_by_role":{"admin":true,"operator":false},"requires":["school.member.read","post.recommend"],"high_risk":true},{"id":"P44","key":"statistics.school.read","name":"查看本校业务统计","description":"学校统计与当前可读快照；不包含全校汇总、跨校比较","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P45","key":"statistics.school.export","name":"导出本校业务统计","description":"已有学校统计快照 CSV 导出；下载时重查","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["statistics.school.read"],"high_risk":false},{"id":"P46","key":"content.simulation.manage","name":"管理仿真发帖","description":"学校管理员及运营发布和管理仿真帖子、模拟作者与批次；不含审核或来源推送","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":true},{"id":"P47","key":"operation.shortcut.read","name":"服务入口：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P48","key":"operation.shortcut.create","name":"服务入口：创建","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P49","key":"operation.shortcut.update","name":"服务入口：更新","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P50","key":"operation.shortcut.delete","name":"服务入口：删除","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P51","key":"operation.shortcut.preview","name":"服务入口：预览","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P52","key":"operation.category.read","name":"服务分类：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P53","key":"operation.category.create","name":"服务分类：创建","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.category.read"],"high_risk":false},{"id":"P54","key":"operation.category.update","name":"服务分类：更新","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.category.read"],"high_risk":false},{"id":"P55","key":"operation.category.delete","name":"服务分类：删除","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.category.read"],"high_risk":false},{"id":"P56","key":"operation.report.read","name":"运营报表：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P57","key":"operation.report.export","name":"运营报表：导出","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":false,"operator":false},"requires":["operation.report.read"],"high_risk":true},{"id":"P58","key":"operation.marketing.read","name":"官方营销：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P59","key":"operation.marketing.update","name":"官方营销：更新","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.marketing.read"],"high_risk":false}]'::jsonb
$$;

CREATE FUNCTION forum_post_has_publication_evidence(item posts) RETURNS boolean
    LANGUAGE sql STABLE
    AS $$
 SELECT EXISTS(SELECT 1 FROM forum_publication_snapshots s JOIN moderation_events m ON m.id=s.moderation_event_id
 WHERE s.id=item.approved_snapshot_id AND s.school_id=item.school_id AND s.post_id=item.id
 AND s.content_revision=item.content_revision AND s.approved_post_revision<=item.revision
 AND s.title=item.title AND s.content=item.content AND s.images=item.images AND s.tags=item.tags
 AND s.decision_source=m.decision_source AND m.school_id=item.school_id AND m.object_id=item.id AND m.object_kind='post' AND m.decision='approved'
 AND m.content_revision=item.content_revision AND m.review_digest IS NOT NULL
 AND ((s.decision_source='human' AND m.actor_id=s.moderator_id)
 OR (s.decision_source='automatic' AND m.actor_id IS NULL AND s.moderator_id IS NULL AND m.policy_revision=s.policy_revision AND m.policy_mode='auto')))
$$;

CREATE FUNCTION forum_post_order_metadata() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF TG_OP='INSERT' THEN
  NEW.is_top:=(NEW.is_top OR NEW.top_sort>0) AND NEW.visibility<>'deleted';
 ELSIF NEW.top_sort IS DISTINCT FROM OLD.top_sort AND NEW.is_top IS NOT DISTINCT FROM OLD.is_top THEN
  NEW.is_top:=NEW.top_sort>0;
 END IF;
 IF NEW.visibility='deleted' THEN NEW.is_top:=false; END IF;
 NEW.top_sort:=CASE WHEN NEW.is_top THEN greatest(NEW.top_sort,1) ELSE 0 END;
 IF TG_OP='UPDATE' AND NEW.is_top IS DISTINCT FROM OLD.is_top THEN NEW.sort_order:=0; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_post_order_version() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF (TG_OP='INSERT' AND NEW.visibility<>'deleted') OR (TG_OP='UPDATE' AND
  ((NEW.visibility='deleted') IS DISTINCT FROM (OLD.visibility='deleted') OR
   NEW.is_top IS DISTINCT FROM OLD.is_top OR NEW.sort_order IS DISTINCT FROM OLD.sort_order)) THEN
  INSERT INTO post_order_versions(school_id,revision) VALUES(NEW.school_id,2)
   ON CONFLICT(school_id) DO UPDATE SET revision=post_order_versions.revision+1;
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION forum_publication_is_current(event uuid) RETURNS boolean
    LANGUAGE sql STABLE
    AS $$
 SELECT EXISTS(SELECT 1 FROM forum_outbox o JOIN forum_publication_snapshots snap ON snap.id=o.publication_snapshot_id
 JOIN posts p ON p.id=o.post_id AND p.school_id=o.school_id JOIN schools s ON s.id=o.school_id
 WHERE o.id=event AND o.delivery_state<>'cancelled' AND s.active AND p.audit_status='approved' AND p.visibility='visible'
 AND p.approved_snapshot_id=snap.id AND p.content_revision=snap.content_revision)
$$;

CREATE FUNCTION linjian_file_history_guard() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF OLD.ever_bound AND NOT NEW.ever_bound THEN
  RAISE EXCEPTION 'File binding history cannot be reset' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_file_protected(asset uuid) RETURNS boolean
    LANGUAGE sql
    AS $$
 SELECT EXISTS(SELECT 1 FROM file_asset_references WHERE asset_id=asset)
 OR EXISTS(SELECT 1 FROM file_asset_leases WHERE asset_id=asset AND expires_at>now())
 OR EXISTS(SELECT 1 FROM media_access_tokens WHERE asset_id=asset AND expires_at>now())
$$;

CREATE FUNCTION linjian_file_protection_guard() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE asset uuid; f file_assets;
BEGIN
 asset := CASE WHEN TG_OP='DELETE' THEN OLD.asset_id ELSE NEW.asset_id END;
 IF TG_OP='UPDATE' AND NEW.asset_id<>OLD.asset_id THEN
  RAISE EXCEPTION 'Protection asset cannot change' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
 END IF;
 SELECT * INTO STRICT f FROM file_assets WHERE id=asset FOR UPDATE;
 IF TG_OP<>'DELETE' AND NEW.expires_at>now() THEN
  IF f.state<>'ready' THEN
   RAISE EXCEPTION 'File no longer available' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
  END IF;
  IF TG_TABLE_NAME='file_asset_leases' THEN
   IF f.owner_id<>NEW.owner_id OR f.purpose<>'post_image' THEN
    RAISE EXCEPTION 'Invalid draft lease' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
   END IF;
  ELSE
   IF f.school_id IS DISTINCT FROM NEW.school_id OR f.purpose<>'post_image' OR NOT EXISTS(
    SELECT 1 FROM posts p JOIN schools s ON s.id=p.school_id WHERE p.id=NEW.post_id AND p.school_id=NEW.school_id
     AND s.active AND p.visibility='visible' AND p.audit_status='approved'
     AND p.images @> jsonb_build_array('/api/v1/schools/'||p.school_id||'/media/'||asset)) THEN
    RAISE EXCEPTION 'Invalid media token' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
   END IF;
  END IF;
 END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_guard_account_kind() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF NEW.account_kind IS DISTINCT FROM OLD.account_kind OR (OLD.account_kind='simulated' AND NEW.bound_school_id IS DISTINCT FROM OLD.bound_school_id) THEN
  RAISE EXCEPTION 'account kind and simulated school are immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_guard_seed_post() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE kind text; scope uuid; batch_state text;
BEGIN
 IF TG_OP='UPDATE' AND (NEW.seed_batch_id,NEW.author_id,NEW.display_published_at,NEW.created_at) IS DISTINCT FROM (OLD.seed_batch_id,OLD.author_id,OLD.display_published_at,OLD.created_at) THEN
  RAISE EXCEPTION 'post identity and time are immutable' USING ERRCODE='23514';
 END IF;
 SELECT account_kind,bound_school_id INTO kind,scope FROM accounts WHERE id=NEW.author_id;
 IF NEW.created_via='seed' THEN
  SELECT state INTO batch_state FROM post_seed_batches WHERE id=NEW.seed_batch_id AND school_id=NEW.school_id FOR SHARE;
  IF kind<>'simulated' OR scope IS DISTINCT FROM NEW.school_id OR batch_state IS NULL
   OR (TG_OP='INSERT' AND batch_state<>'preview') OR NEW.display_published_at>now() THEN
   RAISE EXCEPTION 'invalid seed provenance' USING ERRCODE='23514';
  END IF;
  IF batch_state='revoked' AND (NEW.visibility='visible' OR NEW.audit_status='approved' AND (TG_OP='INSERT' OR OLD.audit_status<>'approved')) THEN
   RAISE EXCEPTION 'revoked seed content cannot be republished' USING ERRCODE='23514';
  END IF;
 ELSIF kind='simulated' THEN RAISE EXCEPTION 'simulated author requires seed provenance' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_media_grace_hours() RETURNS integer
    LANGUAGE plpgsql
    AS $$
DECLARE hours integer := coalesce(nullif(current_setting('linjian.media_orphan_grace_hours',true),''),'24')::integer;
BEGIN
 IF hours NOT BETWEEN 1 AND 168 THEN RAISE EXCEPTION 'Invalid media grace hours'; END IF;
 RETURN hours;
END $$;

CREATE FUNCTION linjian_ordinary_actor() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE subject uuid;
BEGIN
 subject := (to_jsonb(NEW)->>TG_ARGV[0])::uuid;
 IF EXISTS(SELECT 1 FROM accounts WHERE id=subject AND account_kind='simulated') THEN
  RAISE EXCEPTION 'simulated account cannot authenticate or act' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_public_post(item posts) RETURNS jsonb
    LANGUAGE sql STABLE
    AS $$
 SELECT jsonb_build_object('id',item.id,'school_id',item.school_id,'author_id',item.author_id,'title',item.title,'content',item.content,'images',item.images,'tags',item.tags,'audit_status',item.audit_status,'visibility',item.visibility,'browse',item.browse,'revision',item.revision,'display_published_at',item.display_published_at)
$$;

CREATE FUNCTION linjian_seed_batch_guard() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF (NEW.school_id,NEW.created_by,NEW.preview_digest,NEW.policy_digest,NEW.preview_payload,NEW.created_at,NEW.name,NEW.expires_at) IS DISTINCT FROM (OLD.school_id,OLD.created_by,OLD.preview_digest,OLD.policy_digest,OLD.preview_payload,OLD.created_at,OLD.name,OLD.expires_at)
 OR NEW.revision<>OLD.revision+1
 OR (OLD.state='committed' AND (NEW.committed_at,NEW.commit_idempotency_key,NEW.commit_request_digest) IS DISTINCT FROM (OLD.committed_at,OLD.commit_idempotency_key,OLD.commit_request_digest))
 OR NOT ((OLD.state='preview' AND NEW.state='committed') OR (OLD.state='committed' AND NEW.state='revoked')) THEN
  RAISE EXCEPTION 'invalid batch transition' USING ERRCODE='23514';
 END IF;
 IF NEW.state='revoked' AND EXISTS(SELECT 1 FROM posts WHERE seed_batch_id=NEW.id AND visibility NOT IN ('offline','deleted')) THEN
  RAISE EXCEPTION 'batch posts must be offline before revoke' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_seed_item_consistent() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM posts p JOIN post_seed_items i ON i.post_id=p.id AND i.batch_id=p.seed_batch_id AND i.school_id=p.school_id AND i.author_id=p.author_id WHERE p.id=NEW.post_id) THEN
  RAISE EXCEPTION 'seed item mismatch' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_seed_post_consistent() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF NEW.created_via='seed' AND NOT EXISTS(SELECT 1 FROM post_seed_items i WHERE i.post_id=NEW.id AND i.batch_id=NEW.seed_batch_id AND i.school_id=NEW.school_id AND i.author_id=NEW.author_id) THEN
  RAISE EXCEPTION 'seed post requires batch item' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_service_version_immutable() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF OLD.config IS DISTINCT FROM NEW.config OR OLD.target_id<>NEW.target_id OR OLD.creator_id<>NEW.creator_id OR OLD.version_number<>NEW.version_number OR OLD.content_digest<>NEW.content_digest OR OLD.created_at<>NEW.created_at THEN
  RAISE EXCEPTION 'Service version configurations are immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION statistics_capture_registration() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 INSERT INTO statistics_registrations(account_id,registered_at,registration_school_id,school_assignment,time_quality)
 SELECT NEW.account_id,NEW.created_at,a.bound_school_id,CASE WHEN a.bound_school_id IS NULL THEN 'unassigned' ELSE 'known_school' END,'verified'
 FROM accounts a WHERE a.id=NEW.account_id AND a.account_kind='ordinary';
 RETURN NEW;
END $$;

CREATE FUNCTION forum_create_source_intent(item posts, trigger_kind text) RETURNS uuid
    LANGUAGE plpgsql
    AS $$
DECLARE seq bigint; b school_source_bindings; bv uuid; event uuid:=gen_random_uuid();
BEGIN
 IF item.created_via='seed' OR EXISTS(SELECT 1 FROM posts WHERE id=item.id AND created_via='seed') THEN RAISE EXCEPTION 'seed_content_not_recommendable' USING ERRCODE='23514'; END IF;
 IF trigger_kind NOT IN ('review_publication','manual_recommendation') THEN RAISE EXCEPTION 'invalid trigger'; END IF;
 PERFORM id FROM schools WHERE id=item.school_id FOR UPDATE;
 IF item.audit_status<>'approved' OR item.visibility<>'visible' OR item.approved_snapshot_id IS NULL
 OR NOT EXISTS(SELECT 1 FROM schools WHERE id=item.school_id AND active) THEN RAISE EXCEPTION 'publication not eligible'; END IF;
 IF NOT forum_post_has_publication_evidence(item) THEN RAISE EXCEPTION 'publication evidence mismatch'; END IF;
 INSERT INTO forum_source_versions(school_id,post_id,sequence,available) VALUES(item.school_id,item.id,1,true)
 ON CONFLICT(school_id,post_id) DO UPDATE SET sequence=forum_source_versions.sequence+1,available=true,delivery_intent=true RETURNING sequence INTO seq;
 SELECT sb.* INTO b FROM school_source_bindings sb JOIN source_active_configuration ac ON ac.connection_id=sb.connection_id WHERE sb.school_id=item.school_id FOR SHARE OF sb,ac;
 IF b.id IS NOT NULL THEN SELECT id INTO bv FROM school_source_binding_versions WHERE binding_id=b.id AND revision=b.revision; END IF;
 INSERT INTO forum_outbox(id,school_id,post_id,sequence,payload,schema_version,signature_version,delivery_state,publication_snapshot_id,projection_version,trigger_type)
 VALUES(event,item.school_id,item.id,seq,jsonb_build_object('event_id',event,'schema_version',2,'event_type','content.upsert','occurred_at',now()),2,2,'waiting_binding',item.approved_snapshot_id,'message_v1',trigger_kind);
 INSERT INTO forum_publication_preparations(event_id,snapshot_id) VALUES(event,item.approved_snapshot_id);
 IF b.id IS NOT NULL THEN PERFORM forum_bind_prepared_source(event,b.connection_version,bv,b.route_id); END IF;
 IF b.id IS NOT NULL AND NOT b.active THEN UPDATE forum_outbox SET delivery_state='paused' WHERE id=event; END IF;
 RETURN event;
END $$;

CREATE FUNCTION forum_finalize_prepared_source(event uuid, lease uuid, asset uuid, body text, version text) RETURNS boolean
    LANGUAGE plpgsql
    AS $$
DECLARE o forum_outbox; prep forum_publication_preparations; a miniapp_link_assets; attrs jsonb;
BEGIN
 SELECT * INTO STRICT o FROM forum_outbox WHERE id=event FOR UPDATE;
 SELECT * INTO STRICT prep FROM forum_publication_preparations WHERE event_id=event FOR UPDATE;
 IF prep.state<>'generating' OR prep.lease_token IS DISTINCT FROM lease OR prep.lease_until<=now() OR o.request_body IS NOT NULL THEN RETURN false; END IF;
 IF NOT forum_publication_is_current(event) THEN UPDATE forum_outbox SET delivery_state='cancelled' WHERE id=event; RETURN false; END IF;
 IF NOT EXISTS(SELECT 1 FROM school_source_bindings b WHERE b.school_id=o.school_id AND b.active) THEN
  UPDATE forum_publication_preparations SET state='pending',lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event; RETURN false;
 END IF;
 IF asset IS NOT NULL THEN
  SELECT * INTO STRICT a FROM miniapp_link_assets WHERE id=asset;
  IF a.app_id IS DISTINCT FROM prep.resolved_policy->>'app_id' OR a.policy IS DISTINCT FROM prep.resolved_policy
  OR a.page_path<>'pages/index/detail' OR a.canonical_query<>('school='||o.school_id||'&id='||o.post_id) THEN RAISE EXCEPTION 'link does not belong to this publication'; END IF;
  IF a.expires_at IS NOT NULL AND a.expires_at<=now()+make_interval(secs=>(prep.resolved_policy->>'min_remaining')::integer) THEN
   UPDATE forum_publication_preparations SET state='retry_wait',next_attempt_at=now()+interval '5 seconds',last_error_code='link_near_expiry',lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event; RETURN false;
  END IF;
 ELSIF EXISTS(SELECT 1 FROM posts WHERE id=o.post_id AND created_via='miniapp') THEN RAISE EXCEPTION 'miniapp publication requires a link';
 END IF;
 attrs:=body::jsonb->'data'->'attributes';
 IF body::jsonb->'data'->>'type' IS DISTINCT FROM 'source-events'
 OR body::jsonb->'data'->>'id' IS DISTINCT FROM event::text
 OR attrs->>'event_id' IS DISTINCT FROM event::text OR attrs->>'route_id' IS DISTINCT FROM o.route_id
 OR attrs->'schema_version' IS DISTINCT FROM '2'::jsonb
 OR (attrs->>'occurred_at')::timestamptz IS DISTINCT FROM (o.payload->>'occurred_at')::timestamptz
 OR attrs->>'event_type' IS DISTINCT FROM 'content.upsert'
 OR jsonb_typeof(attrs->'content') IS DISTINCT FROM 'string'
 OR coalesce(length(btrim(attrs->>'content')),0)=0 OR length(attrs->>'content')>200
 OR (SELECT array_agg(k ORDER BY k) FROM jsonb_object_keys(attrs) k) IS DISTINCT FROM ARRAY['content','event_id','event_type','occurred_at','route_id','schema_version']
 OR version IS DISTINCT FROM 'summary-20-v1' THEN RAISE EXCEPTION 'invalid frozen message'; END IF;
 UPDATE forum_outbox SET payload=attrs,request_body=body,body_sha256=encode(sha256(convert_to(body,'UTF8')),'hex'),message_version=version,frozen_at=now(),delivery_state='ready' WHERE id=event;
 UPDATE forum_publication_preparations SET state='completed',link_asset_id=asset,completed_at=now(),last_error_code=NULL,lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event;
 RETURN true;
END $$;

CREATE FUNCTION forum_has_permission(actor uuid, school uuid, permission text) RETURNS boolean
    LANGUAGE sql STABLE
    AS $$
 SELECT EXISTS(
 SELECT 1 FROM accounts a JOIN schools s ON s.id=school
 CROSS JOIN LATERAL jsonb_array_elements(forum_permission_catalog()) p
 LEFT JOIN school_memberships m ON m.school_id=s.id AND m.account_id=a.id
 LEFT JOIN school_member_permission_overrides o ON o.school_id=s.id AND o.account_id=a.id AND o.permission_key=permission
 WHERE a.id=actor AND a.active AND p->>'key'=permission
 AND (a.platform_admin OR (s.active AND m.role IN ('admin','operator')
 AND (p->'applicable_roles') ? m.role
 AND CASE WHEN o.effect='deny' THEN false WHEN o.effect='allow' THEN true ELSE coalesce((p->'default_by_role'->>m.role)::boolean,false) END)))
$$;

CREATE FUNCTION linjian_refresh_file_gc(assets uuid[]) RETURNS void
    LANGUAGE plpgsql
    AS $$
DECLARE asset uuid;
BEGIN
 FOR asset IN SELECT id FROM file_assets WHERE id=ANY(assets) ORDER BY id FOR UPDATE LOOP
  UPDATE file_assets SET gc_after=CASE WHEN linjian_file_protected(asset) THEN NULL
   ELSE coalesce(gc_after,now()+make_interval(hours=>linjian_media_grace_hours())) END
   WHERE id=asset AND state IN ('ready','uploading');
 END LOOP;
END $$;

CREATE FUNCTION forum_enqueue_post() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF TG_OP='UPDATE' AND (NEW.title,NEW.content,NEW.images,NEW.tags,NEW.audit_status,NEW.visibility,NEW.approved_snapshot_id) IS NOT DISTINCT FROM
 (OLD.title,OLD.content,OLD.images,OLD.tags,OLD.audit_status,OLD.visibility,OLD.approved_snapshot_id) THEN RETURN NEW; END IF;
 PERFORM id FROM schools WHERE id=NEW.school_id FOR UPDATE;
 UPDATE forum_outbox SET delivery_state='cancelled' WHERE school_id=NEW.school_id AND post_id=NEW.id
 AND attempts=0 AND lease_token IS NULL AND delivery_state NOT IN ('accepted','cancelled');
 IF NEW.created_via='seed' THEN RETURN NEW; END IF;
 IF NEW.audit_status='approved' AND NEW.visibility='visible' AND EXISTS(SELECT 1 FROM schools WHERE id=NEW.school_id AND active) THEN
  PERFORM forum_create_source_intent(NEW,'review_publication');
 ELSE
  UPDATE forum_source_versions SET available=false WHERE school_id=NEW.school_id AND post_id=NEW.id;
 END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_file_protection_changed() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
BEGIN
 IF TG_OP='DELETE' THEN PERFORM linjian_refresh_file_gc(ARRAY[OLD.asset_id]); RETURN OLD; END IF;
 PERFORM linjian_refresh_file_gc(ARRAY[NEW.asset_id]); RETURN NEW;
END $$;

CREATE FUNCTION linjian_file_references() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE v jsonb; ids uuid[] := '{}'; previous uuid[]; asset uuid; object uuid; scope uuid; expected text;
BEGIN
 v := CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 object := coalesce((v->>'id')::uuid,(v->>'account_id')::uuid,'00000000-0000-0000-0000-000000000001'::uuid);
 SELECT coalesce(array_agg(asset_id),'{}') INTO previous FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object;

 scope := (v->>'school_id')::uuid;
 IF TG_OP='DELETE' OR (TG_TABLE_NAME='posts' AND v->>'visibility'='deleted') THEN
  ids := '{}';
 ELSIF TG_TABLE_NAME IN ('posts','forum_publication_snapshots','cases') THEN
  expected := CASE WHEN TG_TABLE_NAME='cases' THEN 'feedback_evidence' ELSE 'post_image' END;
  SELECT coalesce(array_agg(DISTINCT split_part(x,'/',7)::uuid),'{}') INTO ids
   FROM jsonb_array_elements_text(coalesce(v->'images','[]')) x;
 ELSIF TG_TABLE_NAME='schools' THEN
  scope := object; expected := 'operation_artwork';
  ids := array_remove(ARRAY[(v->>'logo_asset_id')::uuid,(v->>'header_bg_asset_id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='miniapp_users' THEN
  expected := 'avatar'; ids := array_remove(ARRAY[(v->>'avatar_asset_id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='customer_config' THEN
  expected := 'operation_artwork';
  ids := array_remove(ARRAY[(v->>'business_asset_id')::uuid,(v->>'join_asset_id')::uuid,(v->>'service_asset_id')::uuid,(v->>'fallback_asset_id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='school_resources' THEN
  expected := 'operation_artwork'; ids := array_remove(ARRAY[(v->'image_ref'->>'id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='resource_publications' THEN
  expected := 'operation_artwork'; ids := array_remove(ARRAY[(v->'snapshot'->'image_ref'->>'id')::uuid],NULL);
 ELSIF TG_TABLE_NAME='product_previews' AND v->>'kind'='resource' THEN
  expected := 'operation_artwork'; ids := array_remove(ARRAY[(v->'payload'->'image_ref'->>'id')::uuid],NULL);
 END IF;
 -- Lock the union before changing references, including assets only being released.
 PERFORM id FROM file_assets WHERE id=ANY(previous || ids) ORDER BY id FOR UPDATE;
 DELETE FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object AND NOT(asset_id=ANY(ids));
 FOR asset IN SELECT DISTINCT unnest(ids) ORDER BY 1 LOOP
  PERFORM 1 FROM file_assets WHERE id=asset AND purpose=expected AND state='ready'
   AND school_id IS NOT DISTINCT FROM scope AND ((NOT ever_bound AND expires_at>now()) OR asset=ANY(previous)
    OR EXISTS(SELECT 1 FROM file_asset_references r WHERE r.asset_id=asset)
    OR EXISTS(SELECT 1 FROM file_asset_leases l WHERE l.asset_id=asset AND l.expires_at>now()));
  IF NOT FOUND THEN RAISE EXCEPTION 'Invalid file asset binding' USING ERRCODE='23514', CONSTRAINT='file_asset_binding'; END IF;
  INSERT INTO file_asset_references VALUES(asset,TG_TABLE_NAME,object) ON CONFLICT DO NOTHING;
  UPDATE file_assets SET ever_bound=true,gc_after=NULL WHERE id=asset;
 END LOOP;
 PERFORM linjian_refresh_file_gc(previous || ids);
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION linjian_service_file_references() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE payload jsonb; previous uuid[]; assets uuid[]; asset uuid; object uuid; scope uuid;
BEGIN
 payload:=CASE WHEN TG_OP='DELETE' THEN to_jsonb(OLD) ELSE to_jsonb(NEW) END;
 object:=(payload->>'id')::uuid; scope:=(payload->>'school_id')::uuid;
 SELECT coalesce(array_agg(asset_id),'{}') INTO previous FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object;
 IF TG_OP='DELETE' THEN assets:='{}';
 ELSIF TG_TABLE_NAME='school_shortcut_overrides' THEN assets:=array_remove(ARRAY[(payload->'presentation'->'image_ref'->>'id')::uuid],NULL);
 ELSE
  SELECT coalesce(array_agg((item->'action'->>'asset_id')::uuid),'{}') INTO assets
   FROM jsonb_array_elements(payload->'config'->'channels') item WHERE item->'action'->>'kind'='qr_code';
 END IF;
 PERFORM id FROM file_assets WHERE id=ANY(previous || assets) ORDER BY id FOR UPDATE;
 DELETE FROM file_asset_references WHERE object_type=TG_TABLE_NAME AND object_id=object AND NOT(asset_id=ANY(assets));
 FOREACH asset IN ARRAY assets LOOP
  PERFORM 1 FROM file_assets WHERE id=asset AND purpose='operation_artwork' AND state='ready' AND school_id IS NOT DISTINCT FROM scope
   AND ((NOT ever_bound AND expires_at>now()) OR asset=ANY(previous) OR EXISTS(SELECT 1 FROM file_asset_references WHERE asset_id=asset));
  IF NOT FOUND THEN RAISE EXCEPTION 'Invalid service file binding' USING ERRCODE='23514', CONSTRAINT='file_asset_binding'; END IF;
  INSERT INTO file_asset_references VALUES(asset,TG_TABLE_NAME,object) ON CONFLICT DO NOTHING;
  UPDATE file_assets SET ever_bound=true,gc_after=NULL WHERE id=asset;
 END LOOP;
 PERFORM linjian_refresh_file_gc(previous || assets);
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;

CREATE FUNCTION site_settings_logo_reference() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
DECLARE old_asset uuid; new_asset uuid; object uuid;
BEGIN
 object := CASE WHEN TG_OP='DELETE' THEN OLD.id ELSE NEW.id END;
 IF TG_OP IN ('UPDATE','DELETE') THEN old_asset := OLD.logo_asset_id; END IF;
 IF TG_OP IN ('INSERT','UPDATE') THEN new_asset := NEW.logo_asset_id; END IF;
 PERFORM id FROM file_assets WHERE id=ANY(ARRAY[old_asset,new_asset]) ORDER BY id FOR UPDATE;
 IF new_asset IS NOT NULL THEN
  PERFORM 1 FROM file_assets f JOIN operational_media m ON m.id=f.id
   WHERE f.id=new_asset AND f.purpose='operation_artwork' AND f.state='ready'
   AND f.school_id IS NULL AND m.school_id IS NULL;
  IF NOT FOUND THEN
   RAISE EXCEPTION 'Invalid site logo asset' USING ERRCODE='23514', CONSTRAINT='file_asset_binding';
  END IF;
  INSERT INTO file_asset_references(asset_id,object_type,object_id)
   VALUES(new_asset,'site_settings',object) ON CONFLICT DO NOTHING;
  UPDATE file_assets SET ever_bound=true,gc_after=NULL WHERE id=new_asset;
  DELETE FROM file_asset_references
   WHERE object_type='site_settings' AND object_id=object AND asset_id<>new_asset;
 ELSE
  DELETE FROM file_asset_references
   WHERE object_type='site_settings' AND object_id=object;
 END IF;
 PERFORM linjian_refresh_file_gc(ARRAY[old_asset,new_asset]);
 IF TG_OP='DELETE' THEN RETURN OLD; END IF;
 RETURN NEW;
END $$;

CREATE TRIGGER account_kind_immutable BEFORE UPDATE ON accounts FOR EACH ROW EXECUTE FUNCTION linjian_guard_account_kind();

CREATE TRIGGER file_history_guard BEFORE UPDATE ON file_assets FOR EACH ROW EXECUTE FUNCTION linjian_file_history_guard();

CREATE TRIGGER file_lease_changed AFTER INSERT OR DELETE OR UPDATE ON file_asset_leases FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_changed();

CREATE TRIGGER file_lease_guard BEFORE INSERT OR DELETE OR UPDATE ON file_asset_leases FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_guard();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON cases FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON customer_config FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON forum_publication_snapshots FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON miniapp_users FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON operation_target_versions FOR EACH ROW EXECUTE FUNCTION linjian_service_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON product_previews FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON resource_publications FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON school_resources FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON school_shortcut_overrides FOR EACH ROW EXECUTE FUNCTION linjian_service_file_references();

CREATE TRIGGER file_references AFTER INSERT OR DELETE OR UPDATE ON schools FOR EACH ROW EXECUTE FUNCTION linjian_file_references();

CREATE TRIGGER forum_binding_history AFTER INSERT OR UPDATE ON school_source_bindings FOR EACH ROW EXECUTE FUNCTION forum_binding_history();

CREATE TRIGGER forum_bound_publication_guard BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_bound_preparation();

CREATE TRIGGER forum_connection_version_key BEFORE INSERT ON source_connection_versions FOR EACH ROW EXECUTE FUNCTION forum_connection_version_key();

CREATE TRIGGER forum_decision_guard BEFORE INSERT OR DELETE OR UPDATE ON moderation_events FOR EACH ROW EXECUTE FUNCTION forum_guard_decision();

CREATE TRIGGER forum_guard_binding_version BEFORE DELETE OR UPDATE ON school_source_binding_versions FOR EACH ROW EXECUTE FUNCTION forum_guard_source_version();

CREATE TRIGGER forum_guard_frozen_source BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_frozen_source();

CREATE TRIGGER forum_guard_message_version BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_message_version();

CREATE TRIGGER forum_guard_source_version BEFORE DELETE OR UPDATE ON source_connection_versions FOR EACH ROW EXECUTE FUNCTION forum_guard_source_version();

CREATE TRIGGER forum_new_snapshot_guard BEFORE INSERT ON forum_publication_snapshots FOR EACH ROW EXECUTE FUNCTION forum_guard_new_snapshot();

CREATE TRIGGER forum_post_order_metadata BEFORE INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_post_order_metadata();

CREATE TRIGGER forum_post_order_version AFTER INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_post_order_version();

CREATE TRIGGER forum_post_outbox AFTER INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_enqueue_post();

CREATE TRIGGER forum_post_publication_guard BEFORE INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION forum_guard_post_publication();

CREATE TRIGGER forum_preparation_invalidation AFTER UPDATE OF delivery_state ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_invalidate_preparation();

CREATE TRIGGER forum_recommendation_immutable BEFORE DELETE OR UPDATE ON forum_post_recommendations FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();

CREATE TRIGGER forum_school_authorization_guard AFTER INSERT ON schools FOR EACH ROW EXECUTE FUNCTION forum_create_school_authorization_guard();

CREATE TRIGGER forum_school_outbox AFTER UPDATE OF active ON schools FOR EACH ROW EXECUTE FUNCTION forum_enqueue_school();

CREATE TRIGGER forum_snapshot_immutable BEFORE DELETE OR UPDATE ON forum_publication_snapshots FOR EACH ROW EXECUTE FUNCTION forum_guard_publication_snapshot();

CREATE TRIGGER forum_source_trigger_immutable BEFORE UPDATE ON forum_outbox FOR EACH ROW EXECUTE FUNCTION forum_guard_source_trigger();

CREATE TRIGGER forum_termination_immutable BEFORE DELETE OR UPDATE ON forum_source_terminations FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();

CREATE TRIGGER media_token_changed AFTER INSERT OR DELETE OR UPDATE ON media_access_tokens FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_changed();

CREATE TRIGGER media_token_guard BEFORE INSERT OR DELETE OR UPDATE ON media_access_tokens FOR EACH ROW EXECUTE FUNCTION linjian_file_protection_guard();

CREATE TRIGGER miniapp_link_immutable BEFORE DELETE OR UPDATE ON miniapp_link_assets FOR EACH ROW EXECUTE FUNCTION forum_guard_publication_snapshot();

CREATE TRIGGER ordinary_activity BEFORE INSERT OR UPDATE ON statistics_activity_events FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_authorization BEFORE INSERT OR UPDATE ON school_member_authorizations FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_batch_actor BEFORE INSERT OR UPDATE ON post_seed_batches FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('created_by');

CREATE TRIGGER ordinary_case BEFORE INSERT OR UPDATE ON cases FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_comment BEFORE INSERT OR UPDATE ON comments FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('author_id');

CREATE TRIGGER ordinary_comment_reaction BEFORE INSERT OR UPDATE ON comment_reactions FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_follow BEFORE INSERT OR UPDATE ON follows FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('follower_id');

CREATE TRIGGER ordinary_identity BEFORE INSERT OR UPDATE ON miniapp_identities FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_membership BEFORE INSERT OR UPDATE ON school_memberships FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_override BEFORE INSERT OR UPDATE ON school_member_permission_overrides FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_reaction BEFORE INSERT OR UPDATE ON post_reactions FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_registration BEFORE INSERT OR UPDATE ON statistics_registrations FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER ordinary_sessions BEFORE INSERT OR UPDATE ON sessions FOR EACH ROW EXECUTE FUNCTION linjian_ordinary_actor('account_id');

CREATE TRIGGER resource_publications_immutable BEFORE DELETE OR UPDATE ON resource_publications FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();

CREATE TRIGGER seed_batch_guard BEFORE UPDATE ON post_seed_batches FOR EACH ROW EXECUTE FUNCTION linjian_seed_batch_guard();

CREATE CONSTRAINT TRIGGER seed_item_consistent AFTER INSERT ON post_seed_items DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_seed_item_consistent();

CREATE TRIGGER seed_item_immutable BEFORE DELETE OR UPDATE ON post_seed_items FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();

CREATE CONSTRAINT TRIGGER seed_post_consistent AFTER INSERT ON posts DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_seed_post_consistent();

CREATE TRIGGER seed_post_guard BEFORE INSERT OR UPDATE ON posts FOR EACH ROW EXECUTE FUNCTION linjian_guard_seed_post();

CREATE TRIGGER service_reviews_immutable BEFORE DELETE OR UPDATE ON operation_target_reviews FOR EACH ROW EXECUTE FUNCTION forum_events_immutable();

CREATE TRIGGER service_version_immutable BEFORE UPDATE ON operation_target_versions FOR EACH ROW EXECUTE FUNCTION linjian_service_version_immutable();

CREATE TRIGGER site_settings_logo_reference AFTER INSERT OR DELETE OR UPDATE ON site_settings FOR EACH ROW EXECUTE FUNCTION site_settings_logo_reference();

CREATE TRIGGER statistics_registration AFTER INSERT ON miniapp_users FOR EACH ROW EXECUTE FUNCTION statistics_capture_registration();
