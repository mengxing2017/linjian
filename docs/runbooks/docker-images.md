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

数据库账号固定为 `linjian`，其密码来自 `.env`；后台管理员是上面手动创建的账号，两者不能混用。Docker 首次启动不会自动创建管理员。同名账号已存在时创建失败，不会重置旧密码。

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
| 浏览器域名和登录模式 | `.env` 的 `LINJIAN_ADMIN_WEB_ORIGIN`、`LINJIAN_BROWSER_ORIGINS`、`LINJIAN_API_PUBLIC_ORIGIN`、`LINJIAN_INSECURE_LOCAL_AUTH` | 注入 API；Origin 必须匹配实际访问协议、主机、端口。 |
| 数据库密码 | `.env` 的 `LINJIAN_DATABASE_PASSWORD` | 同时配置 PostgreSQL 初始化和 API 连接。 |
| 微信小程序身份 | `.env` 的 `LINJIAN_WX_APP_ID`、`LINJIAN_WX_SECRET` | 注入 API，小程序 AppID 另在其 manifest 设置；Secret 只在后端。 |
| 可选平台接入 | `.env` 中对应的后端凭据 | 暂不用时全部留空，后续配置后重建 API 容器。 |

Compose 只传入 `compose.yaml` 明确声明的变量。在 `.env` 新增数据库 URL 不会自动替换固定的 `postgres` 地址。本机 shell 中已 export 的同名变量优先于 `.env`；制作镜像后若在同一终端部署本机版本，先 `unset LINJIAN_API_IMAGE LINJIAN_WEB_IMAGE`，避免沿用发布标签。

配置校验使用 `config --quiet`；直接运行 `config` 会输出插值后的密码。修改 `.env` 后执行 `up -d` 重新创建容器，`restart` 不会加载新配置。

## 4. 停止、备份与更新

停止和恢复（保留数据）：

```bash
docker compose --env-file .env stop
docker compose --env-file .env start
```

更新前备份数据库，以下命令生成当前目录的备份文件，文件名换成自己的备份日期，不覆盖已有备份：

```bash
(umask 077; docker compose --env-file .env exec -T postgres pg_dump -U linjian -d linjian -Fc > linjian-20261115.dump)
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
| `exec format error` | 镜像与服务器架构不一致，在目标架构构建机重新制作。 |
| 打开文档不是此手册 | 当前 Web 镜像提供既有静态帮助页；本文档站用 `make docs` 预览，尚未打包进 Web 镜像。 |

构建前的研发检查按需执行 `make check-web`、`devbox run -- make check-backend`；它们不作为业务启动门禁。本文命令已对照仓库配置核对，实际镜像构建和 HTTPS 证书申请仍需在有 Docker daemon 的部署环境验证。
