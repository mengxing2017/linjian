import { useEffect, useState } from 'react'
import { Avatar } from 'antd'
import { miniappUsers } from './api'

export function UserAvatar({ url }: { url: string | null }) {
  const [src, setSrc] = useState('')
  useEffect(() => {
    let live = true
    let object = ''
    setSrc('')
    if (url)
      void miniappUsers
        .avatar(url)
        .then((bytes) => {
          object = URL.createObjectURL(new Blob([bytes]))
          if (live) setSrc(object)
          else URL.revokeObjectURL(object)
        })
        .catch(() => {})
    return () => {
      live = false
      if (object) URL.revokeObjectURL(object)
    }
  }, [url])
  return <Avatar src={src || undefined}>用户</Avatar>
}
