import { z } from "zod";

export const SettingsRequestSchema = z.object({
  PUB_KEY: z.string().optional(),
});

export type Settings = z.infer<typeof SettingsRequestSchema>;

export const DefaultSettings: Settings = {
  PUB_KEY: "",
};
