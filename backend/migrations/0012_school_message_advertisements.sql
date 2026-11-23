CREATE FUNCTION linjian_message_advertisement_text_valid(value text) RETURNS boolean
LANGUAGE sql IMMUTABLE AS $$
 SELECT length(value) BETWEEN 1 AND 50
 AND value=btrim(value,U&'\0009\000A\000B\000C\000D\0020\0085\00A0\1680\2000\2001\2002\2003\2004\2005\2006\2007\2008\2009\200A\2028\2029\202F\205F\3000')
 AND value !~ U&'[\0001-\001F\007F-\009F\2028\2029\FEFF]'
$$;

CREATE TABLE school_message_advertisements (
 id uuid PRIMARY KEY,
 school_id uuid NOT NULL REFERENCES schools(id) DEFERRABLE INITIALLY DEFERRED,
 current_revision_id uuid,
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 created_by uuid NOT NULL REFERENCES accounts(id) DEFERRABLE INITIALLY DEFERRED,
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 deleted_at timestamptz,
 deleted_by uuid REFERENCES accounts(id) DEFERRABLE INITIALLY DEFERRED,
 deletion_kind text CHECK(deletion_kind IN ('school_delete','platform_remove')),
 deletion_reason text,
 UNIQUE(id,school_id),
 CHECK((deleted_at IS NULL AND deleted_by IS NULL AND deletion_kind IS NULL AND deletion_reason IS NULL)
 OR (deleted_at IS NOT NULL AND deleted_by IS NOT NULL AND deletion_kind IS NOT NULL AND current_revision_id IS NULL
 AND (deletion_kind='school_delete' OR coalesce(length(btrim(deletion_reason)),0) BETWEEN 1 AND 500)))
);
CREATE UNIQUE INDEX school_message_advertisements_one_current ON school_message_advertisements(school_id) WHERE deleted_at IS NULL;

CREATE TABLE school_message_advertisement_revisions (
 id uuid PRIMARY KEY,
 advertisement_id uuid NOT NULL,
 school_id uuid NOT NULL,
 number bigint NOT NULL CHECK(number>0),
 content text NOT NULL CHECK(linjian_message_advertisement_text_valid(content)),
 status text NOT NULL DEFAULT 'draft' CHECK(status IN ('draft','pending','approved','rejected','withdrawn')),
 version bigint NOT NULL DEFAULT 1 CHECK(version>0),
 normalization_version text NOT NULL DEFAULT 'plain-text-v1' CHECK(normalization_version='plain-text-v1'),
 review_digest text,
 created_by uuid NOT NULL REFERENCES accounts(id) DEFERRABLE INITIALLY DEFERRED,
 created_at timestamptz NOT NULL DEFAULT now(),
 updated_at timestamptz NOT NULL DEFAULT now(),
 submitted_by uuid REFERENCES accounts(id) DEFERRABLE INITIALLY DEFERRED,
 submitted_at timestamptz,
 UNIQUE(advertisement_id,number),
 UNIQUE(id,advertisement_id,school_id),
 FOREIGN KEY(advertisement_id,school_id) REFERENCES school_message_advertisements(id,school_id) DEFERRABLE INITIALLY DEFERRED,
 CHECK((status IN ('draft','withdrawn') AND review_digest IS NULL AND submitted_by IS NULL AND submitted_at IS NULL)
 OR (status IN ('pending','approved','rejected','withdrawn') AND review_digest IS NOT NULL AND review_digest ~ '^[0-9a-f]{64}$' AND submitted_by IS NOT NULL AND submitted_at IS NOT NULL))
);
CREATE UNIQUE INDEX school_message_advertisement_one_candidate ON school_message_advertisement_revisions(advertisement_id) WHERE status IN ('draft','pending');
CREATE INDEX school_message_advertisement_review_queue ON school_message_advertisement_revisions(status,(coalesce(submitted_at,created_at)) DESC,id DESC);
ALTER TABLE school_message_advertisements ADD CONSTRAINT school_message_advertisement_current_fk
 FOREIGN KEY(current_revision_id,id,school_id) REFERENCES school_message_advertisement_revisions(id,advertisement_id,school_id) DEFERRABLE INITIALLY DEFERRED;

CREATE TABLE school_message_advertisement_decisions (
 id uuid PRIMARY KEY,
 revision_id uuid NOT NULL UNIQUE,
 advertisement_id uuid NOT NULL,
 school_id uuid NOT NULL,
 decision text NOT NULL CHECK(decision IN ('approve','reject')),
 reason text NOT NULL CHECK(length(reason)<=500 AND (decision='approve' OR length(btrim(reason))>0)),
 reviewer_id uuid NOT NULL REFERENCES accounts(id) DEFERRABLE INITIALLY DEFERRED,
 reviewed_at timestamptz NOT NULL DEFAULT now(),
 review_digest text NOT NULL CHECK(review_digest ~ '^[0-9a-f]{64}$'),
 FOREIGN KEY(revision_id,advertisement_id,school_id) REFERENCES school_message_advertisement_revisions(id,advertisement_id,school_id) DEFERRABLE INITIALLY DEFERRED
);

ALTER TABLE forum_outbox ADD CONSTRAINT forum_outbox_advertisement_school_key UNIQUE(id,school_id);
CREATE TABLE forum_message_advertisement_snapshots (
 event_id uuid PRIMARY KEY,
 school_id uuid NOT NULL,
 advertisement_id uuid,
 revision_id uuid,
 content text,
 result text NOT NULL CHECK(result IN ('applied','no_approved_advertisement','message_budget_exceeded')),
 base_length integer NOT NULL CHECK(base_length BETWEEN 1 AND 200),
 final_length integer NOT NULL CHECK(final_length BETWEEN 1 AND 200),
 composition_version text NOT NULL CHECK(composition_version='summary-20-ad-tail-v1'),
 frozen_at timestamptz NOT NULL DEFAULT now(),
 FOREIGN KEY(event_id,school_id) REFERENCES forum_outbox(id,school_id) DEFERRABLE INITIALLY DEFERRED,
 FOREIGN KEY(revision_id,advertisement_id,school_id) REFERENCES school_message_advertisement_revisions(id,advertisement_id,school_id) DEFERRABLE INITIALLY DEFERRED,
 CHECK((result='no_approved_advertisement' AND advertisement_id IS NULL AND revision_id IS NULL AND content IS NULL AND base_length=final_length)
 OR (result='applied' AND advertisement_id IS NOT NULL AND revision_id IS NOT NULL AND content IS NOT NULL AND final_length=base_length+1+length(content))
 OR (result='message_budget_exceeded' AND advertisement_id IS NOT NULL AND revision_id IS NOT NULL AND content IS NOT NULL AND base_length+1+length(content)>200 AND base_length=final_length))
);
CREATE INDEX forum_message_advertisement_school_history ON forum_message_advertisement_snapshots(school_id,frozen_at DESC,event_id);

CREATE FUNCTION linjian_message_advertisement_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 RAISE EXCEPTION 'advertisement evidence is immutable' USING ERRCODE='23514';
END $$;
CREATE TRIGGER message_advertisement_decision_immutable BEFORE UPDATE OR DELETE ON school_message_advertisement_decisions
 FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_immutable();
CREATE TRIGGER message_advertisement_snapshot_immutable BEFORE UPDATE OR DELETE ON forum_message_advertisement_snapshots
 FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_immutable();

CREATE FUNCTION linjian_message_advertisement_revision_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'advertisement revision cannot be deleted' USING ERRCODE='23514'; END IF;
 IF (NEW.id,NEW.school_id,NEW.advertisement_id,NEW.number,NEW.normalization_version,NEW.created_by,NEW.created_at)
 IS DISTINCT FROM (OLD.id,OLD.school_id,OLD.advertisement_id,OLD.number,OLD.normalization_version,OLD.created_by,OLD.created_at)
 OR (OLD.status<>'draft' AND (NEW.content,NEW.review_digest,NEW.submitted_by,NEW.submitted_at)
 IS DISTINCT FROM (OLD.content,OLD.review_digest,OLD.submitted_by,OLD.submitted_at))
 OR (NEW.status IS DISTINCT FROM OLD.status AND NOT
 ((OLD.status='draft' AND NEW.status IN ('pending','withdrawn')) OR (OLD.status='pending' AND NEW.status IN ('approved','rejected','withdrawn')))) THEN
  RAISE EXCEPTION 'invalid advertisement revision change' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER message_advertisement_revision_guard BEFORE UPDATE OR DELETE ON school_message_advertisement_revisions
 FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_revision_guard();

CREATE FUNCTION linjian_message_advertisement_config_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'advertisement cannot be deleted physically' USING ERRCODE='23514'; END IF;
 IF (NEW.id,NEW.school_id,NEW.created_by,NEW.created_at) IS DISTINCT FROM (OLD.id,OLD.school_id,OLD.created_by,OLD.created_at)
 OR (OLD.deleted_at IS NOT NULL AND NEW IS DISTINCT FROM OLD) THEN
  RAISE EXCEPTION 'advertisement identity or deletion is immutable' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER message_advertisement_config_guard BEFORE UPDATE OR DELETE ON school_message_advertisements
 FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_config_guard();

CREATE FUNCTION linjian_message_advertisement_check_evidence() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE advertisement uuid; item school_message_advertisements;
BEGIN
 IF TG_TABLE_NAME='school_message_advertisements' THEN advertisement:=NEW.id; ELSE advertisement:=NEW.advertisement_id; END IF;
 SELECT * INTO STRICT item FROM school_message_advertisements WHERE id=advertisement;
 IF item.current_revision_id IS NOT NULL AND NOT EXISTS(
  SELECT 1 FROM school_message_advertisement_revisions r JOIN school_message_advertisement_decisions d ON d.revision_id=r.id
  WHERE r.id=item.current_revision_id AND r.advertisement_id=item.id AND r.school_id=item.school_id
  AND r.status='approved' AND d.decision='approve' AND d.review_digest=r.review_digest) THEN
  RAISE EXCEPTION 'current advertisement requires approval evidence' USING ERRCODE='23514';
 END IF;
 IF item.deleted_at IS NOT NULL AND EXISTS(SELECT 1 FROM school_message_advertisement_revisions WHERE advertisement_id=item.id AND status IN ('draft','pending')) THEN
  RAISE EXCEPTION 'deleted advertisement has open revisions' USING ERRCODE='23514';
 END IF;
 IF EXISTS(SELECT 1 FROM school_message_advertisement_revisions r LEFT JOIN school_message_advertisement_decisions d ON d.revision_id=r.id
 WHERE r.advertisement_id=item.id AND
 ((r.status IN ('approved','rejected') AND (d.id IS NULL OR d.decision IS DISTINCT FROM CASE r.status WHEN 'approved' THEN 'approve' ELSE 'reject' END OR d.review_digest IS DISTINCT FROM r.review_digest))
 OR (r.status NOT IN ('approved','rejected') AND d.id IS NOT NULL))) THEN
  RAISE EXCEPTION 'advertisement decision mismatch' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER message_advertisement_config_evidence AFTER INSERT OR UPDATE ON school_message_advertisements
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_check_evidence();
CREATE CONSTRAINT TRIGGER message_advertisement_revision_evidence AFTER INSERT OR UPDATE ON school_message_advertisement_revisions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_check_evidence();
CREATE CONSTRAINT TRIGGER message_advertisement_decision_evidence AFTER INSERT ON school_message_advertisement_decisions
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_check_evidence();

CREATE FUNCTION linjian_message_advertisement_check_snapshot() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE event uuid; item forum_outbox; snapshot forum_message_advertisement_snapshots;
BEGIN
 IF TG_TABLE_NAME='forum_outbox' THEN event:=NEW.id; ELSE event:=NEW.event_id; END IF;
 SELECT * INTO STRICT item FROM forum_outbox WHERE id=event;
 IF TG_TABLE_NAME='forum_message_advertisement_snapshots' AND (item.request_body IS NULL OR item.message_version IS DISTINCT FROM 'summary-20-ad-tail-v1') THEN
  RAISE EXCEPTION 'advertisement snapshot requires a frozen message' USING ERRCODE='23514';
 END IF;
 IF item.request_body IS NULL OR item.message_version IS DISTINCT FROM 'summary-20-ad-tail-v1' THEN RETURN NULL; END IF;
 SELECT * INTO snapshot FROM forum_message_advertisement_snapshots WHERE event_id=event;
 IF snapshot.event_id IS NULL OR snapshot.final_length<>length(item.payload->>'content')
 OR (snapshot.result='applied' AND right(item.payload->>'content',length(snapshot.content)+1) IS DISTINCT FROM E'\n'||snapshot.content)
 OR (snapshot.revision_id IS NOT NULL AND NOT EXISTS(SELECT 1 FROM school_message_advertisement_revisions r
 JOIN school_message_advertisement_decisions d ON d.revision_id=r.id WHERE r.id=snapshot.revision_id
 AND r.status='approved' AND r.content=snapshot.content AND d.decision='approve' AND d.review_digest=r.review_digest)) THEN
  RAISE EXCEPTION 'frozen advertisement snapshot mismatch' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER message_advertisement_outbox_snapshot AFTER INSERT OR UPDATE ON forum_outbox
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_check_snapshot();
CREATE CONSTRAINT TRIGGER message_advertisement_snapshot_evidence AFTER INSERT ON forum_message_advertisement_snapshots
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION linjian_message_advertisement_check_snapshot();

CREATE OR REPLACE FUNCTION forum_permission_catalog() RETURNS jsonb
LANGUAGE sql IMMUTABLE AS $$
 SELECT '[{"id":"P01","key":"content.post.read","name":"查看帖子","description":"管理列表、筛选、详情、待审/隐藏内容、审核记录及授权附件；推荐记录只读，不含重试/终止 Outbox","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P02","key":"content.post.review","name":"审核帖子","description":"通过、驳回帖子；不含修改正文或上下架","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P03","key":"content.post.edit","name":"编辑帖子","description":"修改已有本校帖子的正文、标题、图片与标签；不含平台代发/导入","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P04","key":"content.post.visibility","name":"帖子上架/下架","description":"改变帖子展示状态；不替代审核，不包含删除","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P05","key":"content.post.pin","name":"帖子置顶与排序","description":"置顶、取消置顶及帖子管理排序","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":false},{"id":"P06","key":"content.post.delete","name":"删除帖子","description":"删除已有帖子；不授予恢复或清库能力","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":true},{"id":"P07","key":"post.recommend","name":"推荐/重推帖子","description":"推荐预览、确认提交；仍校验发布证据、频控、幂等和来源就绪条件","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["content.post.read"],"high_risk":true},{"id":"P08","key":"content.comment.read","name":"查看评论","description":"评论管理列表、筛选、正文、审核记录及所属帖子最小上下文","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P09","key":"content.comment.review","name":"审核评论","description":"通过、驳回评论","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":false},{"id":"P10","key":"content.comment.visibility","name":"评论显示/隐藏","description":"隐藏、恢复展示；不等于审核或删除","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":false},{"id":"P11","key":"content.comment.delete","name":"删除评论","description":"管理员删除本校评论；运营获授权也可执行；不影响社区删除本人评论规则","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.comment.read"],"high_risk":true},{"id":"P12","key":"content.case.read","name":"查看举报与反馈工单","description":"case 列表、详情、联系信息及证据；不附赠目标帖子完整管理读取","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P13","key":"content.case.resolve","name":"处理举报与反馈工单","description":"resolved/dismissed 与处理说明；不包含删除工单、直接删帖或封禁","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.case.read"],"high_risk":false},{"id":"P14","key":"operation.qrcode.read","name":"查看群二维码","description":"本校二维码列表、详情和管理图片","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P15","key":"operation.qrcode.create","name":"新增群二维码","description":"新建本校二维码，含该业务必需上传；不含平台默认二维码","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":false},{"id":"P16","key":"operation.qrcode.update","name":"编辑群二维码","description":"修改、启停、展示时机与排序字段","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":false},{"id":"P17","key":"operation.qrcode.delete","name":"删除群二维码","description":"删除本校二维码","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.qrcode.read"],"high_risk":true},{"id":"P18","key":"operation.banner.read","name":"查看首页广告位","description":"本校广告列表、详情、图片和已有曝光/点击汇总","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P19","key":"operation.banner.create","name":"新增首页广告位","description":"新建本校广告和必需上传；只可选择本校已发布可见帖作为落地目标","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":false},{"id":"P20","key":"operation.banner.update","name":"编辑首页广告位","description":"修改、启停、投放时间和排序字段","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":false},{"id":"P21","key":"operation.banner.delete","name":"删除首页广告位","description":"删除本校广告","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.banner.read"],"high_risk":true},{"id":"P22","key":"operation.feedback.read","name":"查看投诉反馈","description":"运营投诉列表、详情及授权图片；与 case 工单区分","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P23","key":"operation.feedback.delete","name":"删除投诉反馈","description":"删除运营投诉记录；不提供 case 删除或用户封禁","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.feedback.read"],"high_risk":true},{"id":"P24","key":"resource.school.read","name":"查看学校展示资源","description":"服务/法律等现有学校资源与管理素材；不授予平台资源编辑","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P25","key":"resource.school.write","name":"配置并发布学校展示资源","description":"现有预览/确认流程中的新增、修改、启停、排序与发布；含所需素材上传，不虚构独立删除/草稿流程","group":"运营展示","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["resource.school.read"],"high_risk":true},{"id":"P26","key":"school.home_menu.read","name":"查看首页菜单/话题","description":"标签菜单、筛选项及现有配置读取","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P27","key":"school.home_menu.write","name":"新增/编辑首页菜单","description":"标签名称、展示状态、启用状态；不包含删除、批量排序或导入","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P28","key":"school.home_menu.delete","name":"删除首页菜单项","description":"删除本校菜单标签","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":true},{"id":"P29","key":"school.home_menu.reorder","name":"调整首页菜单顺序","description":"现有菜单排序提交","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P30","key":"school.home_menu.settings.update","name":"修改首页菜单设置","description":"菜单模块现有独立设置，不等于学校审核策略","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read"],"high_risk":false},{"id":"P31","key":"school.home_menu.import","name":"导入首页菜单","description":"本项目现有导入预览及确认；不连接 wxrobot 数据库或服务","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.home_menu.read","school.home_menu.write"],"high_risk":false},{"id":"P32","key":"school.blocked_word.read","name":"查看屏蔽词","description":"本校词库及对本校生效的平台规则只读","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P33","key":"school.blocked_word.write","name":"新增/编辑屏蔽词","description":"本校词条、备注、启停；不改变平台词库","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":false},{"id":"P34","key":"school.blocked_word.delete","name":"删除屏蔽词","description":"删除本校词条","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":true},{"id":"P35","key":"school.blocked_word.reorder","name":"调整屏蔽词顺序","description":"本校现有排序流程","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read"],"high_risk":false},{"id":"P36","key":"school.blocked_word.import","name":"导入屏蔽词","description":"词库导入预览与确认","group":"菜单与屏蔽词","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.blocked_word.read","school.blocked_word.write"],"high_risk":false},{"id":"P37","key":"school.profile.read","name":"查看学校后台资料","description":"本校后台资料；公开目录另走公开/社区规则","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P38","key":"school.profile.update","name":"修改学校展示资料","description":"学校名称、地区、图标、主题等学校可改字段；不含创建、启停、删除学校","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.profile.read"],"high_risk":false},{"id":"P39","key":"school.review_settings.update","name":"修改帖子/评论审核策略","description":"手动/自动审核设置，字段白名单严格分离","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.profile.read"],"high_risk":true},{"id":"P40","key":"school.user.read","name":"查看本校用户","description":"用户列表、详情、已有状态历史和授权头像；不包含平台范围搜索","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P41","key":"school.user.status.update","name":"封禁/解封本校用户","description":"本校用户状态修改；不含手机号纠正、学校纠正、后台账号启停","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["school.user.read"],"high_risk":true},{"id":"P42","key":"school.member.read","name":"查看本校后台成员与权限","description":"本校后台成员及业务权限，不含全局候选账号、密码和跨校授权","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P43","key":"school.member.recommendation.assign","name":"为运营配置推荐权限","description":"本校管理员的受限推荐授权，不能改其他权限或给本人授权","group":"学校与管理","applicable_roles":["admin"],"default_by_role":{"admin":true,"operator":false},"requires":["school.member.read","post.recommend"],"high_risk":true},{"id":"P44","key":"statistics.school.read","name":"查看本校业务统计","description":"学校统计与当前可读快照；不包含全校汇总、跨校比较","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P45","key":"statistics.school.export","name":"导出本校业务统计","description":"已有学校统计快照 CSV 导出；下载时重查","group":"学校与管理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["statistics.school.read"],"high_risk":false},{"id":"P46","key":"content.simulation.manage","name":"管理仿真发帖","description":"学校管理员及运营发布和管理仿真帖子、模拟作者与批次；不含审核或来源推送","group":"内容治理","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":["content.post.read"],"high_risk":true},{"id":"P47","key":"operation.shortcut.read","name":"服务入口：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P48","key":"operation.shortcut.create","name":"服务入口：创建","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P49","key":"operation.shortcut.update","name":"服务入口：更新","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P50","key":"operation.shortcut.delete","name":"服务入口：删除","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P51","key":"operation.shortcut.preview","name":"服务入口：预览","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.shortcut.read"],"high_risk":false},{"id":"P52","key":"operation.category.read","name":"服务分类：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P53","key":"operation.category.create","name":"服务分类：创建","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.category.read"],"high_risk":false},{"id":"P54","key":"operation.category.update","name":"服务分类：更新","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.category.read"],"high_risk":false},{"id":"P55","key":"operation.category.delete","name":"服务分类：删除","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.category.read"],"high_risk":false},{"id":"P56","key":"operation.report.read","name":"运营报表：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":true},"requires":[],"high_risk":false},{"id":"P57","key":"operation.report.export","name":"运营报表：导出","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":false,"operator":false},"requires":["operation.report.read"],"high_risk":true},{"id":"P58","key":"operation.marketing.read","name":"官方营销：查看","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P59","key":"operation.marketing.update","name":"官方营销：更新","description":"仅限当前学校，不授予平台目标审核或合作凭据访问权限","group":"服务运营","applicable_roles":["admin","operator"],"default_by_role":{"admin":true,"operator":false},"requires":["operation.marketing.read"],"high_risk":false},{"id":"P60","key":"school.message_advertisement.read","name":"消息尾部广告：查看","description":"查看本校广告配置、修订与审核记录，不授予来源路由或发送权限","group":"消息广告","applicable_roles":["admin"],"default_by_role":{"admin":true,"operator":false},"requires":[],"high_risk":false},{"id":"P61","key":"school.message_advertisement.manage","name":"消息尾部广告：管理","description":"管理本校广告草稿、提交、撤回和删除，不包含平台审核","group":"消息广告","applicable_roles":["admin"],"default_by_role":{"admin":true,"operator":false},"requires":["school.message_advertisement.read"],"high_risk":false}]'::jsonb
$$;

CREATE OR REPLACE FUNCTION forum_finalize_prepared_source(event uuid, lease uuid, asset uuid, body text, version text) RETURNS boolean
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
 OR version IS DISTINCT FROM 'summary-20-ad-tail-v1' THEN RAISE EXCEPTION 'invalid frozen message'; END IF;
 UPDATE forum_outbox SET payload=attrs,request_body=body,body_sha256=encode(sha256(convert_to(body,'UTF8')),'hex'),message_version=version,frozen_at=now(),delivery_state='ready' WHERE id=event;
 UPDATE forum_publication_preparations SET state='completed',link_asset_id=asset,completed_at=now(),last_error_code=NULL,lease_token=NULL,lease_until=NULL,revision=revision+1 WHERE event_id=event;
 RETURN true;
END $$;
