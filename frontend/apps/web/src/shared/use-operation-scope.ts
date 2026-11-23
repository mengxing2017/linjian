import { useEffect, useRef } from 'react'

export function useOperationScope(scope: string) {
  const current = useRef({ scope, active: true })
  if (current.current.scope !== scope) current.current = { scope, active: true }
  useEffect(() => {
    const turn = current.current
    turn.active = true
    return () => {
      turn.active = false
    }
  }, [scope])
  return () => {
    const turn = current.current
    return () => turn.active && current.current === turn
  }
}
