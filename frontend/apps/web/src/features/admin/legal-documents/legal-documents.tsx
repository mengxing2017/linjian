import { useEffect, useRef, useState } from 'react'
import { Alert, App, Button, Card, Drawer, Input, Space, Tabs } from 'antd'
import { legalDocuments, legalTitles, validLegalContent, type LegalDocumentKind, type LegalDocument, type LegalRevision } from './api'
import { useUnsavedChanges } from '../../../shared/unsaved-changes'
import { notifySuccess } from '../../../shared/notify'

export function LegalDocuments() {
  const { modal } = App.useApp()
  const [kind, setKind] = useState<LegalDocumentKind>('user_agreement')
  const [dirty, setDirty] = useState(false)
  function change(value: string) {
    const next = value as LegalDocumentKind
    if (dirty) modal.confirm({ title: '放弃未保存的正文？', onOk: () => { setDirty(false); setKind(next) } })
    else setKind(next)
  }
  return <section><Alert type="info" showIcon title="平台统一配置，所有人可公开阅读。保存即生效，未配置的条目在用户端隐藏。" />
    <Tabs activeKey={kind} onChange={change} items={Object.entries(legalTitles).map(([key, label]) => ({ key, label }))} />
    <Editor key={kind} kind={kind} dirtyChanged={setDirty} />
  </section>
}
function Editor({ kind, dirtyChanged }: { kind: LegalDocumentKind; dirtyChanged: (value: boolean) => void }) {
  const { modal } = App.useApp()
  const [document, setDocument] = useState<LegalDocument>()
  const [content, setContent] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [conflict, setConflict] = useState(false)
  const [preview, setPreview] = useState(false)
  const [history, setHistory] = useState<LegalRevision[]>()
  const [page, setPage] = useState(1)
  const generation = useRef(0)
  const dirty = !!document && content !== (document.content ?? '')
  const globalDirty = useUnsavedChanges()
  useEffect(() => { globalDirty(dirty); dirtyChanged(dirty); return () => { globalDirty(false); dirtyChanged(false) } }, [dirty, dirtyChanged, globalDirty])
  async function load() {
    const turn = ++generation.current
    setBusy(true); setError('')
    try {
      const value = await legalDocuments.admin(kind)
      if (turn !== generation.current) return
      setDocument(value); setContent(value.content ?? ''); setConflict(false)
    } catch (failure) { if (turn === generation.current) setError(failure instanceof Error ? failure.message : '读取失败') }
    finally { if (turn === generation.current) setBusy(false) }
  }
  useEffect(() => { void load(); return () => { generation.current++ } }, [kind])
  async function write(clear: boolean) {
    if (!document) return
    const turn = ++generation.current
    setBusy(true); setError('')
    try {
      const value = clear ? await legalDocuments.clear(kind, document.revision) : await legalDocuments.save(kind, document.revision, content)
      if (turn !== generation.current) return
      setDocument(value); setContent(value.content ?? ''); setConflict(false); setHistory(undefined)
      notifySuccess(clear ? '已清空，用户端入口已隐藏' : '已保存并生效')
    } catch (failure) {
      if (turn !== generation.current) return
      setConflict(true)
      setError(`${failure instanceof Error ? failure.message : '保存失败'}。本地正文已保留，请重新读取核对服务端结果后再编辑。`)
    } finally { if (turn === generation.current) setBusy(false) }
  }
  async function readHistory(next: number) {
    const turn = ++generation.current
    setBusy(true); setError('')
    try { const rows = await legalDocuments.history(kind, next); if (turn === generation.current) { setHistory(rows); setPage(next) } }
    catch (failure) { if (turn === generation.current) setError(failure instanceof Error ? failure.message : '历史读取失败') }
    finally { if (turn === generation.current) setBusy(false) }
  }
  return <Card title={legalTitles[kind]}>
    {error && <Alert type="error" showIcon title={error} />}
    <p>{document?.content ? '已配置' : '未配置'} · 版本 {document?.revision ?? '—'} · {document?.updated_at ? new Date(document.updated_at).toLocaleString() : '—'} · {document?.updated_by_name ?? '—'}</p>
    <Input.TextArea aria-label={`${legalTitles[kind]}正文`} rows={18} value={content} disabled={busy || !document} onChange={event => setContent(event.target.value)} placeholder="输入完整纯文本正文，保留换行。不填写时用户端不显示，已有内容请通过清空按钮撤下。" />
    <p>{Array.from(content).length}/50000 字符，支持纯文本，不执行 HTML 或外链。</p>
    <Space wrap>
      <Button type="primary" disabled={!document || !validLegalContent(content) || conflict || busy || !dirty} onClick={() => modal.confirm({ title: '保存后立即对用户展示，确认保存？', onOk: () => write(false) })}>保存生效</Button>
      <Button disabled={!content} onClick={() => setPreview(true)}>预览</Button>
      <Button danger disabled={!document?.content || busy} onClick={() => modal.confirm({ title: '清空后用户端不再显示此入口', content: kind === 'privacy_policy' ? '清空不免除真实运营中的告知义务，请确认运营及合规安排。历史修订会保留。' : '历史修订会保留，确认撤下当前内容？', onOk: () => write(true) })}>清空内容</Button>
      <Button disabled={busy} onClick={() => dirty ? modal.confirm({ title: '重新读取将放弃本地编辑，确认继续？', onOk: load }) : void load()}>重新读取</Button>
      <Button disabled={busy} onClick={() => void readHistory(1)}>修订历史</Button>
    </Space>
    <Drawer title={`${legalTitles[kind]}预览`} open={preview} onClose={() => setPreview(false)}><div style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{content}</div></Drawer>
    <Drawer title="修订历史" open={!!history} onClose={() => setHistory(undefined)}>
      {history?.map(row => <Card key={row.id} title={`版本 ${row.revision} · ${row.updated_by_name}`}><p>{new Date(row.updated_at).toLocaleString()}</p><div style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{row.content ?? '已清空'}</div></Card>)}
      {history?.length === 0 && <p>暂无修订</p>}
      <Space><Button disabled={busy || page === 1} onClick={() => void readHistory(page - 1)}>上一页</Button><span>第 {page} 页</span><Button disabled={busy || history?.length !== 20} onClick={() => void readHistory(page + 1)}>下一页</Button></Space>
    </Drawer>
  </Card>
}
