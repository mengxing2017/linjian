import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Select, Space, Upload } from 'antd'
import { product } from './api'
export function MediaPicker({ school, value, onChange, label }: { school?: string; value: string | null; onChange: (id: string | null) => void; label: string }) {
  const q = useQuery({ queryKey: ['linjian', 'operations-media', school], queryFn: () => product.resources.artwork(school) })
  const [busy, setBusy] = useState(false), [error, setError] = useState('')
  return <Space orientation="vertical" className="full-width">
    <label>{label}<Select aria-label={label} allowClear value={value ?? undefined} placeholder="选择本范围图片" options={q.data?.map((v, i) => ({ value: v.id, label: `图片 ${i + 1} · ${v.width}×${v.height}` }))} onChange={v => onChange(v ?? null)} /></label>
    <Upload accept="image/png,image/jpeg,image/gif,image/webp" showUploadList={false} disabled={busy} beforeUpload={file => {
      setBusy(true); setError('')
      void (async () => { try { const v = await product.resources.upload(school, await file.arrayBuffer()); onChange(v.image_ref.id); await q.refetch() } catch (e) { setError(e instanceof Error ? e.message : '上传失败') } finally { setBusy(false) } })()
      return false
    }}><Button loading={busy}>上传{label}</Button></Upload>
    {value && <img alt={label} src={`${school ? `/api/v1/schools/${school}` : '/api/v1/platform'}/resource-artwork/${value}`} style={{ maxWidth: 220, maxHeight: 180 }} />}
    {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
  </Space>
}
