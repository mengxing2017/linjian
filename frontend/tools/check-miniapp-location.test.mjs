import { test } from 'node:test'
import assert from 'node:assert/strict'
import { validateLocationConfig } from './check-miniapp-location.mjs'

const valid = { permission: { 'scope.userLocation': { desc: '仅用于本次学校推荐，可手动搜索' } }, requiredPrivateInfos: ['getLocation'] }
test('checks location declarations in both source and compiled configurations', () => {
  assert.doesNotThrow(() => validateLocationConfig(valid))
  for (const config of [{}, { ...valid, permission: {} }, { ...valid, requiredPrivateInfos: [] }, { ...valid, requiredPrivateInfos: ['getLocation', 'startLocationUpdate'] }, { ...valid, permission: { ...valid.permission, 'scope.userLocationBackground': {} } }]) assert.throws(() => validateLocationConfig(config))
})
