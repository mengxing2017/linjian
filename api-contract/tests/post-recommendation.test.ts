import { createSchoolPermissionsClient } from "../src/modules/schools";
import { client } from "../src/http";
import { afterEach, expect, it, vi } from "vitest";
import { readFileSync } from "node:fs";
import { workspace, type RecommendPostInput } from "../src/modules/business";
import { setCsrf } from "../src/http";
const fixture = (name: string) =>
  JSON.parse(
    readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), "utf8"),
  );
afterEach(() => {
  vi.unstubAllGlobals();
  setCsrf("");
});
it("uses one new-event endpoint with explicit snapshot and preserves the request identity on retries", async () => {
  const output = fixture("post-recommendation");
  const fetch = vi
    .fn()
    .mockImplementation(async () =>
      Response.json(output, {
        headers: { "content-type": "application/vnd.api+json" },
      }),
    );
  vi.stubGlobal("fetch", fetch);
  setCsrf("csrf-recommendation");
  const input = fixture("post-recommendation-input");
  const result = await workspace.postManagement.recommend(
    "school",
    "post",
    input.data.attributes as RecommendPostInput,
  );
  await workspace.postManagement.recommend(
    "school",
    "post",
    input.data.attributes,
  );
  expect(result.id).toBe(output.data.id);
  expect(result.trigger_type).toBe("manual_recommendation");
  for (const [url, options] of fetch.mock.calls) {
    expect(url).toBe(
      "/api/v1/schools/school/post-management/post/recommendations",
    );
    expect(JSON.parse(options.body)).toEqual(input);
    expect(options.headers["X-CSRF-Token"]).toBe("csrf-recommendation");
  }
});
it("keeps recommendation grants and safe termination separate from event recovery", async () => {
  const fetch = vi
    .fn()
    .mockImplementation(async () =>
      Response.json(
        { data: null },
        { headers: { "content-type": "application/vnd.api+json" } },
      ),
    );
  vi.stubGlobal("fetch", fetch);
  await createSchoolPermissionsClient(client).recommendation("school", "operator", {catalog_version:"2026-11-20.4",expected_revision:"9",effect:"allow",reason:"岗位配置"});
  await workspace.terminateSource("school", "event", "核对为未尝试且无法恢复");
  expect(JSON.parse(fetch.mock.calls[0][1].body).data).toEqual({
    type: "post-recommendation-capabilities",
    attributes: { catalog_version:"2026-11-20.4",expected_revision:"9",effect:"allow",reason:"岗位配置" },
  });
  expect(JSON.parse(fetch.mock.calls[1][1].body).data.type).toBe(
    "source-event-terminations",
  );
});
