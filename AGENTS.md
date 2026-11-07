# Linjian

独立论坛项目，包含 Rust API、React Web 管理后台、uni-app Vue 3 小程序及本项目的 `@linjian/api-contract`。

- 全部论坛功能、学校管理与设置、学校成员与权限、登录账号管理、整个小程序均在本项目实现。
- 不修改 `../wekeeper`；不依赖其源码、工作区、数据库、登录会话或运行服务。
- 不引入手机、目标群、官方群发或发送设备模块。Outbox 是来源推送，不产生发送任务或发送授权。
- Rust 保留 domain、north/local、north/remote、south/port、south/adapter 分层；HTTP 不写 SQL。
- 论坛新库只执行 `backend/migrations/` 中的独立基线与后续 forward-only migration。
- Web 与小程序独立构建，禁止互相 import。只在 feature 的 api/index.ts 引用 contract。
- JSON:API 接口同步维护 Rust 序列化、TypeScript client 与 fixture。保留学校边界、权限、Origin 与 CSRF 校验。
- 浏览器 Cookie、缓存、小程序存储键及环境变量使用 linjian 独立命名空间；不复用旧会话或来源密钥。
- 来源凭据只在后端。正文变更与 Outbox 在同一事务提交；不因浏览、点赞或计数变化推送。接收端故障不影响论坛写入。
- 数据库测试只接受 `LINJIAN_TEST_DATABASE_URL`，无生产地址回退。迁移工具源连接只读，目标必须为空的独立 Linjian 库。
- 研发验收与发布检查不进入业务运行门禁。

验证：`make check-web`、`devbox run -- make check-backend`；隔离数据库执行 `make test-integration`。
