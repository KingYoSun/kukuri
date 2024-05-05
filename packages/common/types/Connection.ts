import { z } from "zod";

export const RoomSchema = z.object({
  id: z.string(),
  name: z.string(),
});

export type Room = z.infer<typeof RoomSchema>;
