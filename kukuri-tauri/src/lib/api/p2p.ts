import { invoke } from "@tauri-apps/api/core";

export interface P2PStatus {
  connected: boolean;
  endpoint_id: string;
  active_topics: TopicStatus[];
  peer_count: number;
}

export interface TopicStatus {
  topic_id: string;
  peer_count: number;
  message_count: number;
  last_activity: number;
}

export const p2pApi = {
  /**
   * P2P機能を初期化
   */
  initialize: () => invoke<void>("initialize_p2p"),

  /**
   * トピックに参加
   */
  joinTopic: (topicId: string, initialPeers: string[] = []) =>
    invoke<void>("join_p2p_topic", { topicId, initialPeers }),

  /**
   * トピック名で参加
   */
  joinTopicByName: (topicName: string, initialPeers: string[] = []) =>
    invoke<void>("join_topic_by_name", { topicName, initialPeers }),

  /**
   * トピックから離脱
   */
  leaveTopic: (topicId: string) =>
    invoke<void>("leave_p2p_topic", { topicId }),

  /**
   * トピックにメッセージをブロードキャスト
   */
  broadcast: (topicId: string, content: string) =>
    invoke<void>("broadcast_to_topic", { topicId, content }),

  /**
   * P2P接続状態を取得
   */
  getStatus: () => invoke<P2PStatus>("get_p2p_status"),

  /**
   * ノードアドレスを取得
   */
  getNodeAddress: () => invoke<string[]>("get_node_address"),
};