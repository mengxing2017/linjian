import { it, expect, vi } from "vitest";
import { readFileSync } from "node:fs";
import { createClient } from "../src/http/client";
import { createSchoolsClient } from "../src/modules/schools";
const fixture = (s: string) =>
  JSON.parse(
    readFileSync(new URL(`../fixtures/${s}.json`, import.meta.url), "utf8"),
  );
it("uses complete school profiles and strict input wire fixtures", async () => {
  const transport = vi
    .fn()
    .mockResolvedValue({
      status: 200,
      headers: { "content-type": "application/vnd.api+json" },
      body: fixture("school-profile"),
    });
  const api = createSchoolsClient(createClient(transport)),
    profile = await api.detail("s");
  expect(profile.status).toBe("normal");
  expect(profile).not.toHaveProperty("active");
  await api.update("s", fixture("school-profile-input").data.attributes);
  expect(JSON.parse(transport.mock.calls[1][0].body)).toEqual(
    fixture("school-profile-input"),
  );
});
it("preserves school directory pagination and encodes search without school management data", async () => {
  const transport = vi
    .fn()
    .mockResolvedValue({
      status: 200,
      headers: { "content-type": "application/vnd.api+json" },
      body: {
        data: [],
        meta: { page: { number: 2, size: 30, has_more: true } },
      },
    });
  const result = await createSchoolsClient(createClient(transport)).directory({
    page: 2,
    search: "名称 &",
    city: "广州",
  });
  expect(result.page?.has_more).toBe(true);
  expect(transport.mock.calls[0][0].path).toContain(
    "search=%E5%90%8D%E7%A7%B0%20%26",
  );
});
