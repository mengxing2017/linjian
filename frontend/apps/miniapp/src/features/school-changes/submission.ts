export function newSubmissionKey() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, character => {
    const random = Math.floor(Math.random() * 16)
    return (character === 'x' ? random : (random & 3) | 8).toString(16)
  })
}
export function createSubmissionKeys() {
  let fingerprint = ''
  let key = ''
  return (payload: unknown) => {
    const next = JSON.stringify(payload)
    if (next !== fingerprint || !key) {
      fingerprint = next
      key = newSubmissionKey()
    }
    return key
  }
}
