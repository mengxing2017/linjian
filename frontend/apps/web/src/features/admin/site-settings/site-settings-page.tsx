import { useEffect, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Button,
  Card,
  Form,
  Input,
  InputNumber,
  Modal,
  Select,
  Space,
  Table,
} from 'antd'
import {
  entryTitle,
  pageTitle,
  publicFromAdmin,
  publicFromInput,
  shanghaiYear,
  SiteBrand,
  SiteFooter,
  siteSettings,
  text,
  useSiteSettings,
  type AdminSiteSettings,
  type SiteSettingsHistory,
  type SiteSettingsInput,
} from '../../site'
import { useUnsavedChanges } from '../../../shared/unsaved-changes'
import { formatDate } from '../../../shared/format-date'
import { MediaPicker } from '../operations/media-picker'

type FormValues = Omit<SiteSettingsInput, 'revision'>

const EMPTY_FORM: FormValues = {
  site_name: null,
  logo_asset_id: null,
  title_subtitle: null,
  login_intro: null,
  icp_number: null,
  police_number: null,
  police_url: null,
  copyright_owner: null,
  copyright_start_year: null,
  copyright_end_mode: null,
  copyright_end_year: null,
  copyright_notice: null,
}

const fieldLabels: Record<string, string> = {
  site_name: '网站名称',
  logo_asset_id: '平台 Logo',
  title_subtitle: '浏览器标题副标题',
  login_intro: '登录页简介',
  icp_number: 'ICP备案号',
  police_number: '公安联网备案号',
  police_url: '公安备案查询链接',
  copyright_owner: '版权所有者',
  copyright_start_year: '版权起始年',
  copyright_end_mode: '截止年模式',
  copyright_end_year: '固定截止年',
  copyright_notice: '补充版权声明',
}

function clean(value: string | null | undefined): string | null {
  const result = typeof value === 'string' ? value.trim() : ''
  return result || null
}

function toForm(settings: AdminSiteSettings): FormValues {
  return {
    site_name: settings.site_name,
    logo_asset_id: settings.logo_asset_id,
    title_subtitle: settings.title_subtitle,
    login_intro: settings.login_intro,
    icp_number: settings.icp_number,
    police_number: settings.police_number,
    police_url: settings.police_url,
    copyright_owner: settings.copyright_owner,
    copyright_start_year: settings.copyright_start_year,
    copyright_end_mode: settings.copyright_end_mode,
    copyright_end_year: settings.copyright_end_year,
    copyright_notice: settings.copyright_notice,
  }
}

function toInput(values: FormValues, revision: number): SiteSettingsInput {
  const owner = clean(values.copyright_owner)
  return {
    revision,
    site_name: clean(values.site_name),
    logo_asset_id: values.logo_asset_id ?? null,
    title_subtitle: clean(values.title_subtitle),
    login_intro: clean(values.login_intro),
    icp_number: clean(values.icp_number),
    police_number: clean(values.police_number),
    police_url: clean(values.police_url),
    copyright_owner: owner,
    copyright_start_year: owner ? (values.copyright_start_year ?? null) : null,
    copyright_end_mode: owner ? (values.copyright_end_mode ?? null) : null,
    copyright_end_year:
      owner && values.copyright_end_mode === 'fixed'
        ? (values.copyright_end_year ?? null)
        : null,
    copyright_notice: owner ? clean(values.copyright_notice) : null,
  }
}

function changedFields(loaded: FormValues, current: SiteSettingsInput): string[] {
  return Object.keys(fieldLabels)
    .filter((key) => {
      const before = loaded[key as keyof FormValues] ?? null
      const after = current[key as keyof SiteSettingsInput] ?? null
      return JSON.stringify(before) !== JSON.stringify(after)
    })
    .map((key) => fieldLabels[key])
}

function clearedGroups(loaded: FormValues, current: SiteSettingsInput): string[] {
  const cleared: string[] = []
  if (loaded.site_name && !current.site_name) cleared.push('网站名称')
  if (loaded.logo_asset_id && !current.logo_asset_id) cleared.push('平台 Logo')
  if (loaded.icp_number && !current.icp_number) cleared.push('ICP备案号')
  if (
    (loaded.police_number || loaded.police_url) &&
    !current.police_number &&
    !current.police_url
  )
    cleared.push('公安备案信息')
  if (loaded.copyright_owner && !current.copyright_owner) cleared.push('版权信息')
  return cleared
}

function LogoPicker({
  value,
  onChange,
  onBusyChange,
}: {
  value?: string | null
  onChange?: (id: string | null) => void
  onBusyChange?: (busy: boolean) => void
}) {
  return (
    <MediaPicker
      value={value ?? null}
      label="平台 Logo"
      onChange={(id) => onChange?.(id)}
      onBusyChange={onBusyChange}
    />
  )
}

function policeLink(_: unknown, value?: string) {
  const link = value?.trim()
  if (!link) return Promise.resolve()
  try {
    const parsed = new URL(link)
    if (
      parsed.protocol !== 'https:' ||
      parsed.hostname !== 'beian.mps.gov.cn' ||
      parsed.username ||
      parsed.password ||
      (parsed.port && parsed.port !== '443')
    )
      throw new Error('invalid')
    return Promise.resolve()
  } catch {
    return Promise.reject(
      new Error('必须使用 https://beian.mps.gov.cn 的官方查询链接'),
    )
  }
}

export function SiteSettings() {
  const [form] = Form.useForm<FormValues>()
  const cache = useQueryClient()
  const { apply } = useSiteSettings()
  const setGlobalDirty = useUnsavedChanges()
  const [revision, setRevision] = useState(0)
  const [loaded, setLoaded] = useState<FormValues>(EMPTY_FORM)
  const [dirty, setDirty] = useState(false)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [done, setDone] = useState('')
  const [conflict, setConflict] = useState(false)
  const [uncertain, setUncertain] = useState(false)
  const submitting = useRef(false)
  const query = useQuery({
    queryKey: ['site-settings', 'admin'],
    queryFn: ({ signal }) => siteSettings.read(signal),
    retry: false,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
  })
  useEffect(() => {
    if (!query.data) return
    const next = toForm(query.data)
    setRevision(query.data.revision)
    setLoaded(next)
    form.setFieldsValue(next)
    setDirty(false)
  }, [form, query.data])
  useEffect(() => {
    setGlobalDirty(dirty)
    return () => setGlobalDirty(false)
  }, [dirty, setGlobalDirty])
  useEffect(() => {
    if (!dirty) return
    const warn = (event: BeforeUnloadEvent) => {
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', warn)
    return () => window.removeEventListener('beforeunload', warn)
  }, [dirty])
  const values = (Form.useWatch([], form) as FormValues | undefined) ?? loaded
  const input = toInput(values, revision)
  const preview = publicFromInput(
    input,
    input.logo_asset_id
      ? `/api/v1/platform/resource-artwork/${input.logo_asset_id}`
      : null,
  )
  const changed = changedFields(loaded, input)
  const emptyPreview =
    !text(preview.site_name) &&
    !text(preview.logo_url) &&
    !text(preview.login_intro) &&
    !text(preview.icp_number) &&
    !text(preview.police_number) &&
    !text(preview.copyright_owner)
  const currentYear = shanghaiYear()
  const owner = text(values.copyright_owner)
  const mode = values.copyright_end_mode
  async function save(next: FormValues, confirmed: boolean) {
    if (submitting.current) return
    const input = toInput(next, revision)
    if (!confirmed) {
      const cleared = clearedGroups(loaded, input)
      if (cleared.length) {
        Modal.confirm({
          title: '确认清空已有站点信息？',
          content: `保存后将清空：${cleared.join('、')}。清空结果会同步到公开页面。`,
          okText: '确认清空',
          cancelText: '继续编辑',
          okButtonProps: { danger: true },
          onOk: () => save(next, true),
        })
        return
      }
    }
    setBusy(true)
    setError('')
    setDone('')
    setConflict(false)
    submitting.current = true
    try {
      const saved = await siteSettings.update(input)
      const savedForm = toForm(saved)
      setRevision(saved.revision)
      setLoaded(savedForm)
      setDirty(false)
      setUncertain(false)
      setFormValues(savedForm)
      apply(publicFromAdmin(saved))
      cache.setQueryData(['site-settings', 'admin'], saved)
      setDone(`已保存并生效，当前版本 ${saved.revision}`)
    } catch (e) {
      const failure = e as { status?: number; message?: string }
      if (failure.status === 409) setConflict(true)
      if (!failure.status || failure.status >= 500) setUncertain(true)
      setError(failure.message ?? '保存失败')
    } finally {
      submitting.current = false
      setBusy(false)
    }
  }
  function setFormValues(next: FormValues) {
    form.setFieldsValue(next)
  }
  async function reload() {
    setError('')
    setConflict(false)
    setUncertain(false)
    await query.refetch()
  }
  return (
    <Space orientation="vertical" className="full-width" size="middle">
      <Card
        title="生效范围：全平台"
        extra={
          query.data && (
            <span>
              当前版本 {query.data.revision}
              {query.data.updated_by_name && ` · ${query.data.updated_by_name}`}
              {query.data.updated_at && ` · ${formatDate(query.data.updated_at)}`}
            </span>
          )
        }
      >
        {query.error && (
          <Alert
            type="error"
            title={query.error.message}
            action={<Button onClick={() => void reload()}>重试</Button>}
          />
        )}
        {error && <Alert type="error" title={error} />}
        {done && <Alert type="success" title={done} />}
        {conflict && (
          <Alert
            type="warning"
            title="配置已被其他管理员更新，本地输入仍保留。请重新加载当前版本后再合并。"
            action={<Button onClick={() => void reload()}>重新加载</Button>}
          />
        )}
        {uncertain && (
          <Alert
            type="warning"
            title="保存结果尚未确认，请重新读取当前版本核对；不要直接覆盖。"
            action={<Button onClick={() => void reload()}>核对当前版本</Button>}
          />
        )}
        <Form
          form={form}
          layout="vertical"
          initialValues={EMPTY_FORM}
          onValuesChange={() => {
            setDirty(true)
            setDone('')
            setError('')
          }}
          onFinish={(value) => void save(value, false)}
          disabled={busy || !query.data}
        >
          <h3>站点品牌</h3>
          <Form.Item
            name="site_name"
            label="网站名称"
            rules={[{ max: 30, message: '网站名称最多 30 个字符' }]}
          >
            <Input maxLength={30} placeholder="留空则不展示品牌文字" />
          </Form.Item>
          <p className="field-caption">
            网站名称为空时，后台页头、登录页和社区入口不显示品牌文字及容器。
          </p>
          <Form.Item name="logo_asset_id" label="平台 Logo">
            <LogoPicker onBusyChange={setBusy} />
          </Form.Item>
          <p className="field-caption">
            建议使用正方形 PNG/WebP。未配置 Logo 时不显示图标，也不以固定字母代替；名称与 Logo
            分别按是否配置决定展示。
          </p>
          <Form.Item
            name="title_subtitle"
            label="浏览器标题副标题"
            rules={[{ max: 40, message: '副标题最多 40 个字符' }]}
          >
            <Input maxLength={40} placeholder="仅用于登录页和平台入口标题" />
          </Form.Item>
          <Form.Item
            name="login_intro"
            label="登录页简介"
            rules={[{ max: 100, message: '登录页简介最多 100 个字符' }]}
          >
            <Input.TextArea maxLength={100} rows={2} placeholder="留空不展示简介" />
          </Form.Item>

          <h3>备案信息</h3>
          <Form.Item
            name="icp_number"
            label="ICP备案号"
            rules={[{ max: 64, message: 'ICP备案号最多 64 个字符' }]}
          >
            <Input maxLength={64} placeholder="填写实际核发的完整备案编号" />
          </Form.Item>
          <p className="field-caption">
            查询入口固定为 https://beian.miit.gov.cn/，不提供隐藏开关或示例号码。
          </p>
          <Form.Item
            name="police_number"
            label="公安联网备案号"
            dependencies={['police_url']}
            rules={[
              { max: 64, message: '公安备案号最多 64 个字符' },
              {
                validator: (_, value) => {
                  const link = clean(form.getFieldValue('police_url'))
                  if ((clean(value) === null) !== (link === null))
                    return Promise.reject(
                      new Error('公安备案号与查询链接必须同时填写或同时清空'),
                    )
                  return Promise.resolve()
                },
              },
            ]}
          >
            <Input maxLength={64} placeholder="例如“京公网安备 …号”" />
          </Form.Item>
          <Form.Item
            name="police_url"
            label="公安备案查询链接"
            dependencies={['police_number']}
            rules={[
              { validator: policeLink },
              {
                validator: (_, value) => {
                  const number = clean(form.getFieldValue('police_number'))
                  if ((clean(value) === null) !== (number === null))
                    return Promise.reject(
                      new Error('公安备案号与查询链接必须同时填写或同时清空'),
                    )
                  return Promise.resolve()
                },
              },
            ]}
          >
            <Input
              maxLength={2048}
              placeholder="https://beian.mps.gov.cn/…"
            />
          </Form.Item>

          <h3>版权信息</h3>
          <Form.Item
            name="copyright_owner"
            label="版权所有者"
            rules={[{ max: 100, message: '版权所有者最多 100 个字符' }]}
          >
            <Input maxLength={100} placeholder="填写真实权利主体" />
          </Form.Item>
          <Space wrap align="start">
            <Form.Item name="copyright_start_year" label="版权起始年">
              <InputNumber min={1900} max={currentYear} precision={0} />
            </Form.Item>
            <Form.Item name="copyright_end_mode" label="截止年模式">
              <Select
                allowClear
                style={{ width: 160 }}
                options={[
                  { value: 'auto', label: '自动当前年' },
                  { value: 'fixed', label: '固定年份' },
                ]}
              />
            </Form.Item>
            {mode === 'fixed' && (
              <Form.Item name="copyright_end_year" label="固定截止年">
                <InputNumber
                  min={values.copyright_start_year ?? 1900}
                  max={currentYear}
                  precision={0}
                />
              </Form.Item>
            )}
          </Space>
          <p className="field-caption">
            自动模式按 Asia/Shanghai 计算截止年；所有者与年份必须整组填写或整组清空。
          </p>
          <Form.Item
            name="copyright_notice"
            label="补充版权声明"
            rules={[{ max: 200, message: '补充声明最多 200 个字符' }]}
          >
            <Input.TextArea maxLength={200} rows={2} />
          </Form.Item>
          {!owner && (
            <p className="field-caption">
              未填写版权所有者时，起始年、截止年模式和补充声明不生效。
            </p>
          )}
          <Space>
            <Button
              type="primary"
              htmlType="submit"
              loading={busy}
              disabled={busy || conflict || !query.data}
            >
              保存并生效
            </Button>
            <Button
              disabled={busy}
              onClick={() => {
                Modal.confirm({
                  title: '放弃未保存的修改？',
                  content: '表单将恢复为最近一次读取或保存的生效版本。',
                  okText: '放弃修改',
                  cancelText: '继续编辑',
                  onOk: () => {
                    form.setFieldsValue(loaded)
                    setDirty(false)
                    setError('')
                    setConflict(false)
                    setUncertain(false)
                  },
                })
              }}
            >
              取消
            </Button>
          </Space>
        </Form>
      </Card>

      <Card title="预览">
        <p>
          <strong>变更摘要：</strong>
          {changed.length ? changed.join('、') : '尚未修改'}
        </p>
        {emptyPreview ? (
          <p>当前没有可展示的站点信息，页头、登录页和页脚将保持隐藏。</p>
        ) : (
          <div className="site-settings-preview">
            <p>
              <strong>登录页标题：</strong>
              {entryTitle(preview) || '（空）'}
            </p>
            <p>
              <strong>业务页标题：</strong>
              {pageTitle('内容管理', preview) || '（空）'}
            </p>
            <div className="site-preview-login">
              <SiteBrand value={preview} className="auth-brand" />
              {text(preview.login_intro) && <p>{preview.login_intro}</p>}
            </div>
            <div className="site-preview-header">
              <SiteBrand value={preview} className="header-brand" />
              <span>论坛管理后台</span>
            </div>
            <SiteFooter value={preview} />
          </div>
        )}
      </Card>

      <Card title="变更记录">
        <Table<SiteSettingsHistory>
          rowKey="id"
          size="small"
          pagination={false}
          dataSource={query.data?.history ?? []}
          locale={{ emptyText: '暂无变更记录' }}
          columns={[
            {
              title: '时间',
              width: 190,
              render: (_, row) => row.created_at,
            },
            {
              title: '操作人',
              width: 160,
              render: (_, row) => row.actor_name ?? '—',
            },
            {
              title: '版本',
              width: 90,
              render: (_, row) => row.revision,
            },
            {
              title: '变更字段',
              render: (_, row) =>
                row.changed.length
                  ? row.changed
                      .map((key) => fieldLabels[key] ?? key)
                      .join('、')
                  : '—',
            },
            {
              title: '清空字段',
              width: 220,
              render: (_, row) =>
                row.cleared.length
                  ? row.cleared
                      .map((key) => fieldLabels[key] ?? key)
                      .join('、')
                  : '—',
            },
          ]}
          expandable={{
            expandedRowRender: (row) => (
              <pre className="site-settings-history">
                {JSON.stringify(
                  { previous: row.previous, current: row.current },
                  null,
                  2,
                )}
              </pre>
            ),
          }}
        />
        <p className="field-caption">
          恢复旧值需人工核对后重新编辑保存，会形成新的版本，不提供一键回滚。
        </p>
      </Card>
    </Space>
  )
}
