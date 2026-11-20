# 学校成员与业务权限

学校角色与业务权限分离：`admin`、`operator` 是后台职责及默认权限来源，`member` 只保留基础社区关系，`revoked` 表示撤销学校关系。后台成员不等于小程序用户；小程序用户和社区归属仍在用户管理维护。

## 平台配置

平台管理员进入「学校成员与权限」，选择本校已有后台成员，或使用「添加后台成员」查询后台候选账号。搜索、角色、单独配置筛选和分页均由服务端处理，不下载全部账号进行前端筛选。平台身份不能通过学校单独禁止来限制。

选定学校管理员或运营角色后，页面按业务分组展示 45 个权限项。每项有默认值、说明、依赖、风险标记和最终结果：

- 两个勾选框都不选：继承角色默认。
- 单独允许：明确授予该项。
- 单独禁止：明确拒绝该项，包括学校管理员原本默认允许的权限。
- 角色不适用的项目不能勾选；运营不能获得受限推荐授权权。

快捷组合只是勾选辅助，会先预览，不产生新角色。全部允许、全部禁止或全部继承也只是修改草稿，必须填写原因、核对差异并确认保存后才生效。依赖不会自动授予；存在缺失依赖时不能保存。

保存是角色与完整覆盖集合的一次原子替换，必须同时携带权限目录版本和授权修订版本。授权版本为十进制字符串，不转换成 JavaScript 数字。遇到冲突保留草稿，读取最新配置后人工核对再提交；不会自动重试或覆盖他人更新。

「取消后台职责」降为基础关系；「撤销学校关系」设置为撤销关系。两者都会清除权限覆盖并递增版本。重新授予时从新角色默认值开始，不恢复之前的覆盖。当前小程序社区归属不能在此撤销。平台可以恢复没有可用本地管理员的学校。

## 学校管理员

通用角色和权限配置只允许平台修改。拥有 `school.member.read` 的账号可查看本校成员与业务权限，不能查询全局候选账号。学校管理员还必须同时拥有 `school.member.recommendation.assign` 和 `post.recommend`，才可单独调整本校其他运营的推荐权限；不得给本人授权，也不能通过此入口改变其他覆盖项。

学校消息路由与操作审计仅对平台管理员开放，不属于学校可配置权限；学校管理员和运营均不显示入口，也不能通过直接调用接口访问。

## 接口与边界

- `GET /api/v1/schools/{school}/permission-catalog`：版本化权限目录。
- `GET /api/v1/schools/{school}/permissions/me`：当前后台身份的有效权限。
- `GET /api/v1/schools/{school}/members`：本校后台成员；`member-candidates` 是平台专用独立查询。
- `GET /api/v1/schools/{school}/members/{account}/permissions`：目标权限快照。
- `PUT /api/v1/schools/{school}/members/{account}/authorization`：完整替换角色与覆盖。
- `POST /api/v1/schools/{school}/members/{account}/authorization-actions`：独立取消职责或撤销关系。
- `PUT /api/v1/schools/{school}/members/{account}/recommendation-capability`：版本化受限推荐调整，使用 `effect=inherit|allow|deny`，不接受旧布尔开关。

浏览器菜单和按钮按实际权限展示；服务端对实际动作独立鉴权，不能靠隐藏按钮实现安全。写事务先取得学校授权栅栏，再复查有效账号、会话、当前权限和业务版本。授权变更取得排他栅栏，业务写取得共享栅栏；撤销完成后，排队请求不能依靠旧权限继续提交。

素材选择与上传的 `context` 绑定实际业务动作（如 `operation.banner.create`、`operation.banner.update`、`school.profile.update`），不能借任意写权限进入通用素材库。列表和读取按用途及学校隔离。展示资料与审核策略字段分别验证 `school.profile.update` 与 `school.review_settings.update`。

授权、审计与版本在同一事务内提交；失败不会部分生效。授权变化本身不修改正文、不产生 Outbox 事件、不授予来源恢复或发送权限。

## 部署与验收

执行新增 forward-only migration `202611200001_member_permissions.sql`，不修改历史基线。没有旧业务数据的转换、旧权限表兼容读或双写；旧推荐能力表由新模型替代。学校授权栅栏是派生同步结构，不代表继承旧授权。API、contract 和 Web 应同步部署。

离线快照版本为 `5`，包含授权修订、覆盖、审计和素材用途，不迁移会话；派生栅栏在创建学校时生成。旧版本快照直接拒绝。

验收运行 `make check-web`、`devbox run -- make check-backend`。数据库测试只使用显式 `LINJIAN_TEST_DATABASE_URL`，或 `devbox run -- python3 tools/test_database.py --bin-dir <PostgreSQL-17-bin>` 创建的临时隔离实例。研发检查不是业务运行门禁。
