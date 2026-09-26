import type { LibrarySkill } from "@/types";

export interface SlugConflictGroup {
  slug: string;
  skills: Array<LibrarySkill>;
}

function bySkillId(left: LibrarySkill, right: LibrarySkill) {
  if (left.skillId === right.skillId) return 0;
  return left.skillId < right.skillId ? -1 : 1;
}

/**
 * Skills that share a slug, ordered by skill id. The order matches the one the resolver derives,
 * because it compares the ids it receives against its own ascending list.
 */
export function slugConflictGroups(
  skills: Array<LibrarySkill>,
): Array<SlugConflictGroup> {
  const groups = new Map<string, Array<LibrarySkill>>();
  for (const skill of skills)
    groups.set(skill.slug, [...(groups.get(skill.slug) ?? []), skill]);
  return [...groups.entries()]
    .filter(([, group]) => group.length > 1)
    .map(([slug, group]) => ({ slug, skills: [...group].sort(bySkillId) }));
}

export function payloadPath(skill: LibrarySkill) {
  return `library/skills/${skill.payloadDirectory}`;
}
