# Linjian

独立校园论坛，使用 Rust + Axum + PostgreSQL、React Web 和 uni-app Vue 3 微信小程序。论坛业务来自原项目，代码和依赖已复制到本仓库，原项目保持不变。

本项目包含：帖子与评论、点赞收藏关注、通知、举报反馈、媒体、审核、展示资源与屏蔽词、平台默认展示配置、学校管理、学校设置、学校成员与权限、小程序用户管理、登录账号管理及平台管理员管理。整个 `frontend/apps/miniapp/` 均为本项目的论坛客户端；文件对应清单见 [小程序迁移清单](docs/miniapp-migration-manifest.json)。

不包含微信群发送、手机设备、目标群或官方群发任务。论坛通过持久 Outbox 提供来源事件，发送运营仍在独立转发项目中预览并人工确认。

小程序注册、学校用户管理、封禁与权限隔离见 [小程序用户管理与登录](docs/miniapp-users.md)。

学校资料、正常/隐藏状态、品牌图片和小程序三色主题见 [学校信息与主题配置](docs/school-profiles.md)。

内容审核字段、全文展示与版本冲突处理见 [内容全文审核](docs/content-review.md)。

小程序人工审核后的摘要与短链接准备、投递和恢复配置见 [小程序内容来源推送](docs/miniapp-publication.md)。

## 目录

- `backend/`：独立 API、论坛数据库基线、Outbox 作业、只读快照导出及原子导入工具。
- `frontend/apps/web/`：社区及论坛管理后台。
- `frontend/apps/miniapp/`：完整微信小程序，独立构建并发布到微信平台。
- `api-contract/`：本仓库的 `@linjian/api-contract`，含平台中立 client 和共享 fixture。
- `docs/`：架构、迁移、接入协议与验证记录。

## 本地开发

环境版本由现有 `devbox.json` 固定。运行：

```sh
make install
devbox run -- make dev-embedded
make web-dev
```

Web 默认 `http://localhost:5173`；API 默认 `127.0.0.1:8080`。嵌入式开发入口创建随机端口与密码的临时 PostgreSQL 17，忽略外部数据库配置，退出删除数据；首次可能下载数据库程序。若与原项目同时开发，可设置独立端口，并将 `LINJIAN_PUBLIC_ORIGIN`、`LINJIAN_BIND_ADDR`、`LINJIAN_API_ORIGIN` 与 Vite 端口同步调整。

也可使用自己的独立数据库，设置 `LINJIAN_DATABASE_URL` 和 `LINJIAN_PUBLIC_ORIGIN` 后运行 `devbox run -- make api-dev`。管理员创建使用 `cargo run --manifest-path backend/Cargo.toml --bin linjian-api -- --create-admin <username>`，密码从 stdin 读取。生产不启用开发账号引导。

浏览器 Cookie 名为 `linjian_session`，小程序存储键为 `linjian:miniapp:session`，独立数据库保存新会话；旧项目 Cookie 和会话不会迁入。

## 验证

```sh
make check-web
devbox run -- make check-backend
make check-miniapp
```

数据库联机集成测试只接受显式隔离地址，不读取生产连接：

```sh
LINJIAN_TEST_DATABASE_URL=postgresql://... devbox run -- make test-integration
# 或启动仅监听临时 Unix socket 的测试实例：
LINJIAN_POSTGRES_BIN=/path/to/postgresql17/bin devbox run -- python3 tools/test_database.py
# 不需要监听套接字的真实迁移与事务验证：
LINJIAN_POSTGRES_BIN=/path/to/postgresql17/bin python3 tools/test_migrations.py
```

联机测试创建随机 source/target schema，验证完整数据导入核对、权限、登录隔离、Origin/CSRF、历史不推送和发送路由不存在。测试无需启动转发服务。

## 部署

复制 `env.example` 到 `.env`，配置独立域名、数据库密码和平台应用凭据。执行 `docker compose --env-file .env config --quiet` 核对配置，再构建部署。Compose 只有 `web`、`api`、`postgres`，独立卷名称以 `linjian` 为前缀。Web 镜像仅构建 contract 和 Web，小程序不进入服务镜像。

生产部署由 HTTPS 域名终止访问；LINJIAN_PUBLIC_ORIGIN 必须与实际浏览器 Origin 一致。默认宿主机端口为 8091/8444，可由现有反向代理转发或显式设置为域名对应的 80/443。API 和 PostgreSQL 不映射公共宿主机端口。

小程序：填写 `frontend/apps/miniapp/src/manifest.json` 的 AppID，设置其 `.env.example` 中的 HTTPS API 域名为独立论坛地址，执行 `make check-miniapp`。用微信开发者工具导入 `frontend/apps/miniapp/dist/build/mp-weixin`；发布前配置微信平台合法请求域名。AppSecret 仅设置在后端，不进入 VITE_* 或构建产物。

## 迁移与推送

[迁移操作说明](docs/migration.md)提供快照导出、空库导入、媒体和权限核对、最终切换及回退步骤。快照包含密码哈希和媒体，文件权限为 0600，不能提交到仓库。

Outbox 在帖子事务中生成事件，后台每 5 秒扫描，失败指数退避并持久保留。接收端未就绪时将 `LINJIAN_WEKEEPER_ORIGIN`、`LINJIAN_WEKEEPER_KEY_ID`、`LINJIAN_WEKEEPER_SECRET` 全部留空，论坛正常运行。填写三项并重建 API 后，在“WeKeeper 连接”验证配置、绑定学校路由；secret 只保存在后端部署配置，不入库，不需要本地加密主密钥。详见 [WeKeeper 部署配置](docs/wekeeper-configuration.md)。接入后在管理后台“来源发件记录”查看收件与发送摘要、重试原事件。来源密钥不等于发送授权。

协议、请求头及跨语言签名向量见 [来源推送协议](docs/source-protocol.md)。实现与尚待实际环境验证的事项见 [迁移状态](docs/migration-status.md)。
