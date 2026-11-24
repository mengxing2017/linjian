# 小程序本地图标

图标由用户指定的原项目 `wxrobot/uniapp/static/iconpark/` 选择复用，运行、测试和构建不需要该目录。仅迁入压缩产物，不保存原始副本；未引入 ColorUI、在线字体或旧业务代码。

当前 32 个图标从 60,583 字节无损压缩至 29,627 字节，减少 30,956 字节（约 51.1%）。每个文件均严格变小，96×96 尺寸、透明度及完整 RGBA 像素不变。逐文件数据和 SHA-256 见 `icon-assets.manifest.json`，选用文件见仓库根 `tools/miniapp-icons-selection.json`。

## 再现迁移

在仓库根目录运行，将 `批次` 换成新的目录名：

```sh
devbox run -- cargo run --locked --manifest-path backend/Cargo.toml \
  --example compress_miniapp_icons -- \
  --source ../wxrobot/uniapp/static/iconpark \
  --selection tools/miniapp-icons-selection.json \
  --staging /tmp/linjian-miniapp-icons-批次
```

工具在全部文件通过体积、解码像素和源文件未变检查后才写暂存目录，任何失败都不会写入本项目静态目录。`preview.png` 按选用清单从左至右、从上至下排列；每个图标依次显示白底、深色底、蓝底下的原始 / 压缩成对图片。

检查对照图后，首次迁入使用：

```sh
node frontend/tools/import-miniapp-icons.mjs /tmp/linjian-miniapp-icons-批次 --reviewed
node frontend/tools/check-miniapp-icons.mjs
```

导入工具不覆盖已有目录或清单；更新现有图标时先在临时工作区验证，评审后显式更新对应产物、清单和映射，不绕过校验或保留原图兜底。PNG 使用 Rust `image` 的 Best 压缩，取六种无损过滤器中体积最小的结果；不缩放、不量化、不去除透明度。

`shared/icon-assets.json` 是语义到文件的映射，`shared/icons.ts` 负责解析，`AppIcon.vue` 只负责展示；原生自定义底栏通过桥接层取得相同路径。固定颜色 PNG 不会自动跟随学校主题换色，主题继续控制背景、文字与按钮。

目前收藏只有现成彩色心形，继续用文案区分操作状态；退出和分享没有专用素材，保留文字，不用错误图标凑齐。

## 验证与验收

```sh
node --test frontend/tools/check-miniapp-icons.test.mjs
devbox run -- cargo test --locked --manifest-path backend/Cargo.toml --example compress_miniapp_icons
make check-miniapp
```

小程序 lint、构建前与构建后校验资源摘要、PNG 完整性、逐图体积、映射和产物。检查不接入业务启动链路，也不请求外部目录或服务。根 `make check-web` 包含新增 Node 工具测试和完整前端回归。

本次已检查像素一致性及暂存成对栅格图；微信开发者工具、iOS / Android 真机截图仍需人工验收，不能以压缩或构建成功代替。白色加号和发送图标需放在有色背景上。

资源来源按用户要求记录，原目录未发现专门随附这些 PNG 的授权说明；不据目录名推断作者或许可证，也未新增第三方授权声明。正式对外分发前应由资产所有者核实使用权及需要保留的说明。
