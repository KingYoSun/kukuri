// ref: https://github.com/libp2p/universal-connectivity/blob/main/js-peer/src/lib/libp2p.ts

import { createLibp2p, type Libp2p } from "libp2p";
import { bootstrap } from "@libp2p/bootstrap";
import { yamux } from "@chainsafe/libp2p-yamux";
import { noise } from "@chainsafe/libp2p-noise";
import { gossipsub } from "@chainsafe/libp2p-gossipsub";
import { PUBSUB_PEER_DISCOVERY } from "../lib/constraints";
import type { PubSub, Message, SignedMessage } from "@libp2p/interface";
import { webRTC, webRTCDirect } from "@libp2p/webrtc";
import { webSockets } from "@libp2p/websockets";
import { webTransport } from "@libp2p/webtransport";
import { circuitRelayTransport } from "@libp2p/circuit-relay-v2";
import { createDelegatedRoutingV1HttpApiClient } from "@helia/delegated-routing-v1-http-api-client";
import { Multiaddr } from "@multiformats/multiaddr";
import { identify } from "@libp2p/identify";
import { pubsubPeerDiscovery } from "@libp2p/pubsub-peer-discovery";
import { sha256 } from "multiformats/hashes/sha2";
import { type Peer } from "common/types/PeerPool";

const baseTopic = "kukuri-chat/";

export default class Libp2pConnection {
  node: Libp2p | undefined;
  gossipsubOpts: object;
  initialized: boolean;
  started: boolean;
  subscribes: string[];
  constructor() {
    this.node = undefined;
    this.initialized = false;
    this.started = false;
    this.subscribes = [PUBSUB_PEER_DISCOVERY];
    this.gossipsubOpts = {
      allowPublishToZeroTopicPeers: true,
      ignoreDuplicatePublishError: true,
      msgIdFn: msgIdFnStrictNoSign,
    };
  }

  async init(peers: Peer[]) {
    this.initialized = true;

    const delegatedClient = createDelegatedRoutingV1HttpApiClient(
      "https://delegated-ipfs.dev",
    );

    console.log(`start with ${peers.length} bootstrapAddrs.`);

    this.node = await createLibp2p({
      addresses: {
        listen: ["/webrtc"],
      },
      transports: [
        webTransport(),
        webSockets(),
        webRTC({
          rtcConfiguration: {
            iceServers: [
              {
                // STUN servers help the browser discover its own public IPs
                urls: [
                  "stun:stun.l.google.com:19302",
                  "stun:global.stun.twilio.com:3478",
                ],
              },
            ],
          },
        }),
        webRTCDirect(),
        circuitRelayTransport({
          discoverRelays: 0,
        }),
      ],
      peerDiscovery: [
        pubsubPeerDiscovery({
          interval: 3_000,
          topics: this.subscribes,
          listenOnly: false,
        }),
        bootstrap({
          list: peers.map((peer) => peer.maddr),
        }),
      ],
      connectionEncryption: [noise()],
      streamMuxers: [yamux()],
      connectionGater: {
        denyDialMultiaddr: async () => false,
      },
      services: {
        pubsub: gossipsub(this.gossipsubOpts),
        delegatedClient: () => delegatedClient,
        identify: identify(),
      },
      connectionManager: {
        maxConnections: 30,
        minConnections: 5,
      },
    });
    this.node.addEventListener("peer:connect", (evt) => {
      console.log(`Connection established to: ${evt.detail.toString()}`);
    });
    this.node.addEventListener("peer:discovery", (evt) => {
      console.log(`Peer is discovered: ${evt.detail.id.toString()}`);
    });

    const pubsub = this.node.services.pubsub as PubSub;
    pubsub.addEventListener("message", (message) => {
      console.log(
        `${message.detail.topic}: ${new TextDecoder().decode(message.detail.data)}`,
      );
    });
    console.log("node created");
    this.node.start();
    this.started = true;
    console.log("libp2p started");
  }

  subscribe(topic: string) {
    if (this.node == undefined) {
      console.log("node is not created");
      return;
    }

    const pubsub = this.node.services.pubsub as PubSub;
    if (!pubsub) {
      console.log("pubsub is not defined");
      return;
    }

    const subTopic = baseTopic + topic;
    pubsub.subscribe(subTopic);
    this.subscribes.concat([subTopic]);
    console.log(`topic: ${topic} is subscribed`);
  }

  async send(topic: string, msg: string) {
    if (this.node == undefined) {
      console.log("node is not created");
      return;
    }

    const pubsub = this.node.services.pubsub as PubSub;
    if (!pubsub) {
      console.log("pubsub is not defined");
      return;
    }

    pubsub.publish(baseTopic + topic, new TextEncoder().encode(msg));
    console.log("message send");
  }

  status() {
    if (this.node == undefined) {
      console.log("node is not created");
      return;
    }

    const peers = this.node.getPeers();
    console.log("peers!");
    console.log(peers);
    const connections = this.node.getConnections();
    console.log("connections!");
    console.log(connections);
  }

  async dial(multiaddr: Multiaddr) {
    if (this.node == undefined) {
      console.log("node is not created");
      return;
    }

    console.log(`dialling: %a`, multiaddr);
    try {
      const conn = await this.node.dial(multiaddr);
      console.log("connected to %p on %a", conn.remotePeer, conn.remoteAddr);
      return conn;
    } catch (e) {
      console.error(e);
      throw e;
    }
  }
}

// message IDs are used to dedupe inbound messages
// every agent in network should use the same message id function
// messages could be perceived as duplicate if this isnt added (as opposed to rust peer which has unique message ids)
export async function msgIdFnStrictNoSign(msg: Message): Promise<Uint8Array> {
  const enc = new TextEncoder();

  const signedMessage = msg as SignedMessage;
  const encodedSeqNum = enc.encode(signedMessage.sequenceNumber.toString());
  return await sha256.encode(encodedSeqNum);
}
