import { readFileSync, existsSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const roots = JSON.parse(readFileSync(new URL('../apps/miniapp/src/custom-tab-bar/routes.json', import.meta.url), 'utf8'))
export function validateNavigation(config) {
  if (config.tabBar?.custom !== true) throw new Error('需要原生自定义底栏')
  const expected = roots.map(root => ({ pagePath: root.path.slice(1), text: root.label }))
  if (JSON.stringify(config.tabBar.list.map(({ pagePath, text }) => ({ pagePath, text }))) !== JSON.stringify(expected)) throw new Error('四根 Tab 与统一路由不一致')
  if (expected.some(root => root.pagePath === 'pages/index/publish')) throw new Error('发布必须是独立页面')
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const root = process.argv[2]
  if (!root) throw new Error('请传入微信小程序构建目录')
  validateNavigation(JSON.parse(readFileSync(path.join(root, 'app.json'), 'utf8')))
  for (const file of ['index.js', 'index.json', 'index.wxml', 'index.wxss', 'routes.json']) {
    if (!existsSync(path.join(root, 'custom-tab-bar', file))) throw new Error(`缺少底栏产物 ${file}`)
  }
  const component = JSON.parse(readFileSync(path.join(root, 'custom-tab-bar/index.json'), 'utf8'))
  if (component.component !== true) throw new Error('自定义底栏未注册为原生组件')
  console.log('微信构建产物四根导航与自定义底栏检查通过。')
}
