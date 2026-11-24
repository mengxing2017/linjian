import assets from './icon-assets.json'

export type IconName = keyof typeof assets
export const iconNames = Object.keys(assets) as IconName[]

export function iconPath(name: string, selected = false): string {
  const pair = Object.prototype.hasOwnProperty.call(assets, name) ? assets[name as IconName] : assets.application
  return `/static/icons/${pair[selected ? 1 : 0]}`
}

export function documentIcon(kind: string): IconName {
  return kind === 'privacy_policy' ? 'privacy' : 'application'
}
