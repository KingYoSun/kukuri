import { describe, it, expect, vi, beforeEach } from "vitest";
import { p2pApi } from "../p2p";
import type { P2PStatus, TopicStatus } from "../p2p";

// Tauri APIのモック
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

describe("p2pApi", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("initialize", () => {
    it("should initialize P2P", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await p2pApi.initialize();

      expect(invoke).toHaveBeenCalledWith("initialize_p2p");
      expect(invoke).toHaveBeenCalledTimes(1);
    });

    it("should handle initialization error", async () => {
      vi.mocked(invoke).mockRejectedValueOnce(new Error("Failed to initialize"));

      await expect(p2pApi.initialize()).rejects.toThrow("Failed to initialize");
    });
  });

  describe("joinTopic", () => {
    it("should join topic with initial peers", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await p2pApi.joinTopic("test-topic", ["peer1", "peer2"]);

      expect(invoke).toHaveBeenCalledWith("join_p2p_topic", {
        topicId: "test-topic",
        initialPeers: ["peer1", "peer2"],
      });
    });

    it("should join topic without initial peers", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await p2pApi.joinTopic("test-topic");

      expect(invoke).toHaveBeenCalledWith("join_p2p_topic", {
        topicId: "test-topic",
        initialPeers: [],
      });
    });
  });

  describe("joinTopicByName", () => {
    it("should join topic by name", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await p2pApi.joinTopicByName("Bitcoin", ["peer1"]);

      expect(invoke).toHaveBeenCalledWith("join_topic_by_name", {
        topicName: "Bitcoin",
        initialPeers: ["peer1"],
      });
    });
  });

  describe("leaveTopic", () => {
    it("should leave topic", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await p2pApi.leaveTopic("test-topic");

      expect(invoke).toHaveBeenCalledWith("leave_p2p_topic", {
        topicId: "test-topic",
      });
    });
  });

  describe("broadcast", () => {
    it("should broadcast message to topic", async () => {
      vi.mocked(invoke).mockResolvedValueOnce(undefined);

      await p2pApi.broadcast("test-topic", "Hello, P2P!");

      expect(invoke).toHaveBeenCalledWith("broadcast_to_topic", {
        topicId: "test-topic",
        content: "Hello, P2P!",
      });
    });
  });

  describe("getStatus", () => {
    it("should get P2P status", async () => {
      const mockStatus: P2PStatus = {
        connected: true,
        endpoint_id: "node123",
        active_topics: [
          {
            topic_id: "test-topic",
            peer_count: 5,
            message_count: 100,
            last_activity: Date.now(),
          },
        ],
        peer_count: 10,
      };

      vi.mocked(invoke).mockResolvedValueOnce(mockStatus);

      const status = await p2pApi.getStatus();

      expect(invoke).toHaveBeenCalledWith("get_p2p_status");
      expect(status).toEqual(mockStatus);
    });

    it("should handle disconnected status", async () => {
      const mockStatus: P2PStatus = {
        connected: false,
        endpoint_id: "",
        active_topics: [],
        peer_count: 0,
      };

      vi.mocked(invoke).mockResolvedValueOnce(mockStatus);

      const status = await p2pApi.getStatus();

      expect(status.connected).toBe(false);
      expect(status.active_topics).toHaveLength(0);
    });
  });

  describe("getNodeAddress", () => {
    it("should get node addresses", async () => {
      const mockAddresses = [
        "/ip4/192.168.1.1/udp/4001",
        "/ip6/::1/udp/4001",
      ];

      vi.mocked(invoke).mockResolvedValueOnce(mockAddresses);

      const addresses = await p2pApi.getNodeAddress();

      expect(invoke).toHaveBeenCalledWith("get_node_address");
      expect(addresses).toEqual(mockAddresses);
    });

    it("should handle empty addresses", async () => {
      vi.mocked(invoke).mockResolvedValueOnce([]);

      const addresses = await p2pApi.getNodeAddress();

      expect(addresses).toHaveLength(0);
    });
  });

  describe("Error handling", () => {
    it("should propagate errors from Tauri commands", async () => {
      const errorMessage = "P2P manager not initialized";
      vi.mocked(invoke).mockRejectedValueOnce(new Error(errorMessage));

      await expect(p2pApi.joinTopic("test-topic")).rejects.toThrow(errorMessage);
    });
  });

  describe("Type validation", () => {
    it("should validate TopicStatus type", () => {
      const topicStatus: TopicStatus = {
        topic_id: "test-topic",
        peer_count: 5,
        message_count: 100,
        last_activity: 1234567890,
      };

      expect(topicStatus.topic_id).toBe("test-topic");
      expect(topicStatus.peer_count).toBe(5);
      expect(topicStatus.message_count).toBe(100);
      expect(topicStatus.last_activity).toBe(1234567890);
    });

    it("should validate P2PStatus type", () => {
      const p2pStatus: P2PStatus = {
        connected: true,
        endpoint_id: "node123",
        active_topics: [],
        peer_count: 0,
      };

      expect(p2pStatus.connected).toBe(true);
      expect(p2pStatus.endpoint_id).toBe("node123");
      expect(p2pStatus.active_topics).toHaveLength(0);
      expect(p2pStatus.peer_count).toBe(0);
    });
  });
});