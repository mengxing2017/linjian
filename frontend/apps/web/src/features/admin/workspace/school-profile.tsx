import { useState, useEffect } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Button,
  Descriptions,
  Drawer,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
  Upload,
} from 'antd'
import { workspace, product, type School, type SchoolInput, type SchoolFilters } from './api'
import { PaginationControls } from '../../../shared/pagination-controls'
const defaults = {
  name: '',
  short_name: '',
  province: '',
  city: '',
  area: '',
  latitude: null,
  longitude: null,
  logo_asset_id: null,
  header_bg_asset_id: null,
  theme_primary: '#6A5AF9',
  theme_secondary: '#8F6BFF',
  theme_text_color: '#FFFFFF',
}
export function SchoolImage({
  school,
  id,
  source,
  alt,
}: {
  school: string
  id?: string | null
  source?: string | null
  alt: string
}) {
  const [url, setUrl] = useState('')
  useEffect(() => {
    let live = true,
      object = ''
    setUrl('')
    if (id || source)
      void (
        id
          ? workspace.schoolProfiles.artwork(school, id)
          : workspace.schoolProfiles.branding(school, source!.endsWith('/logo') ? 'logo' : 'header')
      )
        .then((bytes) => {
          if (live) {
            object = URL.createObjectURL(new Blob([bytes]))
            setUrl(object)
          }
        })
        .catch(() => {})
    return () => {
      live = false
      if (object) URL.revokeObjectURL(object)
    }
  }, [school, id, source])
  return url ? (
    <img src={url} alt={alt} style={{ maxWidth: 200, maxHeight: 90 }} />
  ) : (
    <span>{alt}：默认</span>
  )
}
export function SchoolForm({
  school,
  platform = false,
  onSaved,
}: {
  school?: School
  platform?: boolean
  onSaved: (school?: School) => Promise<unknown>
}) {
  const [form] = Form.useForm<SchoolInput>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(''),
    [conflict, setConflict] = useState(false)
  const gallery = useQuery({
    queryKey: ['linjian', 'school-artwork', school?.id],
    queryFn: () => product.resources.artwork(school!.id),
    enabled: !!school,
  })
  const primary = Form.useWatch('theme_primary', form) || defaults.theme_primary,
    secondary = Form.useWatch('theme_secondary', form) || defaults.theme_secondary,
    text = Form.useWatch('theme_text_color', form) || defaults.theme_text_color
  const logo = Form.useWatch('logo_asset_id', form),
    header = Form.useWatch('header_bg_asset_id', form)
  async function save(v: SchoolInput) {
    setBusy(true)
    setError('')
    try {
      const body = { ...v, revision: school?.revision ?? 0 }
      if (!platform) delete body.status
      const result = school
        ? await workspace.schoolProfiles.update(school.id, body)
        : await workspace.schoolProfiles.create(body)
      await onSaved(result)
    } catch (e) {
      setError((e as Error).message)
      if (e && typeof e === 'object' && 'status' in e && e.status === 409) setConflict(true)
    } finally {
      setBusy(false)
    }
  }
  async function reload() {
    if (!school) return
    setBusy(true)
    try {
      const latest = await workspace.schoolProfiles.detail(school.id)
      await onSaved(latest)
      form.setFieldsValue({ ...latest })
      setConflict(false)
      setError('')
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Form
      form={form}
      layout="vertical"
      initialValues={{
        ...defaults,
        ...school,
        status: school?.status ?? 'normal',
        config: school?.config ?? { post_audit_mode: 'manual', comment_audit_mode: 'manual' },
      }}
      onFinish={(v) => {
        if (v.status === 'hidden' && school?.status !== 'hidden') {
          Modal.confirm({
            title: '确认隐藏学校？',
            content: '成员将无法访问学校，已有来源将按原规则撤回。恢复学校不会批量重放历史内容。',
            okText: '确认隐藏',
            cancelText: '取消',
            onOk: () => save(v),
          })
        } else void save(v)
      }}
    >
      {(error || gallery.error) && <Alert type="error" title={error || gallery.error?.message} />}
      {conflict && (
        <>
          <p>资料已被其他操作更新，本地输入已保留。重新加载会舍弃本地输入。</p>
          <Button onClick={() => void reload()}>重新加载学校资料</Button>
        </>
      )}
      <Form.Item
        label="学校名称"
        name="name"
        rules={[{ required: true, whitespace: true }, { max: 100 }]}
      >
        <Input maxLength={100} />
      </Form.Item>
      {(
        [
          ['short_name', '学校简称', 50],
          ['province', '省份', 100],
          ['city', '城市', 100],
          ['area', '区域', 100],
        ] as const
      ).map(([name, label, max]) => (
        <Form.Item key={name} name={name} label={label} rules={[{ max }]}>
          <Input maxLength={max} />
        </Form.Item>
      ))}
      {(
        [
          ['latitude', '纬度', 90],
          ['longitude', '经度', 180],
        ] as const
      ).map(([name, label, max]) => (
        <Form.Item key={name} name={name} label={label}>
          <InputNumber min={-max} max={max} step={0.000001} />
        </Form.Item>
      ))}
      <p>经纬度需同时填写或清空，最多六位小数；0 是有效坐标。</p>
      {platform && (
        <>
          <Form.Item name="status" label="状态">
            <Select
              options={[
                { value: 'normal', label: '正常' },
                { value: 'hidden', label: '隐藏' },
              ]}
            />
          </Form.Item>
          <p>隐藏后成员不可访问学校，已有来源将按原规则撤回；恢复不会批量重放历史内容。</p>
        </>
      )}
      {school ? (
        (
          [
            ['logo_asset_id', '学校图标'],
            ['header_bg_asset_id', '头部背景图'],
          ] as const
        ).map(([key, label]) => (
          <section key={key}>
            <Form.Item name={key} label={label}>
              <Select
                allowClear
                onClear={() => form.setFieldValue(key, null)}
                options={gallery.data?.map((a, i) => ({
                  value: a.id,
                  label: `图片 ${i + 1} · ${a.width}×${a.height}`,
                }))}
              />
            </Form.Item>
            <Upload
              disabled={busy}
              accept="image/png,image/jpeg,image/gif,image/webp"
              showUploadList={false}
              beforeUpload={(file) => {
                setBusy(true)
                setError('')
                void file
                  .arrayBuffer()
                  .then((bytes) => product.resources.upload(school.id, bytes))
                  .then(async (a) => {
                    form.setFieldValue(key, a.id)
                    await gallery.refetch()
                  })
                  .catch((e) => setError((e as Error).message))
                  .finally(() => setBusy(false))
                return false
              }}
            >
              <Button disabled={busy}>上传{label}</Button>
            </Upload>
          </section>
        ))
      ) : (
        <p>先保存学校，再编辑上传图标和背景；未配置时显示默认图片。</p>
      )}
      {(
        [
          ['theme_primary', '主题主色'],
          ['theme_secondary', '主题辅色'],
          ['theme_text_color', '主题文字色'],
        ] as const
      ).map(([key, label]) => (
        <Form.Item
          name={key}
          key={key}
          label={label}
          rules={[
            { required: true },
            { pattern: /^#(?:[\da-fA-F]{3}|[\da-fA-F]{6})$/, message: '请填写 #RGB 或 #RRGGBB' },
          ]}
        >
          <Input />
        </Form.Item>
      ))}
      <div
        style={{
          padding: 20,
          background:
            /^#[\da-fA-F]{3}(?:[\da-fA-F]{3})?$/.test(primary) &&
            /^#[\da-fA-F]{3}(?:[\da-fA-F]{3})?$/.test(secondary)
              ? `linear-gradient(135deg,${primary},${secondary})`
              : defaults.theme_primary,
          color: /^#[\da-fA-F]{3}(?:[\da-fA-F]{3})?$/.test(text) ? text : defaults.theme_text_color,
        }}
      >
        <strong>学校品牌头部预览</strong>
        {school && (
          <>
            <SchoolImage school={school.id} id={logo} alt="学校图标" />
            <SchoolImage school={school.id} id={header} alt="头部背景图" />
          </>
        )}
      </div>
      {school && (
        <>
          <Form.Item
            name={['config', 'post_audit_mode']}
            label="帖子审核模式（小程序帖子始终人工审核）"
          >
            <Select
              options={[
                { value: 'manual', label: '人工审核' },
                { value: 'auto', label: '自动通过' },
              ]}
            />
          </Form.Item>
          <Form.Item name={['config', 'comment_audit_mode']} label="评论审核模式">
            <Select
              options={[
                { value: 'manual', label: '人工审核' },
                { value: 'auto', label: '自动通过' },
              ]}
            />
          </Form.Item>
        </>
      )}
      <Button type="primary" htmlType="submit" loading={busy} disabled={conflict}>
        保存学校资料
      </Button>
    </Form>
  )
}
export function Schools({
  refresh,
  enter,
}: {
  schools: School[]
  loading: boolean
  refresh: () => Promise<void>
  enter: (id: string, view: string) => void
}) {
  const [filters, setFilters] = useState<SchoolFilters>({}),
    [creating, setCreating] = useState(false),
    [detail, setDetail] = useState<School>(),
    [editing, setEditing] = useState<School>()
  const cache = useQueryClient()
  const q = useQuery({
    queryKey: ['linjian', 'school-management', filters],
    queryFn: ({ signal }) => workspace.schoolProfiles.platform(filters, signal),
  })
  async function saved() {
    await cache.invalidateQueries({ queryKey: ['linjian', 'school-management'] })
    await refresh()
    setCreating(false)
    setEditing(undefined)
    setDetail(undefined)
  }
  const change = (v: Partial<SchoolFilters>) => setFilters((f) => ({ ...f, ...v, page: 1 }))
  return (
    <Space orientation="vertical" className="full-width">
      <Button type="primary" onClick={() => setCreating(true)}>
        新建学校
      </Button>
      {q.error && <Alert type="error" title={q.error.message} />}
      <Space wrap>
        <Input
          aria-label="搜索学校"
          placeholder="名称 / 简称 / 区域"
          value={filters.search ?? ''}
          onChange={(e) => change({ search: e.target.value })}
        />
        {(['province', 'city', 'area'] as const).map((k, i) => (
          <Input
            key={k}
            aria-label={['筛选省份', '筛选城市', '筛选区域'][i]}
            placeholder={['省份', '城市', '区域'][i]}
            value={filters[k] ?? ''}
            onChange={(e) => change({ [k]: e.target.value })}
          />
        ))}
        <Select
          aria-label="学校状态筛选"
          allowClear
          placeholder="全部状态"
          value={filters.status}
          options={[
            { value: 'normal', label: '正常' },
            { value: 'hidden', label: '隐藏' },
          ]}
          onChange={(status) => change({ status })}
        />
      </Space>
      <Table<School>
        rowKey="id"
        pagination={false}
        dataSource={q.data?.items ?? []}
        loading={q.isFetching}
        scroll={{ x: 1000 }}
        columns={[
          {
            title: '学校图标',
            render: (_, s) => <SchoolImage school={s.id} id={s.logo_asset_id} alt="学校图标" />,
          },
          ...(['name', 'short_name', 'province', 'city', 'area'] as const).map((key, i) => ({
            title: ['学校名称', '学校简称', '省份', '城市', '区域'][i],
            dataIndex: key,
          })),
          { title: '纬度', dataIndex: 'latitude' },
          { title: '经度', dataIndex: 'longitude' },
          { title: '状态', render: (_, s) => (s.status === 'normal' ? '正常' : '隐藏') },
          {
            title: '操作',
            render: (_, s) => (
              <Space>
                <Button onClick={() => setDetail(s)}>详情</Button>
                <Button onClick={() => setEditing(s)}>编辑学校</Button>
                <Button onClick={() => enter(s.id, 'settings')}>学校配置</Button>
                <Button disabled={s.status === 'hidden'} onClick={() => enter(s.id, 'members')}>
                  学校成员
                </Button>
              </Space>
            ),
          },
        ]}
      />
      <PaginationControls
        page={filters.page ?? 1}
        more={q.data?.page?.has_more}
        busy={q.isFetching}
        change={(page) => setFilters((v) => ({ ...v, page }))}
      />
      <Modal
        open={creating || !!editing}
        title={editing ? '编辑学校' : '新建学校'}
        footer={null}
        onCancel={() => {
          setCreating(false)
          setEditing(undefined)
        }}
        destroyOnHidden
      >
        <SchoolForm key={editing?.id ?? 'new'} school={editing} platform onSaved={saved} />
      </Modal>
      <Drawer
        open={!!detail}
        title={`${detail?.name ?? ''} · 学校详情`}
        onClose={() => setDetail(undefined)}
      >
        {detail && (
          <>
            <Descriptions
              column={1}
              items={[
                ...(
                  [
                    'name',
                    'short_name',
                    'province',
                    'city',
                    'area',
                    'latitude',
                    'longitude',
                    'status',
                    'theme_primary',
                    'theme_secondary',
                    'theme_text_color',
                  ] as const
                ).map((k, i) => ({
                  key: k,
                  label: [
                    '学校名称',
                    '学校简称',
                    '省份',
                    '城市',
                    '区域',
                    '纬度',
                    '经度',
                    '状态',
                    '主题主色',
                    '主题辅色',
                    '主题文字色',
                  ][i],
                  children:
                    k === 'status'
                      ? detail.status === 'normal'
                        ? '正常'
                        : '隐藏'
                      : String(detail[k] ?? '未填写'),
                })),
              ]}
            />
            <SchoolImage school={detail.id} id={detail.logo_asset_id} alt="学校图标" />
            <SchoolImage school={detail.id} id={detail.header_bg_asset_id} alt="头部背景图" />
          </>
        )}
      </Drawer>
    </Space>
  )
}
