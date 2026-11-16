import { createContext, useContext, type PropsWithChildren } from 'react'
import { afterEach, expect, it } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import { theme } from 'antd'

const Context = createContext('missing')
function Probe() {
  const { token } = theme.useToken()
  return <><p>{`${useContext(Context)}:${token.motion}`}</p><button style={{ display: 'none' }}>hidden action</button></>
}
function Wrapper({ children }: PropsWithChildren) {
  return <Context.Provider value="wrapped">{children}</Context.Provider>
}
afterEach(cleanup)
it('disables Ant Design motion while preserving custom wrappers, rerenders and visibility checks', () => {
  const view = render(<Probe />, { wrapper: Wrapper })
  expect(screen.getByText('wrapped:false')).toBeTruthy()
  expect(screen.queryByRole('button', { name: 'hidden action' })).toBeNull()
  view.rerender(<Probe />)
  expect(screen.getByText('wrapped:false')).toBeTruthy()
})
