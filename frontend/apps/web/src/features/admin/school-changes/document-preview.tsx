import { useEffect, useState } from 'react'
import { Alert, Image } from 'antd'
import { schoolChanges } from './api'

export function DocumentPreview({ request, document }: { request: string; document: string }) {
  const [src, setSrc] = useState('')
  const [error, setError] = useState('')
  useEffect(() => {
    const controller = new AbortController()
    let url = ''
    setSrc('')
    setError('')
    void schoolChanges.document(document, request, controller.signal).then(bytes => {
      if (controller.signal.aborted) return
      url = URL.createObjectURL(new Blob([bytes], { type: 'image/png' }))
      setSrc(url)
    }).catch(failure => {
      if (!controller.signal.aborted) setError(failure instanceof Error ? failure.message : '证件读取失败')
    })
    return () => { controller.abort(); if (url) URL.revokeObjectURL(url) }
  }, [document, request])
  if (error) return <Alert type="error" title={error} />
  return src ? <Image src={src} width={240} preview={false} alt="本次换校的目标学校学生证明" /> : <span>正在受控读取证件…</span>
}
