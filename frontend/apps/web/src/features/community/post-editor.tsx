import { useState } from 'react'
import { Button, Alert, Form, Input, Select, Upload } from 'antd'
import { Field } from '../../shared/field'
import { workspace, normalizePostContent, postFieldError, type Post } from './api'
import type { CommunityAction } from './actions'

export function PostEditor({
  school: s,
  editing,
  images,
  setImages,
  busy,
  run,
  publish,
  error,
}: {
  school: string
  editing?: Post
  images: string[]
  setImages: (images: string[]) => void
  busy: boolean
  run: CommunityAction
  publish: (values: { title: string; content: string; tags?: string[] }) => Promise<void>
  error?: string
}) {
  const [failed, setFailed] = useState<File[]>([])
  const [form] = Form.useForm()
  const [titleExpanded, setTitleExpanded] = useState(!!editing?.title)
  return (
    <Form
      layout="vertical"
      className="business-form"
      form={form}
      onFinish={(values) => publish({ ...values, ...normalizePostContent(values) })}
      disabled={busy}
      key={editing?.id ?? 'new'}
      initialValues={{
        title: editing?.title ?? '',
        content: editing?.content,
        tags: editing?.tags ?? [],
      }}
    >
      <h2>{editing ? '编辑内容' : '发布内容'}</h2>
      <Form.Item name="title" label="标题（选填）" hidden={!titleExpanded} rules={[{ validator: (_, value = '') => { const error = postFieldError('title', value); return error ? Promise.reject(new Error(error)) : Promise.resolve() } }]}>
        <Input name="title" />
      </Form.Item>
      <Form.Item name="content" label="正文" rules={[{ validator: (_, value = '') => { const error = postFieldError('content', value); return error ? Promise.reject(new Error(error)) : Promise.resolve() } }]}>
        <Input.TextArea name="content" rows={8} placeholder="分享此刻的想法，或说说你想问的事……" />
      </Form.Item>
      <Button htmlType="button" onClick={() => setTitleExpanded(!titleExpanded)}>{titleExpanded ? '收起标题' : '添加标题（选填）'}</Button>
      <Button htmlType="button" onClick={() => { form.setFieldValue('title', ''); setTitleExpanded(false) }}>移除标题</Button>
      <Field name="tags" label={<>标签</>}>
        <Select
          mode="tags"
          maxCount={10}
          tokenSeparators={[',', '，']}
          placeholder="输入后回车，或用逗号分隔"
        />
      </Field>
      <div>
        <p>图片（最多9张）</p>
        <Upload
          accept="image/png,image/jpeg,image/webp,image/gif"
          multiple
          disabled={busy}
          showUploadList={false}
          beforeUpload={(_, files) => {
            if (_ === files[0])
              void run(async () => {
                if (images.length + files.length > 9) throw new Error('最多9张图片')
                const uploaded = [...images]
                for (const file of files) {
                  try { uploaded.push((await workspace.upload(s, file)).url); setImages([...uploaded]) }
                  catch { setFailed(v => [...v, file]) }
                }
              })
            return false
          }}
        >
          <Button disabled={busy} loading={busy}>
            选择图片
          </Button>
        </Upload>
      </div>
      <p>PNG/JPEG/GIF/WebP，单张最大 5 MiB。上传后提交生效。</p>
      {failed.map((file, i) => <div key={i}>{file.name} 上传失败<Button disabled={busy} onClick={() => void run(async () => { if (images.length >= 9) throw new Error('最多9张图片'); const v = await workspace.upload(s, file); setImages([...images, v.url]); setFailed(items => items.filter((_, n) => n !== i)) })}>重试此图片</Button><Button disabled={busy} onClick={() => setFailed(items => items.filter((_, n) => n !== i))}>移除失败项</Button></div>)}
      {images.map((src, index) => (
        <div key={src}>
          <img src={src} alt="上传预览" width={100} />
          {index > 0 && <Button disabled={busy} onClick={() => { const next = [...images]; [next[index - 1], next[index]] = [next[index]!, next[index - 1]!]; setImages(next) }}>前移</Button>}
          <Button disabled={busy} htmlType="button" onClick={() => setImages(images.filter((v) => v !== src))}>
            移除
          </Button>
        </div>
      ))}
      <div className="form-actions form-actions-sticky">
        {error && <Alert type="error" title={error} />}
        <Button type="primary" htmlType="submit" loading={busy} disabled={busy}>
          提交审核
        </Button>
      </div>
    </Form>
  )
}
