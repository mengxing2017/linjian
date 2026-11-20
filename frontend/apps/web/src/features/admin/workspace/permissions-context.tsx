import { createContext, useContext } from 'react'
export const SchoolPermissionsContext = createContext<{ platform: boolean; effective: readonly string[] }>({ platform: false, effective: [] })
export function useSchoolPermission() {
  const scope = useContext(SchoolPermissionsContext)
  return (key: string) => scope.platform || scope.effective.includes(key)
}
