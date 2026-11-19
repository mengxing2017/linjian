# 中国行政区划数据

## 来源

- 主数据：`china-division@2.7.0` 的 `dist/pca-code.json`
- 港澳台补充：同版本 `dist/HK-MO-TW.json`
- 上游项目：`modood/Administrative-divisions-of-China`

`data/source/` 保留下载时的原始文件，`data/regions.json` 由
`frontend/tools/build-regions.mjs` 生成，禁止手工编辑。

## 生成

```bash
node frontend/tools/build-regions.mjs
```

脚本会：

1. 读取 `pca-code.json` 与 `HK-MO-TW.json`。
2. 将直辖市的「市辖区」层规范为直辖市名称。
3. 为港澳台生成内部使用的合成编码。
4. 校验编码唯一性、父子前缀与非空名称。
5. 输出 `data/regions.json` 并打印体积。

## 版本与更新

- 数据版本：`china-division@2.7.0`
- 更新时替换 `data/source/` 原始文件，重新运行生成脚本与
  `frontend/tools/regions.test.mjs`，不要直接修改生成文件。

## 编码说明

- 大陆省、市、区编码沿用上游数据。
- 港澳台原始数据没有编码，生成脚本使用以下合成编码：
  - 香港特别行政区：`810000`
  - 澳门特别行政区：`820000`
  - 台湾省：`710000`
- 合成编码只用于前端级联选择与去重，不写入后端。
