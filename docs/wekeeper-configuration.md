# WeKeeper 连接部署配置

Linjian 使用后端部署配置中的 WeKeeper 服务地址、调用方凭据 ID 和签名密钥。一个下游对应一条 WeKeeper 调用方凭据；secret 直接用于 HMAC 签名，不在 Linjian 数据库保存，不需要加解密或 `LINJIAN_SOURCE_MASTER_KEY_V1`。

## Docker 配置

复制 `env.example` 为 `.env`，填写同一条 WeKeeper 凭据：

```dotenv
LINJIAN_WEKEEPER_ORIGIN=https://keeper.example.com
LINJIAN_WEKEEPER_KEY_ID=<WeKeeper 签发的 key_id>
LINJIAN_WEKEEPER_SECRET=<同一条凭据的 secret>
```

地址必须是规范 HTTPS origin，例如 `https://keeper.example.com`，允许显式端口；不要填写接口路径、末尾斜杠、query 或 fragment。不要将 `LINJIAN_PUBLIC_ORIGIN` 填成 WeKeeper 域名：它仍是 Linjian 自己的公开论坛地址，用于帖子详情链接。无需填写 source_id，内部归属由认证响应自动取得。

三项全部留空时不启用连接；部分填写或格式错误时 API 启动会返回明确的配置错误码，不输出配置值。Compose 将三项传给 API 运行时，不传给 Web 或小程序构建。修改 `.env` 后执行：

```sh
docker compose --env-file .env config --quiet
docker compose --env-file .env up -d --build api
```

单纯重启旧容器不会重新读取 `.env`。所有 API 实例使用一致的配置；`.env` 不提交版本库。独立启动二进制或 `make api-dev` 时，需由启动环境导出相同的三项变量；Make 不会自动加载 `.env`。

## 后台使用

“WeKeeper 连接”仅显示当前配置的地址、key_id 和已配置状态，不提供凭据录入或下载。点击“验证连接”检查真实凭据并保存连接元数据，学校消息路由直接使用此唯一全局 WeKeeper 配置，管理员只填写 WeKeeper 为本校分配的 route_id。每所学校只维护一条路由，一个 Linjian 实例只对应一个 WeKeeper，各校使用独立 route_id，不选择或填写连接 ID。修改并保存会更新本校原配置，停用后仍占用本校唯一配置，不新增第二条记录。验证需要现有管理员会话和 CSRF，失败不会保存成功状态。

连接 ID、内部归属、路由绑定、连接版本和消息回执继续保存在数据库。只有与配置中 origin、key_id 相符的连接才能取得签名 secret；更换到另一调用方凭据或另一 WeKeeper 实例不会给历史事件套用新凭据。相同凭据修复配置后可重新验证并恢复可重试队列，事件正文和 receipt_id 保持不变。WeKeeper 临时不可用不会阻止论坛正常写入。

## 已有部署升级

升级前将原连接的 origin、key_id 和原始 secret 填入上述三项。secret 可由 WeKeeper 调用方凭据详情重复查看；不需要从 Linjian 数据库解密导出。新增前向迁移删除旧 encrypted_secret 与 encryption_version 列，保留原连接、学校绑定、历史 Outbox 和回执标识。同一 origin、key_id 验证时复用已有连接。

移除旧 `LINJIAN_SOURCE_MASTER_KEY_V1`。旧 `LINJIAN_SOURCE_ENDPOINT`、`LINJIAN_SOURCE_KEY_ID`、`LINJIAN_SOURCE_SECRET` 不作为 v2 配置使用，统一改用本页三项名称。WeKeeper 自身的存储主密钥配置不受影响，也不应复制到 Linjian。

## 学校路由关系升级

数据库使用 school_id 唯一约束，学校消息路由为单条配置表单；上游 WeKeeper 由全局部署配置唯一确定，route_id 是本校消息的分发目标。学校保存接口只接受 route_id 和启停状态，后端自动解析唯一当前连接。停用既有路由不依赖上游在线或配置可用。更换本校路由只影响后续消息，历史 Outbox 继续使用冻结的原路由和连接版本。

如果旧版本已为同一学校配置多条路由，升级优先保留启用记录，并在同状态记录中保留 ID 最大的一条（旧版本创建使用 UUIDv7）。其余原配置快照保存到 school_source_binding_archive，便于核对，不再参与后续推送。只有一条配置的学校及其他学校不受合并影响；历史连接版本、消息及回执不删除。
