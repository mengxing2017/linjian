import { useEffect, useRef, useState } from 'react'
import { Alert, App, Button, Card, Input, Space } from 'antd'
import { registrationSettings, type RegistrationSettings } from './api'
import { notifySuccess } from '../../../shared/notify'
import { formatDate } from '../../../shared/format-date'
import { useUnsavedChanges } from '../../../shared/unsaved-changes'

export function MiniappRegistrationSettings() {
  const { modal } = App.useApp()
  const [settings, setSettings] = useState<RegistrationSettings>()
  const [prefix, setPrefix] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [conflict, setConflict] = useState(false)
  const generation = useRef(0)
  const mounted = useRef(false)
  const normalized = prefix.trim()
  const length = Array.from(normalized).length
  const valid = length > 0 && length <= 91 && !Array.from(normalized).some((character) => {
    const code = character.charCodeAt(0)
    return code < 32 || (code >= 127 && code <= 159)
  })
  const dirty = !!settings && prefix !== settings.nickname_prefix
  const setGlobalDirty = useUnsavedChanges()
  useEffect(() => {
    setGlobalDirty(dirty)
    return () => setGlobalDirty(false)
  }, [dirty, setGlobalDirty])

  async function load() {
    if (!mounted.current) return
    const turn = ++generation.current
    setBusy(true)
    setError('')
    try {
      const value = await registrationSettings.read()
      if (turn !== generation.current) return
      setSettings(value)
      setPrefix(value.nickname_prefix)
      setConflict(false)
    } catch (failure) {
      if (turn === generation.current) setError(failure instanceof Error ? failure.message : '读取失败')
    } finally {
      if (turn === generation.current) setBusy(false)
    }
  }

  useEffect(() => {
    mounted.current = true
    void load()
    return () => { mounted.current = false; generation.current++ }
  }, [])

  async function save() {
    if (!settings || busy || !valid || conflict) return
    const turn = ++generation.current
    setBusy(true)
    setError('')
    try {
      const value = await registrationSettings.update({ nickname_prefix: normalized, revision: settings.revision })
      if (turn !== generation.current) return
      setSettings(value)
      setPrefix(value.nickname_prefix)
      notifySuccess('注册昵称前缀已保存，仅影响后续新注册用户')
    } catch (failure) {
      if (turn !== generation.current) return
      setConflict(!!failure && typeof failure === 'object' && 'status' in failure && failure.status === 409)
      setError(failure instanceof Error ? failure.message : '保存失败')
    } finally {
      if (turn === generation.current) setBusy(false)
    }
  }

  function reload() {
    if (dirty) modal.confirm({ title: '丢弃当前输入并重新加载？', okText: '丢弃并重新加载', cancelText: '取消', onOk: load })
    else void load()
  }

  return <Card title="新用户注册昵称" loading={busy && !settings}>
    <Space orientation="vertical" style={{ width: '100%' }}>
      <Alert type="info" title="新用户无需填写昵称，自动使用前缀加随机九位数字。用户之后可自行修改，手动昵称允许重名。" />
      {error && <Alert type="error" title={conflict ? '配置已变化，当前输入已保留，请重新加载后编辑' : error} />}
      <label htmlFor="registration-prefix">新用户昵称前缀</label>
      <Input id="registration-prefix" value={prefix} disabled={busy || !settings} onChange={(event) => setPrefix(event.target.value)} />
      <span>{length}/91 个字符；示例：{normalized}583194726</span>
      {!valid && settings && <Alert type="warning" title="请输入 1～91 个字符，不能包含换行或控制字符" />}
      {settings && <span>修订号：{settings.revision}；修改时间：{formatDate(settings.updated_at)}；操作人：{settings.updated_by_name || '系统初始化'}</span>}
      <Space>
        <Button type="primary" loading={busy} disabled={!settings || !valid || !dirty || conflict} onClick={() => void save()}>保存前缀</Button>
        <Button disabled={busy} onClick={reload}>重新加载</Button>
      </Space>
    </Space>
  </Card>
}
