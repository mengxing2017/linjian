# miniapp_users

独立小程序注册资料、私有头像和学校用户管理。domain 定义字段及输入约束，north/local 编排经微信校验后的身份注册，north/remote 提供 JSON:API；south/port 声明注册和资料端口，south/adapter/postgres 实现事务、学校授权、乐观锁及审计。

账号锁在资料锁之前获取，状态和学校变更与会话撤销同事务。论坛小程序写事务通过 guard_write 复查会话和用户归属。用户状态不变更 accounts.active，不授予后台角色，不生成 Outbox。完整入口、迁移及部署说明见 docs/miniapp-users.md。
