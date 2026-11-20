# 仿真作者头像

资源迁自原 wxrobot 项目的 `public/assets/img/default-avatars/`，保留全部 30 张校园人物头像及原文件名。

- 槽位为文件名顺序 0～29，槽位 0 对应 `student-avatar-01-default.png`。
- 原图为 512×512 RGB PNG；使用 Lanczos3 缩放为 128×128，再进行 PNG 最优压缩，不保留原图元数据。
- 原文件合计 10,466,397 字节；压缩后合计 858,402 字节，减少约 91.8%。
- 资源版本为 `linjian-student-avatars-v1`，构建时嵌入后端。运行和部署不依赖原项目，不在小程序中打包完整头像库。
- 预览与正式作者头像使用同一份压缩字节；正式头像仍遵循受控资产及用户 owner 约束。

在仓库根目录重新压缩迁入资源：

```sh
devbox run -- cargo run --locked --manifest-path backend/Cargo.toml \
  --example compress_simulated_avatars -- \
  ../wxrobot/public/assets/img/default-avatars backend/assets/simulated-avatars
```

工具只读取源目录，输出必须与源目录独立，不修改原项目。更换资源时同步更新资源版本和契约 fixture。
