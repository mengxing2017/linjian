# Docker 镜像打包与部署

命令均从 Linjian 仓库根目录执行。先按第一节在本机跑通；发布到服务器时再执行第二节。交互输入密码的命令使用 Bash，zsh 用户先执行 `bash`。

需要已启动的 Docker Engine、支持 `up --wait` 的 Compose v2 和 BuildKit。构建需要联网下载基础镜像、Debian、Cargo 和 npm 依赖；宿主机无需安装 Rust、Node.js 或 pnpm。

```bash
docker version
docker compose version
docker buildx version
```

## 1. 本机：构建镜像并登录后台

### 准备配置

首次复制，已有 `.env` 时直接编辑，不覆盖：

```bash
cp env.example .env
chmod 600 .env
```

打开 `.env`，修改下面这些项。数据库密码使用你自己生成的长随机密码，例如运行 `openssl rand -hex 32`，把输出保存到密码项。其余可选平台凭据暂时留空。

```dotenv
LINJIAN_DATABASE_PASSWORD=替换为你生成的数据库密码
LINJIAN_API_IMAGE=linjian-api:local
LINJIAN_WEB_IMAGE=linjian-web:local
LINJIAN_POSTGRES_IMAGE=postgres:17-bookworm
LINJIAN_WEB_BIND_HOST=127.0.0.1
LINJIAN_HTTP_PORT=8091
LINJIAN_HTTPS_PORT=8444
LINJIAN_WEB_LISTEN=:80
LINJIAN_ADMIN_WEB_ORIGIN=http://127.0.0.1:8091
LINJIAN_BROWSER_ORIGINS=["http://127.0.0.1:8091"]
LINJIAN_API_PUBLIC_ORIGIN=http://127.0.0.1:8091
LINJIAN_INSECURE_LOCAL_AUTH=true
```

上传文件由 API 写入 `/data/media`，Compose 将独立的 `media-data` 命名卷挂载到该目录。镜像已为应用用户准备写权限，无需 TOML 配置文件或配置文件挂载。上传限额及清理期限通过 `.env` 中的 `LINJIAN_MEDIA_*` 变量配置，修改后重新创建 API 容器生效；容器内目录固定为 `/data/media`，修改路径时须同时修改卷挂载。

### 构建并等待启动完成

依次执行，前一条成功后再执行下一条：

```bash
docker compose --env-file .env config --quiet
docker compose --env-file .env build --pull
docker compose --env-file .env up -d --no-build --wait --wait-timeout 180
docker compose --env-file .env ps
curl --fail --show-error --retry 10 --retry-connrefused --retry-delay 2 http://127.0.0.1:8091/readyz
```

有 Make 时，前三条也可以合并为 `make compose-up`。首次 Rust 编译可能较久；180 秒等待从启动容器时开始计算，不包含构建时间。

成功结果：`postgres`、`api` 为 `healthy`，`web` 运行中，最后一条 curl 返回 HTTP 200。等待超时或命令报错时先查看日志：

```bash
docker compose --env-file .env logs --tail=100 postgres api web
```

镜像构建上下文均为仓库根目录。构建完成后，本机镜像名为 `linjian-api:local`、`linjian-web:local`。数据库使用官方 PostgreSQL 镜像。小程序单独构建，不进入这两个业务镜像。

### 创建第一个管理员

在宿主机 Bash 终端执行，输入 12–128 字节的密码并回车。密码输入时不显示：

```bash
read -r -s -p '管理员密码（12–128 字节）：' LINJIAN_ADMIN_PASSWORD
printf '\n'
printf '%s' "$LINJIAN_ADMIN_PASSWORD" |
  docker compose --env-file .env exec -T api /usr/local/bin/linjian-api --create-admin admin
unset LINJIAN_ADMIN_PASSWORD
```

看到 `Administrator created` 后，打开 `http://127.0.0.1:8091/admin`，使用 `admin` 和刚输入的密码登录。

数据库账号默认为 `linjian`，可通过 `.env` 的 `LINJIAN_DATABASE_USER` 配置，其密码来自 `LINJIAN_DATABASE_PASSWORD`；后台管理员是上面手动创建的账号，两者不能混用。Docker 首次启动不会自动创建管理员。同名账号已存在时创建失败，不会重置旧密码。

## 2. 服务器：使用制作好的镜像

### 在构建机制作和推送

下面三个值替换为自己的镜像仓库、登录地址和本次版本号。API 与 Web 使用同一个发布版本：

```bash
export LINJIAN_RELEASE_REGISTRY=registry.example.com/team
export LINJIAN_RELEASE_VERSION=2026.11.15-1
export LINJIAN_API_IMAGE="$LINJIAN_RELEASE_REGISTRY/linjian-api:$LINJIAN_RELEASE_VERSION"
export LINJIAN_WEB_IMAGE="$LINJIAN_RELEASE_REGISTRY/linjian-web:$LINJIAN_RELEASE_VERSION"
docker login registry.example.com

docker build --pull -f backend/Dockerfile -t "$LINJIAN_API_IMAGE" .
docker build --pull -f frontend/Dockerfile -t "$LINJIAN_WEB_IMAGE" .
docker push "$LINJIAN_API_IMAGE"
docker push "$LINJIAN_WEB_IMAGE"
```

两个 Dockerfile 的构建上下文均为末尾的 `.`，不要在应用子目录执行。版本标签不要覆盖旧发布。以上命令构建本机架构，构建机与目标服务器应使用相同 CPU 架构。

数据库密码、应用 Secret 和域名不参与构建，在服务器运行时配置。Web 请求同源 `/api`，不需要设置生产 Vite API 地址。

### 在服务器配置

服务器只需 `compose.yaml`、`env.example`、Docker 和镜像仓库访问权限，不需要复制源码，也不需要覆盖文件。

```bash
cp env.example .env
chmod 600 .env
```

编辑 `.env`。镜像名必须与上一步推送的名称完全一致；数据库密码换成自己的长随机密码，域名换成已解析到服务器的实际域名：

```dotenv
LINJIAN_API_IMAGE=registry.example.com/team/linjian-api:2026.11.15-1
LINJIAN_WEB_IMAGE=registry.example.com/team/linjian-web:2026.11.15-1
LINJIAN_POSTGRES_IMAGE=postgres:17-bookworm
LINJIAN_DATABASE_PASSWORD=替换为你生成的数据库密码
LINJIAN_WEB_BIND_HOST=0.0.0.0
LINJIAN_HTTP_PORT=80
LINJIAN_HTTPS_PORT=443
LINJIAN_WEB_LISTEN=forum.example.com
LINJIAN_ADMIN_WEB_ORIGIN=https://forum.example.com
LINJIAN_BROWSER_ORIGINS=["https://forum.example.com"]
LINJIAN_API_PUBLIC_ORIGIN=https://api.example.com
LINJIAN_API_LISTEN=api.example.com
LINJIAN_INSECURE_LOCAL_AUTH=false
```

确保公网 80/443 可达且未被其他服务占用，Caddy 自动申请 HTTPS 证书。然后执行：

```bash
docker login registry.example.com
docker compose --env-file .env config --quiet
docker compose --env-file .env pull
docker compose --env-file .env up -d --no-build --wait --wait-timeout 180
docker compose --env-file .env ps
curl --fail --show-error --retry 10 --retry-connrefused --retry-delay 2 https://forum.example.com/readyz
```

将 curl 中域名也替换为实际域名。首次部署按第一节的相同命令创建管理员，再访问 `https://你的域名/admin`。

`--no-build` 保证部署机直接运行镜像；虽然主 Compose 文件保留源码构建配置，此流程不会使用它。

### 已有 Nginx / 1Panel 终止 HTTPS

如果服务器已有 HTTPS 入口，不占用其 80/443。把上面的入口配置换成：

```dotenv
LINJIAN_WEB_BIND_HOST=127.0.0.1
LINJIAN_HTTP_PORT=8091
LINJIAN_HTTPS_PORT=8444
LINJIAN_WEB_LISTEN=:80
LINJIAN_ADMIN_WEB_ORIGIN=https://forum.example.com
LINJIAN_BROWSER_ORIGINS=["https://forum.example.com"]
LINJIAN_API_PUBLIC_ORIGIN=https://api.example.com
LINJIAN_API_LISTEN=api.example.com
LINJIAN_INSECURE_LOCAL_AUTH=false
```

在入口代理中，将实际域名的全部路径转发到 `http://127.0.0.1:8091`，保留 Host、Origin、Cookie，允许图片上传。代理在容器中时，`127.0.0.1` 指代理容器本身，需让代理加入相同网络并转发到 `web:80`，或使用代理可达的宿主机地址。

### 无镜像仓库：文件交付

此分支替代 push/pull。构建机完成两个镜像的构建后执行：

```bash
docker pull postgres:17-bookworm
docker save -o /tmp/linjian-images.tar "$LINJIAN_API_IMAGE" "$LINJIAN_WEB_IMAGE" postgres:17-bookworm
(cd /tmp && sha256sum linjian-images.tar > linjian-images.tar.sha256)
```

把 tar 和校验文件传到服务器 `/tmp/`，同时交付 `compose.yaml` 和 `env.example`。服务器 `.env` 的镜像名与归档标签保持一致。服务器执行：

```bash
(cd /tmp && sha256sum -c linjian-images.tar.sha256)
docker load -i /tmp/linjian-images.tar
docker compose --env-file .env config --quiet
docker compose --env-file .env up -d --no-build --pull never --wait --wait-timeout 180
```

本分支不执行 pull。镜像归档不包含数据库数据和运行凭据，小程序等平台功能仍需联网。

## 3. 环境配置放在哪里

| 配置 | 填写位置 | 说明 |
| --- | --- | --- |
| 镜像标签、宿主机端口 | 部署 `.env` 的 `LINJIAN_*_IMAGE`、端口和 bind host | Compose 使用，不写进镜像。 |
| Web 监听入口与 API 转发 | `.env` 的 `LINJIAN_WEB_LISTEN`、`LINJIAN_API_LISTEN`、`LINJIAN_API_UPSTREAM` | 注入 Web 容器的 Caddy；前两项是对外入口，第三项是连接后端的地址。 |
| 浏览器域名和登录模式 | `.env` 的 `LINJIAN_ADMIN_WEB_ORIGIN`、`LINJIAN_BROWSER_ORIGINS`、`LINJIAN_API_PUBLIC_ORIGIN`、`LINJIAN_INSECURE_LOCAL_AUTH` | 注入 API；Origin 必须匹配实际访问协议、主机、端口。 |
| 数据库连接 | `.env` 的 `LINJIAN_DATABASE_HOST`、`PORT`、`NAME`、`USER`、`PASSWORD`、`SSL_MODE`、`MAX_CONNECTIONS`（均带 `LINJIAN_DATABASE_` 前缀） | 注入 API；库名、角色和密码同时用于 PostgreSQL 首次初始化。 |
| 微信小程序身份 | `.env` 的 `LINJIAN_WX_APP_ID`、`LINJIAN_WX_SECRET` | 注入 API，小程序 AppID 另在其 manifest 设置；Secret 只在后端。 |
| WeKeeper 来源接入 | `.env` 的 `LINJIAN_WEKEEPER_ORIGIN`、`LINJIAN_WEKEEPER_KEY_ID`、`LINJIAN_WEKEEPER_SECRET` | 三项同时填写或全部留空；仅注入 API，配置与验证步骤见下文。 |
| 可选平台接入 | `.env` 中对应的后端凭据 | 暂不用时全部留空，后续配置后重建 API 容器。 |

Compose 只传入 `compose.yaml` 明确声明的变量。数据库连接分项已显式传入；完整 URL 仍需在编排中显式注入。本机 shell 中已 export 的同名变量优先于 `.env`；制作镜像后若在同一终端部署本机版本，先 `unset LINJIAN_API_IMAGE LINJIAN_WEB_IMAGE`，避免沿用发布标签。

配置校验使用 `config --quiet`；直接运行 `config` 会输出插值后的密码。修改 `.env` 后执行 `up -d` 重新创建容器，`restart` 不会加载新配置。

### Web 连接后端：LINJIAN_API_UPSTREAM

Web 镜像内的 Caddy 通过 `LINJIAN_API_UPSTREAM` 连接 Rust API，默认 `api:8080`。Web 入口下的 `/api`、`/healthz`、`/readyz` 和独立 API 入口共用此上游。该变量在 Web 容器运行时注入，不是前端构建参数，无需设置 `VITE_*`。

| 变量 | 注入容器 | 用途 |
| --- | --- | --- |
| `LINJIAN_WEB_LISTEN` | Web | Caddy 的网页入口，例如 `forum.example.com` 或本机 `:80`。 |
| `LINJIAN_API_LISTEN` | Web | Caddy 的独立 API 入口，例如 `api.example.com`；不决定后端连接地址。 |
| `LINJIAN_API_UPSTREAM` | Web | Caddy 实际连接的后端，例如 `api:8080`、`linjian-api:8080`；不带 `/api` 路径。 |
| `LINJIAN_API_PUBLIC_ORIGIN` | API | 客户端访问 API 的公开 HTTPS origin，例如 `https://api.example.com`。 |
| `LINJIAN_BIND_ADDR` | API | Rust 服务自身监听地址，容器中通常为 `0.0.0.0:8080`。 |

仓库 Compose 使用默认 `api:8080` 即可；独立 Docker / 1Panel 中，若后端容器名是 `linjian-api`，可将下面内容保存为 Web 专用 `.env.web`，通过 `docker run --env-file .env.web` 或平台的 Web 容器环境变量配置传入：

```dotenv
LINJIAN_WEB_LISTEN=forum.example.com
LINJIAN_API_LISTEN=api.example.com
LINJIAN_API_UPSTREAM=linjian-api:8080
```

Web 与 API 容器必须加入同一个自定义 Docker 网络，使 Web 能解析并访问 `linjian-api`。这里填写容器内部 API 端口；API 在宿主机映射为其他端口时，同网络容器通信仍使用 `8080`。如果 API 位于其他服务器，填写 Web 容器可达的实际地址。Web 容器中的 `127.0.0.1` 指 Web 自身。

此变量需要包含新 Caddy 配置的 Web 镜像；旧镜像仍将上游写死为 `api:8080`。首次接入此配置须重新构建并部署 Web 镜像，后续仅更改变量时重新创建 Web 容器即可。Compose 用户执行 `docker compose --env-file .env up -d --no-build --no-deps web`；独立 Docker / 1Panel 用户沿用原端口、网络及卷挂载配置重新创建 Web 容器。API 容器无需因上游地址变更而重新构建。

### 可选浏览器入口与 Origin 校验

`LINJIAN_BROWSER_ORIGINS` 可省略或留空，API 自动使用已配置的 `LINJIAN_ADMIN_WEB_ORIGIN`、`LINJIAN_OFFICIAL_ACCOUNT_WEB_ORIGIN`、`LINJIAN_COMMUNITY_WEB_ORIGIN` 作为浏览器来源集合。显式填写 JSON 数组时，仍必须与这些入口的集合一致；`LINJIAN_API_PUBLIC_ORIGIN` 是公开 API 地址，不属于浏览器入口，不要仅因配置了 API 地址就将它加入列表。

独立 Docker / 1Panel 仅运行 API、不使用后台或 H5 时，删除或留空三个浏览器入口变量，并删除或留空 `LINJIAN_BROWSER_ORIGINS`，也可显式填写 `[]`。此时不允许后台浏览器登录；不会从请求的 Host 或 Origin 自动信任任何来源。使用仓库完整 Compose 栈时仍需配置管理入口。

配置后台时，填写实际 `LINJIAN_ADMIN_WEB_ORIGIN` 即可，`LINJIAN_BROWSER_ORIGINS` 无需重复填写。不使用的 H5 入口不要保留模板地址。Origin 必须不带路径或末尾斜杠；默认要求 HTTPS，只有 `LINJIAN_INSECURE_LOCAL_AUTH=true` 时允许本机回环地址使用 HTTP。

### WeKeeper 来源接入配置

需要向 WeKeeper 推送来源消息时，在部署 `.env` 填写以下三项，替换示例地址和占位文本：

```dotenv
LINJIAN_WEKEEPER_ORIGIN=https://keeper.example.com
LINJIAN_WEKEEPER_KEY_ID=替换为WeKeeper签发的key_id
LINJIAN_WEKEEPER_SECRET=替换为同一条调用方凭据的secret
```

| 变量 | 填写说明 |
| --- | --- |
| `LINJIAN_WEKEEPER_ORIGIN` | WeKeeper 的规范 HTTPS origin，允许显式端口；不带接口路径、末尾斜杠、用户信息、query 或 fragment。此处填写 WeKeeper 地址，与 Linjian 的管理入口和公开 API 地址独立。 |
| `LINJIAN_WEKEEPER_KEY_ID` | 从 WeKeeper“调用方凭据”复制的 `key_id`；1–128 个字符，只含英文字母、数字、`-` 和 `_`。 |
| `LINJIAN_WEKEEPER_SECRET` | 从同一条调用方凭据复制的完整原始 secret；无填充 Base64URL，解码后至少 32 字节。不要自行生成替代值，也不要填写数据库密码或 WeKeeper 的存储主密钥。 |

三项必须同时填写，或全部留空表示暂不接入；部分填写或格式错误会导致 API 启动失败。`compose.yaml` 已将它们注入 API，无需修改 Rust 代码或编排。secret 仅由后端读取用于 HMAC 签名，不入库、不回显、不传入 Web 或小程序构建，不需要 `LINJIAN_SOURCE_MASTER_KEY_V1`。环境文件不提交版本库，所有 API 实例使用一致配置。

首次部署随第二节的完整栈启动命令生效。已有部署修改 `.env` 后，确认当前终端没有旧的同名 export 覆盖文件值，再执行以下命令重新创建 API 容器：

```bash
chmod 600 .env
docker compose --env-file .env config --quiet
docker compose --env-file .env up -d --no-build --no-deps --force-recreate --wait --wait-timeout 180 api
```

独立 Docker / 1Panel 部署将相同三项填写到 API 容器环境变量并重新创建容器。登录后台“WeKeeper 连接”点击“验证连接”，成功后为学校配置 WeKeeper 分配的 `route_id`；后台不提供 secret 录入。API 就绪不代表上游凭据已验证，WeKeeper 临时不可用不影响论坛正常写入。格式错误、认证失败及路由配置详见 [WeKeeper 连接部署配置](wekeeper-configuration.md)。

### 数据库连接参数

使用仓库原始 `compose.yaml` 时，数据库连接已配置为下表中的 Compose 值，只需填写 `.env` 的 `LINJIAN_DATABASE_PASSWORD`。PostgreSQL 不发布宿主机端口，API 通过 Compose 内部网络连接数据库。

| API 环境变量 | 程序默认值 | 仓库 Compose 中的值 / 用途 |
| --- | --- | --- |
| `LINJIAN_DATABASE_HOST` | `127.0.0.1` | 默认 `postgres`，即数据库服务名，可在 `.env` 修改。 |
| `LINJIAN_DATABASE_PORT` | `5432` | 默认 `5432`，可在 `.env` 修改；填写数据库服务端口。 |
| `LINJIAN_DATABASE_NAME` | `linjian` | 默认 `linjian`，可在 `.env` 修改。 |
| `LINJIAN_DATABASE_USER` | `linjian` | 默认 `linjian`，可在 `.env` 修改。 |
| `LINJIAN_DATABASE_PASSWORD` | 无，分项连接时必填 | 从 `.env` 注入，与 PostgreSQL 初始化密码一致；无需 URL 编码。 |
| `LINJIAN_DATABASE_SSL_MODE` | `prefer` | 默认 `prefer`，可在 `.env` 修改。 |
| `LINJIAN_DATABASE_MAX_CONNECTIONS` | `10` | 默认 `10`，可在 `.env` 修改；连接池上限允许 1–100。 |
| `LINJIAN_DATABASE_URL` | 不设置 | 未注入；设置后优先于 host、port、name、user、password、SSL 分项，连接池上限仍单独生效。 |

数据库账号用于 API 连接 PostgreSQL，与 `/admin` 登录账号独立。Compose 将 `LINJIAN_DATABASE_NAME`、`LINJIAN_DATABASE_USER`、`LINJIAN_DATABASE_PASSWORD` 分别传给 PostgreSQL 的 `POSTGRES_DB`、`POSTGRES_USER`、`POSTGRES_PASSWORD`。这些初始化变量只对空数据卷生效。

### 使用外部 PostgreSQL

先由数据库管理员准备 Linjian 专用数据库、角色和独立密码，不连接 WeKeeper 的业务库。首次初始化使用空库，角色需要连接及在目标 schema 创建表、索引、函数等迁移对象的权限。API 启动只执行本项目 `backend/migrations/`，不要导入其他项目的迁移。

在部署 `.env` 中补充实际连接信息，例如：

```dotenv
LINJIAN_DATABASE_HOST=db.example.com
LINJIAN_DATABASE_PORT=5432
LINJIAN_DATABASE_NAME=linjian
LINJIAN_DATABASE_USER=linjian
LINJIAN_DATABASE_PASSWORD=替换为外部数据库角色的实际密码
LINJIAN_DATABASE_SSL_MODE=verify-full
LINJIAN_DATABASE_MAX_CONNECTIONS=10
```

以上连接分项已由 `compose.yaml` 注入 API，无需再修改 `api.environment`。同时调整部署机的编排：删除 `api.depends_on.postgres`（它是唯一依赖时删除整个 `api.depends_on`），再删除 `services.postgres`，避免启动本地数据库。已有 `postgres-data` 卷保留，不执行删卷操作；从已有库切换时须先停写、备份并将数据迁移到目标库。外部数据库的备份由其运维流程执行，不能再使用第四节的 `exec postgres pg_dump` 命令。

远程生产数据库示例使用 `verify-full`，数据库证书主机名须匹配连接地址，API 容器须信任签发 CA；私有 CA 按数据库提供方要求只读挂载到容器，并使用下方 URL 的 `sslrootcert` 参数指定路径。同机可信容器网络可按部署条件使用 `disable`。容器中的 `127.0.0.1` 是 API 容器自身；宿主机数据库应填写容器可达的宿主机地址，并配置 PostgreSQL 监听、访问规则及防火墙。

如数据库提供方给出完整 URL，可改为在 `api.environment` 显式注入 `LINJIAN_DATABASE_URL`，替代已有的连接分项；保留连接池上限配置：

```yaml
    environment:
      LINJIAN_DATABASE_URL: ${LINJIAN_DATABASE_URL:?Set the database URL}
      LINJIAN_DATABASE_MAX_CONNECTIONS: ${LINJIAN_DATABASE_MAX_CONNECTIONS:-10}
```

对应 `.env` 示例（密码中的特殊字符须做 URL 百分号编码）：

```dotenv
LINJIAN_DATABASE_URL=postgresql://linjian:replace-with-url-encoded-password@db.example.com:5432/linjian?sslmode=verify-full&sslrootcert=/run/certs/linjian-db-ca.pem
LINJIAN_DATABASE_MAX_CONNECTIONS=10
```

`sslrootcert` 路径必须对应已挂载的 CA 文件；证书由系统受信任 CA 签发时可省略该参数。不使用 URL 时删除该变量，不要设置为空值。改完后按第二节执行 `config --quiet` 和 `up -d --no-build --wait --wait-timeout 180`，通过 API 日志及 `/readyz` 确认连接与迁移成功。

### 独立 Docker / 1Panel 的数据库配置

独立创建容器时，将上述 `LINJIAN_DATABASE_*` 分项或 URL 直接填入 **API 容器**的环境变量，并同时配置第二节的 Origin、登录模式及 `/data/media` 持久卷。API 监听地址使用 `LINJIAN_BIND_ADDR=0.0.0.0:8080`。使用 `docker run --env-file .env.api` 时，文件填写实际值，不使用 `${变量}` 引用。

若自行创建 PostgreSQL 容器，其环境变量使用官方镜像的名称：

```dotenv
POSTGRES_DB=linjian
POSTGRES_USER=linjian
POSTGRES_PASSWORD=替换为与API连接配置一致的数据库密码
```

将数据卷挂载到 PostgreSQL 17 的 `/var/lib/postgresql/data`，数据库与 API 加入同一自定义 Docker 网络；API 的 `LINJIAN_DATABASE_HOST` 填数据库容器名，端口填 `5432`。使用外部数据库则无需创建 PostgreSQL 容器。凭据只注入后端和数据库，不写进 Web 镜像或前端变量；环境文件限制为 `600` 权限，修改后重新创建对应容器。

## 4. 停止、备份与更新

停止和恢复（保留数据）：

```bash
docker compose --env-file .env stop
docker compose --env-file .env start
```

更新前备份数据库，以下命令生成当前目录的备份文件，文件名换成自己的备份日期，不覆盖已有备份：

```bash
(umask 077; docker compose --env-file .env exec -T postgres sh -c 'pg_dump -U "$POSTGRES_USER" -d "$POSTGRES_DB" -Fc' > linjian-20261115.dump)
```

将备份文件和受控保存的 `.env` 放到可靠存储。数据库保存媒体元数据、引用和会话；图片二进制位于 `media-data` 命名卷。上面的数据库备份命令不包含图片，必须在停止 API 写入及后台清理期间同时备份数据库和文件卷，并成套恢复，保留文件卷中的 `.linjian-store` 标识。Caddy 证书状态另保存在命名卷中。

源码部署更新：

```bash
docker compose --env-file .env build --pull
docker compose --env-file .env up -d --no-build --wait --wait-timeout 180
```

镜像部署更新：修改 `.env` 的两个镜像版本，再执行第二节的 `pull` 和 `up`；离线交付则先 load，再按离线命令启动。

API 启动自动执行 `backend/migrations/` 中的迁移。回退镜像前确认旧应用兼容新结构，不执行反向迁移，不使用 `down --volumes` 更新或重置生产数据。仅修改 `.env` 中数据库密码不会更改已有数据库卷内的实际密码。

## 5. 常见问题

| 现象 | 下一步 |
| --- | --- |
| 构建失败 | 查看失败步骤；确认从仓库根目录构建，且基础镜像和依赖源可访问。 |
| 等待启动超时 | 执行 `docker compose --env-file .env logs --tail=100 postgres api web`，先解决数据库或 API 错误。 |
| 无法登录 | 首先创建后台管理员；再核对实际访问地址与 public origin，本机 HTTP 才设 insecure 为 true。 |
| 密码改后数据库连接失败 | 已有卷不会被新环境变量重置密码，恢复正确密码或由数据库管理员修改角色密码。 |
| 启动时 `Error: Unavailable` | 旧镜像会隐藏媒体存储初始化的底层错误；更新镜像后错误会包含媒体路径、失败操作及原因。检查 `/data/media` 挂载目录是否允许 UID/GID `10001:10001` 读写，挂载是否只读，以及 `.linjian-store` 是否可读且有效；不要删除存储标识。 |
| `exec format error` | 镜像与服务器架构不一致，在目标架构构建机重新制作。 |
| 打开文档不是此手册 | 当前 Web 镜像提供既有静态帮助页；本文档站用 `make docs` 预览，尚未打包进 Web 镜像。 |

构建前的研发检查按需执行 `make check-web`、`devbox run -- make check-backend`；它们不作为业务启动门禁。本文命令已对照仓库配置核对，实际镜像构建和 HTTPS 证书申请仍需在有 Docker daemon 的部署环境验证。

独立 Docker / 1Panel 部署中，API 即使在启动阶段退出，标准错误仍可通过已退出容器的日志读取，无需使用 `docker exec`：

```bash
docker ps -a
docker logs --tail=100 你的API容器名
```

若启动时使用 `--rm`，容器退出后会被删除；排查时去掉 `--rm`，保留原环境变量和卷挂载配置重现错误。镜像内 `/data/media` 已设置应用用户权限，但宿主机目录挂载会覆盖该目录的镜像权限，需要单独配置宿主机目录的权限。
