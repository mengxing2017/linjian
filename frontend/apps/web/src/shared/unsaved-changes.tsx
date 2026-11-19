import { createContext, useContext } from 'react'

export const UnsavedChangesContext = createContext<(dirty: boolean) => void>(
  () => {},
)

export function useUnsavedChanges() {
  return useContext(UnsavedChangesContext)
}
