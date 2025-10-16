import { invoke } from '@tauri-apps/api/core';

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

export interface GossipMetricDetails {
  total: number;
  failures: number;
  last_success_ms: number | null;
  last_failure_ms: number | null;
}

export interface GossipMetrics {
  joins: number;
  leaves: number;
  broadcasts_sent: number;
  messages_received: number;
  join_details: GossipMetricDetails;
  leave_details: GossipMetricDetails;
  broadcast_details: GossipMetricDetails;
  receive_details: GossipMetricDetails;
}

export const p2pApi = {
  /**
   * P2P讖溯・繧貞・譛溷喧
   */
  initialize: () => invoke<void>('initialize_p2p'),

  /**
   * 繝医ヴ繝・け縺ｫ蜿ょ刈
   */
  joinTopic: (topicId: string, initialPeers: string[] = []) =>
    invoke<void>('join_p2p_topic', { topicId, initialPeers }),

  /**
   * 繝医ヴ繝・け蜷阪〒蜿ょ刈
   */
  joinTopicByName: (topicName: string, initialPeers: string[] = []) =>
    invoke<void>('join_topic_by_name', { topicName, initialPeers }),

  /**
   * 繝医ヴ繝・け縺九ｉ髮｢閼ｱ
   */
  leaveTopic: (topicId: string) => invoke<void>('leave_p2p_topic', { topicId }),

  /**
   * 繝医ヴ繝・け縺ｫ繝｡繝・そ繝ｼ繧ｸ繧偵ヶ繝ｭ繝ｼ繝峨く繝｣繧ｹ繝・   */
  broadcast: (topicId: string, content: string) =>
    invoke<void>('broadcast_to_topic', { topicId, content }),

  /**
   * P2P謗･邯夂憾諷九ｒ蜿門ｾ・   */
  getStatus: () => invoke<P2PStatus>('get_p2p_status'),

  /**
   * 繝弱・繝峨い繝峨Ξ繧ｹ繧貞叙蠕・   */
  getNodeAddress: () => invoke<string[]>('get_node_address'),

  /**
   * 謖・ｮ壹＆繧後◆繝斐い繧｢繝峨Ξ繧ｹ縺ｫ謇句虚縺ｧ謗･邯・   */
  connectToPeer: (peerAddress: string) => invoke<void>('connect_to_peer', { peerAddress }),

  // Bootstrap UI
  getBootstrapConfig: () => invoke<string>('get_bootstrap_config'),
  setBootstrapNodes: (nodes: string[]) => invoke<string>('set_bootstrap_nodes', { nodes }),
  clearBootstrapNodes: () => invoke<string>('clear_bootstrap_nodes'),

  /**
   * Gossip繝｡繝医Μ繧ｯ繧ｹ繧貞叙蠕・   */
  getMetrics: () => invoke<GossipMetrics>('get_p2p_metrics'),
};
