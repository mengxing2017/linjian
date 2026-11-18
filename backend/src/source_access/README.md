# WeKeeper v2 来源适配

在部署配置填写 `LINJIAN_WEKEEPER_ORIGIN`、`LINJIAN_WEKEEPER_KEY_ID` 和 `LINJIAN_WEKEEPER_SECRET`，同时填写或全部留空。secret 只从后端环境读取用于签名，不入库、不解密，不需要 Linjian 存储主密钥。规范 HTTPS origin 禁止路径、用户信息、query、fragment 和重定向；配置解析与连接匹配在 `configuration.rs`。连接认证不依赖 `LINJIAN_ADMIN_WEB_ORIGIN`；该变量和 LINJIAN_BROWSER_ORIGINS 用于管理浏览器 Origin/CSRF。来源推送将完整 `content` 中的链接作为不透明正文数据，不生成或校验内容 URL，不探测链接，也不判断发送资格；链接由内容发布投影提供。

后台“WeKeeper 连接”只查看配置和验证凭据，不录入 secret。验证认证响应后自动获取内部归属并保存不含密钥的连接元数据，复用相同 origin、key_id 的既有连接。启动前校验部署配置格式，不在启动时依赖上游网络可用性。部署及升级步骤见 `docs/runbooks/wekeeper-configuration.md`。

在“学校消息路由”只配置 route_id 和启停状态。Linjian 实例只对应全局部署配置中的一个 WeKeeper，后端自动解析该配置对应的连接，不接受学校请求提交 connection_id。学校与消息路由一一对应，所有学校路由共用同一个全局 WeKeeper。school_id 使用数据库唯一约束，保存按 school_id upsert，更新时保留绑定 ID；GET 返回单个可空 binding，而非路由集合。一个路由不能跨学校复用，WeKeeper 不保存学校对象。启用时检查当前 key 的真实路由范围；暂停绑定保留未交付意图，恢复后继续准备或交付。

帖子正文事务只写发布意图，后台从审核快照构造最多 20 个 Unicode 标量的摘要，按需附加换行和真实小程序链接，完整 content 最多 200 个标量。冻结时一次序列化六字段 v2 消息并保存哈希、连接版本和路由。重试使用同一 event_id 和原始字节，每次重新生成 nonce 与签名。

只发送 content.upsert。未尝试交付的失效意图本地取消，不向上游发送撤回；已尝试但结果未知的请求保留原文核对，收到合法最小 received 回执后进入 accepted，不继续轮询发送状态。路由变更不批量补发已受理内容。

认证失败、路由撤销、确定性协议错误进入阻塞。429、服务错误及协议响应错误退避；连续时间窗口/nonce 错误限制自动恢复次数。修复可恢复错误后可重试同事件，冻结内容不能修改。保留上游实际错误码，受理成功不代表实际发送成功。

POST `/api/v2/source-connections` 使用空属性文档验证当前部署配置并保存连接元数据，GET 同路径读取公开配置和已验证连接；GET/POST `/api/v2/schools/{school}/source-bindings` 查询/维护绑定。使用本项目管理会话及 CSRF，WeKeeper key 无权调用管理 API。类型和字节 fixture 在本项目 `api-contract/source-v2/` 与 `api-contract/fixtures/source-v2.json`。

应用和集成测试不读取 WeKeeper 工作区、数据库或会话。数据库测试只使用隔离 LINJIAN_TEST_DATABASE_URL；`tests/forum_database.rs` 验证完整消息冻结、受理、幂等租约、取消与学校边界，不触发真实网络发送。

所有网络调用由 Linjian 发起：认证、路由检查、消息推送及回执查询。WeKeeper 不回调、不探测 Linjian 在线状态。
