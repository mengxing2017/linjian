# WeKeeper v2 来源适配

在部署配置填写 `LINJIAN_WEKEEPER_ORIGIN`、`LINJIAN_WEKEEPER_KEY_ID` 和 `LINJIAN_WEKEEPER_SECRET`，同时填写或全部留空。secret 只从后端环境读取用于签名，不入库、不解密，不需要 Linjian 存储主密钥。规范 HTTPS origin 禁止路径、用户信息、query、fragment 和重定向；配置解析与连接匹配在 `configuration.rs`。配置 HTTPS `LINJIAN_PUBLIC_ORIGIN` 为本论坛地址，用于冻结内容详情链接。

后台“WeKeeper 连接”只查看配置和验证凭据，不录入 secret。验证认证响应后自动获取内部归属并保存不含密钥的连接元数据，复用相同 origin、key_id 的既有连接。启动前校验部署配置格式，不在启动时依赖上游网络可用性。部署及升级步骤见 `docs/wekeeper-configuration.md`。

在“学校消息路由”只配置 route_id 和启停状态。Linjian 实例只对应全局部署配置中的一个 WeKeeper，后端自动解析该配置对应的连接，不接受学校请求提交 connection_id。学校与消息路由一一对应，所有学校路由共用同一个全局 WeKeeper。school_id 使用数据库唯一约束，保存按 school_id upsert，更新时保留绑定 ID；GET 返回单个可空 binding，而非路由集合。一个路由不能跨学校复用，WeKeeper 不保存学校对象。启用时检查当前 key 的真实路由范围；停用绑定不改变旧事件。

帖子事务产生 v2 Outbox 时冻结连接版本、来源、route_id、正文、event_id 和摘要。收到响应前崩溃可用新 nonce 重试原文。已保存连接的 key_id 保持不变；修复同一凭据的配置不会重写队列正文或 receipt_id。

存量 v1 保留原样并标记 legacy_version_blocked，v1 fixture 保留。当前接收服务只发布 v2，不能把历史正文静默改为 v2。配置与学校路由明确绑定，历史事件不套用其他 origin 或 key_id 的凭据。

认证失败/路由撤销/版本不支持进入阻塞；修复后可重试同事件，保存修复后的连接会唤醒同来源可恢复队列。429、服务错误及协议错误退避；连续时间窗口/nonce 错误限制自动恢复次数。收到 rejected 或 ignored_stale 停止无效重投；applied 不等于发送成功，继续获取摘要。

POST `/api/v2/source-connections` 使用空属性文档验证当前部署配置并保存连接元数据，GET 同路径读取公开配置和已验证连接；GET/POST `/api/v2/schools/{school}/source-bindings` 查询/维护绑定。使用本项目管理会话及 CSRF，WeKeeper key 无权调用管理 API。类型和字节 fixture 在本项目 `api-contract/source-v2/` 与 `api-contract/fixtures/source-v2.json`。

应用和集成测试不读取 WeKeeper 工作区、数据库或会话。数据库测试只使用隔离 LINJIAN_TEST_DATABASE_URL；`tests/source_v2.sql` 验证冻结与学校唯一绑定，不触发网络发送。

所有网络调用由 Linjian 发起：认证、路由检查、消息推送及回执查询。WeKeeper 不回调、不探测 Linjian 在线状态。
