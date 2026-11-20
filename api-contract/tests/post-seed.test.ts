import { describe, expect, it } from "vitest";
import { createClient } from "../src/http/client";
import { resourceType, decodeResource } from "../src/http/protocol";
import {
  createPostSeedClient,
  type SeedBatch,
  type SeedPreviewInput,
} from "../src/modules/business/post-seed";
import preview from "../fixtures/post-seed-preview.json";
import input from "../fixtures/post-seed-preview-input.json";
import commit from "../fixtures/post-seed-commit-input.json";
import revoke from "../fixtures/post-seed-revoke-input.json";
import avatar from "../fixtures/post-seed-avatar.json";

describe("simulation posts contract", () => {
  it("loads the migrated avatar catalog with stable slots and version", async () => {
    const client = createClient(async (request) => {
      expect(request.path).toBe(
        `/api/v1/schools/${preview.data.attributes.school_id}/post-seed-avatars`,
      );
      return {
        status: 200,
        headers: { "content-type": "application/vnd.api+json" },
        body: { data: [avatar.data] },
      };
    });
    const result = await createPostSeedClient(client).avatars(
      preview.data.attributes.school_id,
    );
    expect(result.items[0]).toMatchObject({
      id: "2",
      slot: 2,
      version: "linjian-student-avatars-v1",
      url: avatar.data.attributes.url,
    });
  });
  it("uses one final resource type for preview, commit and revoke", async () => {
    const requests: Array<{ path: string; body: unknown }> = [];
    const client = createClient(async (request) => {
      requests.push({
        path: request.path,
        body:
          typeof request.body === "string"
            ? JSON.parse(request.body)
            : request.body,
      });
      return {
        status: 200,
        headers: { "content-type": "application/vnd.api+json" },
        body: preview,
      };
    });
    const api = createPostSeedClient(client);
    const school = preview.data.attributes.school_id;
    const result = await api.preview(
      school,
      input.data.attributes as SeedPreviewInput,
    );
    expect(result.preview_payload.authors[0].nickname).toBe("校园微风");
    expect(requests[0].body).toEqual(input);
    await api.commit(school, result.id, commit.data.attributes);
    expect(requests[1].body).toEqual(commit);
    await api.revoke(
      school,
      result.id,
      revoke.data.attributes.revision,
      revoke.data.attributes.reason,
    );
    expect(requests[2].body).toEqual(revoke);
    expect(resourceType(requests[2].path)).toBe("post-seed-batches");
    expect(
      resourceType(
        `/api/v1/schools/${school}/post-seed-batches/${result.id}/items`,
      ),
    ).toBe("post-seed-items");
  });
  it("keeps the fixed preview and internal data in the management resource only", () => {
    const result = decodeResource(preview.data) as unknown as SeedBatch;
    expect(result.preview_payload.rows).toHaveLength(1);
    expect(result.preview_payload.rows[0].display_published_at).toBe(
      "2026-01-01T01:00:00Z",
    );
    expect(result.created_at).not.toBe(
      result.preview_payload.rows[0].display_published_at,
    );
    expect(result).not.toHaveProperty("commit_idempotency_key");
    expect(result).not.toHaveProperty("policy_digest");
  });
});
