# WeKeeper 连接部署配置

Linjian 使用后端部署配置中的 WeKeeper 服务地址、调用方凭据 ID 和签名密钥。一个下游对应一条 WeKeeper 调用方凭据；secret 直接用于 HMAC 签名，不在 Linjian 数据库保存，不需要加解密或 `LINJIAN_SOURCE_MASTER_KEY_V1`。

## 本地开发：make dev-embedded

`make dev-embedded`（或 `devbox run -- make dev-embedded`）从启动进程的环境读取配置，**不会自动加载项目根目录的 `.env`**。只在 `.env` 中填写变量不会生效；需要先在启动 API 的同一终端中导出：

```sh
export LINJIAN_WEKEEPER_ORIGIN='https://keeper.example.com'
export LINJIAN_WEKEEPER_KEY_ID='<WeKeeper 签发的 key_id>'
export LINJIAN_WEKEEPER_SECRET='<同一条凭据的 secret>'
devbox run -- make dev-embedded
```

运行前将示例地址和占位符替换为实际值。已进入 Devbox shell 或本机已安装所需 Rust 工具链时，最后一行可直接使用 `make dev-embedded`。

| 环境变量 | 配置说明 |
| --- | --- |
| `LINJIAN_WEKEEPER_ORIGIN` | WeKeeper 的规范 HTTPS origin，例如 `https://keeper.example.com`；允许非默认端口，不带路径、末尾斜杠、用户信息、query 或 fragment。 |
| `LINJIAN_WEKEEPER_KEY_ID` | 从 WeKeeper“调用方凭据”复制的 `key_id`，与下面的 secret 属于同一条凭据。长度为 1–128 个字符，只含英文字母、数字、`-` 和 `_`。 |
| `LINJIAN_WEKEEPER_SECRET` | 从同一条调用方凭据复制的原始 secret；格式为无填充的 Base64URL，解码后至少 32 字节。不要填写普通密码、存储主密钥或自行编造的示例字符串。 |

三项必须同时填写，或全部留空。只配置 `LINJIAN_WEKEEPER_KEY_ID` 和 `LINJIAN_WEKEEPER_SECRET` 而遗漏 origin 也会导致启动失败。凭据仅供后端使用，不放入 `VITE_*`、Web 或小程序配置，不提交到版本库。管理入口使用 `LINJIAN_ADMIN_WEB_ORIGIN` 和 `LINJIAN_BROWSER_ORIGINS`，与 WeKeeper 地址独立。连接验证不读取或校验前端入口。

如已将配置保存在本地 `.env` 文件中，可在同一终端用以下方式显式加载。仅加载自己维护且可信的文件，`source` 会执行其中的 shell 内容；值包含 shell 特殊字符时须正确加引号。

```sh
set -a
source ./.env
set +a
devbox run -- make dev-embedded
```

暂不接入 WeKeeper 时，可清除当前终端中遗留的三项配置再启动，论坛仍可正常开发：

```sh
unset LINJIAN_WEKEEPER_ORIGIN LINJIAN_WEKEEPER_KEY_ID LINJIAN_WEKEEPER_SECRET
devbox run -- make dev-embedded
```

修改变量后需要停止并重新启动 API。嵌入式入口每次创建临时数据库，退出后删除数据，因此重启后还需在管理后台重新“验证连接”并配置学校消息路由；环境凭据不会代替这些步骤。

### 启动配置排查

| 错误码或现象 | 处理方式 |
| --- | --- |
| `wekeeper_configuration_incomplete` | 检查同一启动终端是否已导出全部三项；不接入时全部清空。 |
| `wekeeper_origin_invalid` | 使用规范 HTTPS origin，移除路径和末尾斜杠等额外部分。 |
| `wekeeper_key_id_invalid` | 重新复制调用方凭据的 key_id，检查字符、长度及多余空白。 |
| `wekeeper_secret_invalid` | 重新复制同一凭据的完整 secret，检查格式及多余空白。 |
| `authentication_failed` | 上游返回非成功状态；查看 API 日志中的 `upstream_status`，核对调用方凭据、授权、两端时钟及反向代理。 |
| `temporarily_unavailable` | 查看 API 日志，网络失败记录 `timeout`、`connect` 等类别；数据库失败记录 SQLSTATE。检查网络、TLS、数据库后重试。 |
| `protocol_error` / `unsupported_version` | 检查 WeKeeper 地址是否指向来源 v2 API，以及反向代理是否返回 HTML 或改写响应。 |
| 后台显示未配置 | 确认变量已 `export` 或显式加载 `.env`，并重启 API；仅编辑文件不会更新运行进程。 |

验证失败会在 API 日志的 `verify_wekeeper_connection` 操作中记录错误码；上游拒绝请求还会记录 HTTP 状态码和请求 ID。默认日志级别即可查看，无需开启 HTTP 请求正文或请求头调试；日志不记录 secret、签名或上游响应正文。嵌入式开发查看启动终端，Docker 部署使用 `docker compose --env-file .env logs --tail=100 api`。

## Docker 配置

复制 `env.example` 为 `.env`，填写同一条 WeKeeper 凭据：

```dotenv
LINJIAN_WEKEEPER_ORIGIN=https://keeper.example.com
LINJIAN_WEKEEPER_KEY_ID=<WeKeeper 签发的 key_id>
LINJIAN_WEKEEPER_SECRET=<同一条凭据的 secret>
```

地址必须是规范 HTTPS origin，例如 `https://keeper.example.com`，允许显式端口；不要填写接口路径、末尾斜杠、query 或 fragment。不要将 Linjian 的管理、H5 或公开 API 地址填成 WeKeeper 域名；小程序内容链接由发布流程独立生成。无需填写 source_id，内部归属由认证响应自动取得。

三项全部留空时不启用连接；部分填写或格式错误时 API 启动会返回明确的配置错误码，不输出配置值。Compose 将三项传给 API 运行时，不传给 Web 或小程序构建。修改 `.env` 后执行：

```sh
docker compose --env-file .env config --quiet
docker compose --env-file .env up -d --build api
```

单纯重启旧容器不会重新读取 `.env`。所有 API 实例使用一致的配置；`.env` 不提交版本库。独立启动二进制或 `make api-dev` 时，需由启动环境导出相同的三项变量；Make 不会自动加载 `.env`。

## 连接验证与内容发布的边界

来源连接只配置 WeKeeper 地址、key_id 和 secret。无需额外配置论坛详情链接。`LINJIAN_ADMIN_WEB_ORIGIN` 与 `LINJIAN_BROWSER_ORIGINS` 用于管理浏览器 Origin/CSRF，必须与实际访问来源一致；本地 HTTP 开发无需为了验证上游凭据改成 HTTPS。

来源推送只投递本项目构造的完整 content（摘要加换行和原始链接），不发送 detail_url 或独立标题字段，不校验内容 URL、不探测链接或判断发送资格。连接验证不依赖内容地址。上游拒绝保留实际错误码，不代表凭据验证失败。

从空的独立 Linjian 数据库执行全部 forward-only migration。消息构造使用审核快照，不读取上游源码或运行数据。所有 API 实例使用一致的浏览器 Origin/CSRF 配置与来源凭据。

## 后台使用

“WeKeeper 连接”仅显示当前配置的地址、key_id 和已配置状态，不提供凭据录入或下载。点击“验证连接”检查真实凭据并保存连接元数据，学校消息路由直接使用此唯一全局 WeKeeper 配置，管理员只填写 WeKeeper 为本校分配的 route_id。每所学校只维护一条路由，一个 Linjian 实例只对应一个 WeKeeper，各校使用独立 route_id，不选择或填写连接 ID。修改并保存会更新本校原配置，停用后仍占用本校唯一配置，不新增第二条记录。验证需要现有管理员会话和 CSRF，失败不会保存成功状态。

连接 ID、内部归属、路由绑定、连接版本和消息回执继续保存在数据库。只有与配置中 origin、key_id 相符的连接才能取得签名 secret；更换到另一调用方凭据或另一 WeKeeper 实例不会给历史事件套用新凭据。相同凭据修复配置后可重新验证并恢复可重试队列，事件正文和 receipt_id 保持不变。WeKeeper 临时不可用不会阻止论坛正常写入。

## 学校路由与暂停

学校路由保存使用 `If-Match: "<revision>"`；未配置学校使用 `"0"`。请求 JSON:API 属性只接受 route_id 和 active。修订冲突返回 409，缺少修订返回 428。保存后立即生效，无需重启服务；全局环境变更仍需重建 API 容器，所有实例配置必须一致。

无路由时，新内容以待绑定意图留存。配置完成后后台绑定并准备正文。暂停推送保留未交付意图，恢复后继续处理；帖子下架或学校停用取消从未尝试交付的意图。路由变更取消旧路由上尚未尝试交付的意图，只影响之后的新发布；已受理记录保留，不补发历史内容，不发送撤回。

来源消息通过短事务租约领取，网络调用期间不持有数据库事务。租约到期使用原事件与原字节重试，过期 worker 的结果不能覆盖新进展。已尝试但结果未知的消息继续使用原路由核对，避免误判为未交付。

消息由 Linjian 自行构造为最多 20 字的摘要，按需加换行和原始小程序链接，完整正文最多 200 个 Unicode 标量。来源发件页展示准备状态、冻结正文、受理时间与原因；合法 received 回执即完成来源投递，不等待发送结果。修复可恢复错误后填写原因重试；确定性协议错误与冻结内容冲突不能强制解除。

## 初始化与运行检查

本项目尚未上线且没有历史数据，不提供旧协议转换、回填或双协议运行分支。在空库执行完整基线序列，其中 `0006_source_outbox.sql` 定义来源状态，`0008_foreign_keys.sql` 与 `0009_functions_triggers.sql` 创建关系约束及冻结函数。基线整理不兼容旧迁移记录，后续数据库变更只追加迁移；论坛写入与发布意图保持事务原子性。

监控按学校统计准备状态、Outbox 状态与最老积压，不记录密钥、签名或完整正文。研发验收在隔离数据库执行，不作为业务运行门禁；不依赖上游工作区或真实发送服务。
