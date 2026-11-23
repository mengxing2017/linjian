import { Alert, Input, Space, Typography } from 'antd'
import { advertisementContentError, normalizeAdvertisement } from './api'

export function AdvertisementEditor({ content, disabled, onChange }: { content: string; disabled: boolean; onChange: (content: string) => void }) {
  const normalized = normalizeAdvertisement(content)
  const error = advertisementContentError(content)
  return <Space orientation="vertical" style={{ width: '100%' }}>
    <Input.TextArea aria-label="广告文案" autoSize={{ minRows: 2, maxRows: 5 }} value={content} disabled={disabled} onChange={event => onChange(event.target.value)} />
    <Typography.Text type={error ? 'danger' : 'secondary'}>{Array.from(normalized).length} / 50 个 Unicode 字符；仅支持单段文字</Typography.Text>
    {content && error && <Alert type="warning" title={error} />}
    <Typography.Text type="secondary">原有消息后换行追加全文，总长超过 200 字符时本条省略广告，不截断原消息或链接。</Typography.Text>
    <pre aria-label="消息尾部示意" style={{ whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' }}>{`原有完整消息（示例）${normalized ? `\n${normalized}` : ''}`}</pre>
  </Space>
}
