import { Button, Form, Input, Upload } from 'antd'
import { Field } from '../../shared/field'
import { workspace, type Post } from './api'
import type { CommunityAction } from './actions'

export function PostEditor({
  school: s,
  editing,
  images,
  setImages,
  busy,
  run,
  publish,
}: {
  school: string
  editing?: Post
  images: string[]
  setImages: (images: string[]) => void
  busy: boolean
  run: CommunityAction
  publish: (values: { title: string; content: string; tags?: string }) => Promise<void>
}) {
  return (
    <Form
      layout="vertical"
      className="business-form"
      onFinish={publish}
      disabled={busy}
      key={editing?.id ?? 'new'}
      initialValues={{
        title: editing?.title,
        content: editing?.content,
        tags: editing?.tags.join(','),
      }}
    >
      <h2>{editing ? '编辑内容' : '发布内容'}</h2>
      <Field name="title" required label={<>标题</>}>
        <Input name="title" maxLength={200} />
      </Field>
      <Field name="content" required label={<>正文</>}>
        <Input.TextArea name="content" maxLength={20000} />
      </Field>
      <Field name="tags" label={<>标签（逗号分隔）</>}>
        <Input name="tags" />
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
                  uploaded.push((await workspace.upload(s, file)).url)
                  setImages([...uploaded])
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
      {images.map((src) => (
        <div key={src}>
          <img src={src} alt="上传预览" width={100} />
          <Button htmlType="button" onClick={() => setImages(images.filter((v) => v !== src))}>
            移除
          </Button>
        </div>
      ))}
      <Button type="primary" htmlType="submit" loading={busy} disabled={busy}>
        提交审核
      </Button>
    </Form>
  )
}
