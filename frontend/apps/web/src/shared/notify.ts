import { App } from 'antd'
import { useEffect } from 'react'

let notify: ((content: string) => void) | null = null

/** Mount once inside Ant Design's App context. */
export function NotifyBridge() {
  const { message } = App.useApp()
  useEffect(() => {
    notify = (content) =>
      message.success({ content, key: 'save-success', duration: 3 })
    return () => {
      notify = null
    }
  }, [message])
  return null
}

export function notifySuccess(content: string) {
  notify?.(content)
}
