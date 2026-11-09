import { useEffect, useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Image } from 'antd'
import { workspace } from './api'

export function ReviewImage({
  school,
  source,
  index,
  report,
  attachmentLabel = '帖子附件',
}: {
  school: string
  source: string
  index: number
  attachmentLabel?: string
  report?: (index: number, ready: boolean) => void
}) {
  const [url, setUrl] = useState(''),
    [failed, setFailed] = useState(false),
    [attempt, setAttempt] = useState(0)
  const q = useQuery({
    queryKey: ['linjian', 'review-image', school, source, attempt],
    queryFn: ({ signal }) => {
      const prefix = `/api/v1/schools/${school}/media/`
      const id = source.startsWith(prefix) ? source.slice(prefix.length) : ''
      if (!/^[\da-f-]{36}$/i.test(id)) throw new Error('附件地址不属于当前学校')
      return workspace.reviewImage(school, id, signal)
    },
    retry: false,
    refetchOnWindowFocus: false,
  })
  useEffect(() => {
    report?.(index, false)
    setFailed(false)
    if (!q.data) {
      setUrl('')
      return
    }
    const object = URL.createObjectURL(new Blob([q.data]))
    setUrl(object)
    return () => URL.revokeObjectURL(object)
  }, [q.data, index, report])
  return (
    <div>
      {(q.error || failed) && (
        <Alert type="error" title={`附件 ${index + 1} 读取失败，请重试`} />
      )}
      {url && (
        <Image
          src={url}
          alt={`${attachmentLabel} ${index + 1}`}
          width={140}
          onLoad={() => report?.(index, true)}
          onError={() => {
            setFailed(true)
            report?.(index, false)
          }}
        />
      )}
      {(q.error || failed) && (
        <Button
          onClick={() => {
            report?.(index, false)
            setAttempt((value) => value + 1)
          }}
        >
          重试附件 {index + 1}
        </Button>
      )}
    </div>
  )
}

