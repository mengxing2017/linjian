import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

export function validateLocationConfig(config) {
  if (typeof config?.permission?.['scope.userLocation']?.desc !== 'string' || !config.permission['scope.userLocation'].desc.trim()) throw new Error('缺少定位用途声明 scope.userLocation.desc')
  if (!Array.isArray(config.requiredPrivateInfos) || config.requiredPrivateInfos.length !== 1 || config.requiredPrivateInfos[0] !== 'getLocation') throw new Error('requiredPrivateInfos 应仅声明本期使用的 getLocation')
  if ('scope.userLocationBackground' in config.permission) throw new Error('学校推荐不允许申请后台定位')
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const file = process.argv[2]
  if (!file) throw new Error('请传入构建后的微信 app.json 路径')
  validateLocationConfig(JSON.parse(readFileSync(file, 'utf8')))
  console.log('微信构建产物定位用途与接口声明检查通过。')
}
