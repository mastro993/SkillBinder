import { describe, expect, it } from "vitest";
import type { LibrarySkill } from "@/types";
import { payloadPath, slugConflictGroups } from "../conflicts";

function skill(
  skillId: string,
  slug: string,
  payloadDirectory = slug,
): LibrarySkill {
  return {
    skillId,
    slug,
    displayName: null,
    description: null,
    validation: { status: "valid", messages: [] },
    fileCount: 1,
    totalBytes: "1",
    sources: [],
    digest: `sha256:${skillId}`,
    payloadDirectory,
  };
}

describe("slug conflict groups", () => {
  it("groups shared slugs in skill id order and leaves settled slugs alone", () => {
    const groups = slugConflictGroups([
      skill("cccc3333", "caveman"),
      skill("aaaa1111", "caveman"),
      skill("bbbb2222", "outline"),
    ]);
    expect(groups).toHaveLength(1);
    expect(groups[0]?.slug).toBe("caveman");
    expect(groups[0]?.skills.map((entry) => entry.skillId)).toEqual([
      "aaaa1111",
      "cccc3333",
    ]);
  });

  it("reports the payload directory of a copy", () => {
    expect(payloadPath(skill("aaaa", "caveman", "caveman-aaaa"))).toBe(
      "library/skills/caveman-aaaa",
    );
  });
});
