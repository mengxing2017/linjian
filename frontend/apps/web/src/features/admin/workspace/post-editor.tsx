import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Drawer, Form, Image, Input, Select, Space, Upload } from 'antd'
import { workspace, type PostContentInput } from './api'
import { ReviewImage } from './review-image'

function ContentForm({
  school,
  id,
  initial,
  close,
  saved,
  reload,
  blocked,
}: {
  school: string
  id?: string
  initial: PostContentInput
  close: () => void
  saved: () => Promise<void>
  reload?: () => void
  blocked: boolean
}) {
  const [form] = Form.useForm<PostContentInput>()
  const images = Form.useWatch('images', form) ?? initial.images
  const [busy, setBusy] = useState(false),
    [uploading, setUploading] = useState(false),
    [error, setError] = useState(''),
    [conflict, setConflict] = useState(false)
  async function submit(values: PostContentInput) {
    setBusy(true)
    setError('')
    try {
      const input = {
        ...values,
        images,
        tags: values.tags ?? [],
        ...(id ? { revision: initial.revision } : {}),
      }
      if (id)
        await workspace.postManagement.update(school, id, { ...input, revision: initial.revision! })
      else await workspace.postManagement.create(school, input)
      await saved()
      close()
    } catch (e) {
      setError((e as Error).message)
      if (e && typeof e === 'object' && 'status' in e && e.status === 409) setConflict(true)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Form
      disabled={blocked || busy || uploading}
      form={form}
      layout="vertical"
      initialValues={initial}
      onFinish={(values) => void submit(values)}
    >
      {error && <Alert type="error" title={error} />}
      {conflict && (
        <Alert type="warning" title="帖子已被修改，草稿已保留。请核对最新详情后重新加载编辑。" />
      )}
      {conflict && <Button onClick={reload}>重新加载编辑（替换草稿）</Button>}
      <Form.Item name="title" label="标题" rules={[{ required: true, whitespace: true, max: 200 }]}>
        <Input maxLength={200} />
      </Form.Item>
      <Form.Item
        name="content"
        label="正文"
        rules={[{ required: true, whitespace: true, max: 20000 }]}
      >
        <Input.TextArea rows={8} maxLength={20000} />
      </Form.Item>
      <Form.Item
        name="tags"
        label="标签名称"
        rules={[
          {
            validator: (_, v: string[] = []) =>
              v.length <= 10 && v.every((t) => t.trim().length > 0 && Array.from(t).length <= 50)
                ? Promise.resolve()
                : Promise.reject(new Error('最多 10 个标签，每个不超过 50 字')),
          },
        ]}
      >
        <Select mode="tags" tokenSeparators={[',', '，']} />
      </Form.Item>
      <Form.Item name="images" hidden>
        <Input />
      </Form.Item>
      <p>主图为图集第一张，最多 9 张。调整主图会重新审核正文。</p>
      <Image.PreviewGroup>
        <Space wrap>
          {images.map((source, i) => (
            <div key={`${i}:${source}`}>
              <ReviewImage school={school} source={source} index={i} />
              <Button
                disabled={blocked || busy || uploading || i === 0}
                onClick={() =>
                  form.setFieldValue('images', [source, ...images.filter((_, j) => j !== i)])
                }
              >
                {i === 0 ? '主图' : '设为主图'}
              </Button>
              <Button
                danger
                disabled={blocked || busy || uploading}
                onClick={() =>
                  form.setFieldValue(
                    'images',
                    images.filter((_, j) => j !== i),
                  )
                }
              >
                移除图片 {i + 1}
              </Button>
            </div>
          ))}
        </Space>
      </Image.PreviewGroup>
      <Upload
        accept="image/png,image/jpeg,image/gif,image/webp"
        showUploadList={false}
        disabled={blocked || images.length >= 9 || busy || uploading}
        beforeUpload={(file) => {
          setUploading(true)
          setError('')
          void workspace
            .upload(school, file)
            .then((result) => form.setFieldValue('images', [...images, result.url]))
            .catch((e: Error) => setError(e.message))
            .finally(() => setUploading(false))
          return false
        }}
      >
        <Button loading={uploading}>上传图片</Button>
      </Upload>
      <Button
        type="primary"
        htmlType="submit"
        loading={busy}
        disabled={blocked || conflict || uploading}
      >
        保存帖子
      </Button>
    </Form>
  )
}
export function PostEditor({
  school,
  id,
  close,
  saved,
  context,
}: {
  school: string
  id?: string
  close: () => void
  saved: () => Promise<void>
  context?: string
}) {
  const q = useQuery({
    queryKey: ['linjian', 'post-edit', school, id],
    queryFn: ({ signal }) => workspace.postManagement.detail(school, id!, signal),
    enabled: !!id,
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  })
  return (
    <Drawer open title={id ? '编辑帖子' : '添加帖子'} size="large" onClose={close}>
      <p>
        {q.data
          ? `学校：${q.data.school_name}；发布人：${q.data.author_nickname}；正文变更将按当前审核模式重新审核。`
          : context}
      </p>
      {q.error && (
        <>
          <Alert type="error" title={q.error.message} />
          <Button onClick={() => void q.refetch()}>重新加载帖子</Button>
        </>
      )}
      {id && q.isFetching && <p>正在加载帖子…</p>}
      {(!id || q.data) && (
        <ContentForm
          blocked={!!id && (q.isFetching || !!q.error)}
          key={q.data?.revision ?? 'new'}
          school={school}
          id={id}
          initial={
            q.data
              ? {
                  title: q.data.title,
                  content: q.data.content,
                  images: q.data.images,
                  tags: q.data.tags,
                  revision: q.data.revision,
                }
              : { title: '', content: '', images: [], tags: [] }
          }
          close={close}
          saved={saved}
          reload={() => void q.refetch()}
        />
      )}
    </Drawer>
  )
}
