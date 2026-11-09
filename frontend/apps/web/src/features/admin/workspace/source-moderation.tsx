import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Space } from 'antd'
import { workspace, type Row } from './api'
import { useDialogs } from '../../../shared/use-dialogs'
export function SourceModeration({
  school,
  post,
  refresh,
}: {
  school: string
  post: Row
  refresh: () => Promise<unknown>
}) {
  const [busy, setBusy] = useState(false),
    [error, setError] = useState('')
  const dialogs = useDialogs()
  const q = useQuery({
    queryKey: ['linjian', 'source-preview', school, post.id, post.revision],
    queryFn: ({ signal }) => workspace.sourcePreview(school, post.id, signal),
  })
  async function decide(decision: string) {
    if (decision === 'approved' && (!q.data || q.data.post_revision !== post.revision)) return
    const reason = await dialogs.prompt('审核说明')
    if (reason === null) return
    setBusy(true)
    setError('')
    try {
      await workspace.mutate(school, `posts/${post.id}/moderation-decisions`, {
        revision: post.revision,
        decision,
        reason,
        ...(decision === 'approved' ? { source_preview_digest: q.data!.preview_digest } : {}),
      })
      await refresh()
    } catch (e) {
      setError((e as Error).message)
      await q.refetch()
    } finally {
      setBusy(false)
    }
  }
  return (
    <section aria-label="来源摘要预览">
      {dialogs.contextHolder}
      <p>小程序内容须经人工审核。通过后自动准备短链接并推送来源，无需作者再次分享。</p>
      {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
      <p className="text-content">{q.data?.summary ?? '正在生成来源摘要预览…'}</p>
      <p>摘要规则：{q.data?.summary_algorithm ?? '—'}。微信链接将在审核通过后生成。</p>
      <Space>
        {[
          ['approved', '通过审核'],
          ['rejected', '拒绝'],
          ['pending', '撤销审核'],
        ].map(([value, label]) => (
          <Button
            key={value}
            disabled={
              busy ||
              (value === 'approved' &&
                (!q.data || q.data.post_revision !== post.revision || q.isFetching))
            }
            danger={value === 'rejected'}
            onClick={() => void decide(value)}
          >
            {label}
          </Button>
        ))}
      </Space>
    </section>
  )
}
