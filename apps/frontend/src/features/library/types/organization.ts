import { z } from "zod";

export const organizationNameSchema = z.object({
  name: z
    .string()
    .trim()
    .min(1, "Enter a name.")
    .max(100, "Use at most 100 characters."),
});

export type OrganizationNameForm = z.infer<typeof organizationNameSchema>;
