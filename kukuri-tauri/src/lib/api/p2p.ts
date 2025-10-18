import { invokeCommand, invokeCommandVoid } from '@/lib/api/tauriClient';

export interface P2PStatus {
  connected: boolean;
  endpoint_id: string;
  active_topics: TopicStatus[];
  peer_count: number;
  metrics_summary: GossipMetricsSummary;
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

export interface GossipMetricsSummary {
  joins: number;
  leaves: number;
  broadcasts_sent: number;
  messages_received: number;
}

export interface GossipMetricsSection {
  joins: number;
  leaves: number;
  broadcasts_sent: number;
  messages_received: number;
  join_details: GossipMetricDetails;
  leave_details: GossipMetricDetails;
  broadcast_details: GossipMetricDetails;
  receive_details: GossipMetricDetails;
}

export interface MainlineMetrics {
  connected_peers: number;
  connection_attempts: number;
  connection_successes: number;
  connection_failures: number;
  connection_last_success_ms: number | null;
  connection_last_failure_ms: number | null;
  routing_attempts: number;
  routing_successes: number;
  routing_failures: number;
  routing_success_rate: number;
  routing_last_success_ms: number | null;
  routing_last_failure_ms: number | null;
  reconnect_attempts: number;
  reconnect_successes: number;
  reconnect_failures: number;
  last_reconnect_success_ms: number | null;
  last_reconnect_failure_ms: number | null;
}

export interface P2PMetrics {
  gossip: GossipMetricsSection;
  mainline: MainlineMetrics;
}

export interface BootstrapConfig {
  mode: 'default' | 'custom';
  nodes: string[];
}

export const p2pApi = {
  /**
   * P2P讖溯・繧貞・譛溷喧
   */
  initialize: () => invokeCommandVoid('initialize_p2p'),

  /**
   * 繝医ヴ繝・け縺ｫ蜿ょ刈
   */
  joinTopic: (topicId: string, initialPeers: string[] = []) =>
    invokeCommandVoid('join_p2p_topic', { topicId, initialPeers }),

  /**
   * 繝医ヴ繝・け蜷阪〒蜿ょ刈
   */
  joinTopicByName: (topicName: string, initialPeers: string[] = []) =>
    invokeCommandVoid('join_topic_by_name', { topicName, initialPeers }),

  /**
   * 繝医ヴ繝・け縺九ｉ髮｢閼ｱ
   */
  leaveTopic: (topicId: string) => invokeCommandVoid('leave_p2p_topic', { topicId }),

  /**
   * 繝医ヴ繝・け縺ｫ繝｡繝・そ繝ｼ繧ｸ繧偵ヶ繝ｭ繝ｼ繝峨く繝｣繧ｹ繝・   */
  broadcast: (topicId: string, content: string) =>
    invokeCommandVoid('broadcast_to_topic', { topicId, content }),

  /**
   * P2P謗･邯夂憾諷九ｒ蜿門ｾ・   */
  getStatus: () => invokeCommand<P2PStatus>('get_p2p_status'),

  /**
   * 繝弱・繝峨い繝峨Ξ繧ｹ繧貞叙蠕・   */
  getNodeAddress: async () => {
    const response = await invokeCommand<{ addresses: string[] }>('get_node_address');
    return response.addresses;
  },

  /**
   * 謖・ｮ壹＆繧後◆繝斐い繧｢繝峨Ξ繧ｹ縺ｫ謇句虚縺ｧ謗･邯・   */
  connectToPeer: (peerAddress: string) =>
    invokeCommandVoid('connect_to_peer', { peerAddress }),

  // Bootstrap UI
  getBootstrapConfig: () => invokeCommand<BootstrapConfig>('get_bootstrap_config'),
  setBootstrapNodes: (nodes: string[]) => invokeCommandVoid('set_bootstrap_nodes', { nodes }),
  clearBootstrapNodes: () => invokeCommandVoid('clear_bootstrap_nodes'),

  /**
   * Gossip繝｡繝医Μ繧ｯ繧ｹ繧貞叙蠕・   */
  getMetrics: () => invokeCommand<P2PMetrics>('get_p2p_metrics'),
};
