export type CommunityAction = (action: () => Promise<unknown>, options?: { refreshFailure: string }) => Promise<void>
