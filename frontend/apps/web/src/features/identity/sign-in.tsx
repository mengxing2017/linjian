import { Alert, Button, Form, Input } from 'antd'
import { Field } from '../../shared/field'
import {
  SiteBrand,
  SiteFooter,
  text,
  useSiteSettings,
  useSiteTitle,
} from '../site'

export function SignIn({
  error,
  busy,
  onLogin,
}: {
  error: string
  busy: boolean
  onLogin: (values: { username: string; password: string }) => Promise<void>
}) {
  const { settings } = useSiteSettings()
  const intro = text(settings.login_intro)
  useSiteTitle()
  return (
    <div className="auth-page">
      <section className="auth-panel">
        <SiteBrand className="auth-brand" />
        <div>
          <h1>登录工作空间</h1>
          {intro && <p>{intro}</p>}
        </div>
        {error && <Alert type="error" title={error} showIcon />}
        <Form layout="vertical" onFinish={onLogin} disabled={busy} className="business-form">
          <Field name="username" required label={<>账号</>}>
            <Input
              name="username"
              placeholder="请输入账号"
              autoComplete="username"
              maxLength={100}
            />
          </Field>
          <Field name="password" required label={<>密码</>}>
            <Input.Password
              name="password"
              placeholder="请输入密码"
              autoComplete="current-password"
              maxLength={128}
            />
          </Field>
          <Button type="primary" htmlType="submit" block loading={busy} disabled={busy}>
            {busy ? '正在登录…' : '登录'}
          </Button>
        </Form>
        <p className="auth-caption">使用管理员分配的账号进入工作空间</p>
        <a href="/docs/guide/quick-start">登录与使用帮助</a>
      </section>
      <SiteFooter />
    </div>
  )
}
