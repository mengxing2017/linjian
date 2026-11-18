import { useRef, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Select, Space, Upload } from 'antd'
export interface Picture { id: string; url?: string; width?: number; height?: number; size_bytes?: number }
export function ImagePicker({ scope, label, value, onChange, onBusyChange, load, upload, source }: {
  scope: string; label: string; value: string | null; onChange: (id: string | null) => void
  onBusyChange?: (busy: boolean) => void
  load: (page: number) => Promise<{ items: Picture[]; page?: { has_more: boolean } }>
  upload: (file: File) => Promise<Picture>; source: (id: string) => string
}) {
  const [page, setPage] = useState(1), [busy, setBusy] = useState(false), [error, setError] = useState('')
  const [retry, setRetry] = useState<File>(), pending = useRef(false), current = useRef(scope)
  current.current = scope
  const q = useQuery({ queryKey: ['linjian', 'image-picker', scope, page], queryFn: () => load(page) })
  async function send(file: File) {
    if (pending.current) return
    const turn = scope
    pending.current = true; setBusy(true); onBusyChange?.(true); setError('')
    try {
      const picture = await upload(file)
      if (current.current !== turn) return
      onChange(picture.id); setRetry(undefined); await q.refetch()
    } catch (e) { if (current.current === turn) { setRetry(file); setError(e instanceof Error ? e.message : '上传失败') } }
    finally { pending.current = false; setBusy(false); onBusyChange?.(false) }
  }
  return <Space orientation="vertical" className="full-width">
    <Select aria-label={label} allowClear value={value ?? undefined} disabled={busy} placeholder="选择图片或上传本地文件"
      options={q.data?.items.map(v => ({ value: v.id, label: <Space><img src={v.url ?? source(v.id)} alt="素材缩略图" width={36} height={28} /><span>{v.id.slice(0, 8)} · {v.width}×{v.height}{v.size_bytes ? ` · ${Math.ceil(v.size_bytes / 1024)} KiB` : ''}</span></Space> }))}
      onChange={id => onChange(id ?? null)} />
    <Space><Button disabled={page <= 1 || busy} onClick={() => setPage(page - 1)}>上一页</Button><span>第 {page} 页</span><Button disabled={!q.data?.page?.has_more || busy} onClick={() => setPage(page + 1)}>下一页</Button></Space>
    <Upload accept="image/png,image/jpeg,image/gif,image/webp" showUploadList={false} disabled={busy} beforeUpload={file => { void send(file); return false }}><Button loading={busy}>上传{label}</Button></Upload>
    <span>PNG / JPEG / GIF / WebP，最大 5 MiB。上传后保存表单生效。</span>
    {retry && <Button disabled={busy} onClick={() => void send(retry)}>重试上传</Button>}
    {value && <img alt={label} src={source(value)} style={{ maxWidth: 220, maxHeight: 180 }} />}
    {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
  </Space>
}
