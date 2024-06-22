import { z } from "zod";

export const PeerPoolIndexReqSchema = z.object({
  topic: z.string(),
});

export type PeerPoolIndexReq = z.infer<typeof PeerPoolIndexReqSchema>;

export const PeerPoolCreateReqSchema = z.object({
  topic: z.string(),
  maddr: z.string(),
  connectionCount: z.number().int().nullable(),
});

export type PeerPoolCreateReq = z.infer<typeof PeerPoolCreateReqSchema>;

export type Peer = {
  id: number;
  connectionCount: number;
  maddr: string;
  topic: string;
  createdAt: Date;
  updatedAt: Date;
};
