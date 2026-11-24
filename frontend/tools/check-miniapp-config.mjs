import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { validateLocationConfig } from './check-miniapp-location.mjs'

// project.config.json 允许提交，因此不能包含环境相关的 appid。
// 个人 appid 放在 project.private.config.json（已被 .gitignore 忽略）。
const file = fileURLToPath(new URL('../apps/miniapp/project.config.json', import.meta.url))
let config
try {
  config = JSON.parse(readFileSync(file, 'utf8'))
} catch (error) {
  if (error && typeof error === 'object' && error.code === 'ENOENT') {
    console.error(
      '缺少 apps/miniapp/project.config.json：这是微信开发者工具的导入入口，请提交该文件（不要写入 appid）。',
    )
  } else {
    console.error(`无法读取 ${file}: ${error instanceof Error ? error.message : error}`)
  }
  process.exit(1)
}

if (typeof config.appid === 'string' && config.appid && config.appid !== 'touristappid') {
  console.error(
    'apps/miniapp/project.config.json 不得包含真实 appid；请把 appid 放到已被 .gitignore 忽略的 project.private.config.json 中。',
  )
  process.exit(1)
}
const manifest = JSON.parse(readFileSync(fileURLToPath(new URL('../apps/miniapp/src/manifest.json', import.meta.url)), 'utf8'))
validateLocationConfig(manifest['mp-weixin'])
