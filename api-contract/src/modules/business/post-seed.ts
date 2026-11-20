import { client, type Client, type Cancellation } from "../../http";

export interface SeedAuthorInput {
  key: string;
  existing_id?: string | null;
  nickname?: string | null;
  avatar_id?: number | null;
}
export interface SeedRow {
  author_key: string;
  title: string;
  content: string;
  tags: string[];
  display_published_at: string;
}
export interface SeedPreviewInput {
  name: string;
  authors: SeedAuthorInput[];
  rows: SeedRow[];
}
export interface SeedAuthor {
  key: string;
  id: string;
  existing: boolean;
  nickname: string;
  avatar_id: number;
  revision: number | null;
  avatar_url: string | null;
}
export interface SeedBatch {
  id: string;
  school_id: string;
  created_by: string;
  name: string;
  state: "preview" | "committed" | "revoked";
  revision: number;
  preview_digest: string;
  expires_at: string;
  created_at: string;
  committed_at: string | null;
  revoked_at: string | null;
  revoked_by: string | null;
  revoke_reason: string | null;
  preview_payload: {
    authors: SeedAuthor[];
    rows: SeedRow[];
    errors: Array<{ row: number; message: string }>;
    audit_mode: "manual" | "auto";
    avatar_version: string;
  };
  post_count: number;
  interaction_count: number;
}
export type SeedBatchSummary = Omit<
  SeedBatch,
  "preview_payload" | "post_count" | "interaction_count"
>;
export interface SeedItem {
  id: string;
  row_number: number;
  title: string;
  author_id: string;
  author_nickname: string;
  audit_status: string;
  visibility: string;
  created_at: string;
  display_published_at: string;
}
export interface SeedAvatar {
  id: string;
  slot: number;
  version: string;
  url: string;
}
export interface SeedExistingAuthor {
  id: string;
  nickname: string;
  revision: number;
  avatar_url: string | null;
}
export interface SeedCommit {
  revision: number;
  digest: string;
  idempotency_key: string;
}
export function createPostSeedClient(
  transport: Pick<
    Client,
    "requestResource" | "requestCollection" | "requestBinary"
  >,
) {
  const base = (school: string) =>
    `/api/v1/schools/${encodeURIComponent(school)}/post-seed-batches`;
  const detail = (school: string, batch: string) =>
    `${base(school)}/${encodeURIComponent(batch)}`;
  return {
    preview: (school: string, input: SeedPreviewInput) =>
      transport.requestResource<SeedBatch>(
        `${base(school)}/preview`,
        "POST",
        input,
      ),
    commit: (school: string, batch: string, input: SeedCommit) =>
      transport.requestResource<SeedBatch>(
        `${detail(school, batch)}/commit`,
        "POST",
        input,
      ),
    revoke: (school: string, batch: string, revision: number, reason: string) =>
      transport.requestResource<SeedBatch>(
        `${detail(school, batch)}/revoke`,
        "POST",
        { revision, reason },
      ),
    list: (
      school: string,
      page = 1,
      state = "committed",
      signal?: Cancellation,
    ) =>
      transport.requestCollection<SeedBatchSummary>(
        `${base(school)}?page[number]=${page}&page[size]=20&state=${encodeURIComponent(state)}`,
        signal,
      ),
    detail: (school: string, batch: string, signal?: Cancellation) =>
      transport.requestResource<SeedBatch>(
        detail(school, batch),
        "GET",
        undefined,
        signal,
      ),
    items: (school: string, batch: string, page = 1, signal?: Cancellation) =>
      transport.requestCollection<SeedItem>(
        `${detail(school, batch)}/items?page[number]=${page}&page[size]=30`,
        signal,
      ),
    authors: (school: string, page = 1, search = "", signal?: Cancellation) =>
      transport.requestCollection<SeedExistingAuthor>(
        `/api/v1/schools/${encodeURIComponent(school)}/post-seed-authors?page[number]=${page}&page[size]=30&search=${encodeURIComponent(search)}`,
        signal,
      ),
    avatars: (school: string, signal?: Cancellation) =>
      transport.requestCollection<SeedAvatar>(
        `/api/v1/schools/${encodeURIComponent(school)}/post-seed-avatars`,
        signal,
      ),
    authorAvatar: (school: string, author: string, signal?: Cancellation) =>
      transport.requestBinary(
        `/api/v1/schools/${encodeURIComponent(school)}/post-seed-authors/${encodeURIComponent(author)}/avatar`,
        signal,
      ),
    avatar: (school: string, slot: number, signal?: Cancellation) =>
      transport.requestBinary(
        `/api/v1/schools/${encodeURIComponent(school)}/post-seed-avatars/${slot}`,
        signal,
      ),
  };
}
export const postSeed = createPostSeedClient(client);
