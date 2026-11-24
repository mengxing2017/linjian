# Linjian 文档

Linjian 是包含 Rust API、Web 管理后台与微信小程序的独立校园论坛。

## 部署与运维

- [Docker 构建与启动](runbooks/docker-images.md)：配置、构建、创建管理员和打开后台。
- [来源连接配置](runbooks/wekeeper-configuration.md)：`make dev-embedded` 环境变量、Docker 配置与连接验证。
- [数据迁移](runbooks/migration.md)：快照导出、空库导入、切换及回退。

## 功能说明

- [学校信息与主题](guide/school-profiles.md)
- [小程序用户与登录](guide/miniapp-users.md)
- [协议管理与首次入驻](guide/legal-documents-onboarding.md)
- [内容全文审核](guide/content-review.md)
- [仿真发帖](guide/simulated-posts.md)
- [小程序内容来源推送](guide/miniapp-publication.md)
- [运营中心](guide/operations.md)
- [统计概览](guide/business-statistics.md)

## 开发参考

- [来源推送协议](references/source-protocol.md)

## 架构决策

- [独立论坛架构](adr/0001-independent-forum.md)

## 目录约定

| 目录 | 内容 |
| --- | --- |
| `runbooks/` | 构建部署、接入配置、数据迁移等操作手册。 |
| `guide/` | 学校、用户、审核和运营等功能使用说明。 |
| `references/` | 接口协议等开发参考。 |
| `adr/` | 架构决策。 |
