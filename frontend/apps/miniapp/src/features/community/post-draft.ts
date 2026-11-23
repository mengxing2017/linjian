export function postDraftKey(account: string | undefined, school: string, post?: string) {
  return `linjian:draft:v2:${account}:${school}:${post ?? 'new'}`
}
