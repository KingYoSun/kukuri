// ref: https://github.com/libp2p/universal-connectivity/blob/main/js-peer/src/lib/libp2p.ts

import { kadDHT, removePrivateAddressesMapper } from "@libp2p/kad-dht";
import { createLibp2p, type Libp2p } from "libp2p";
import { bootstrap } from "@libp2p/bootstrap";
import { yamux } from "@chainsafe/libp2p-yamux";
import { noise } from "@chainsafe/libp2p-noise";
import { gossipsub } from "@chainsafe/libp2p-gossipsub";
import { BOOTSTRAP_PEER_IDS, PUBSUB_PEER_DISCOVERY } from "@/lib/constraints";
import type { PubSub, PeerId, Message, SignedMessage } from "@libp2p/interface";
import { webRTC, webRTCDirect } from "@libp2p/webrtc";
import { webSockets } from "@libp2p/websockets";
import { webTransport } from "@libp2p/webtransport";
import { circuitRelayTransport } from "@libp2p/circuit-relay-v2";
import {
  createDelegatedRoutingV1HttpApiClient,
  DelegatedRoutingV1HttpApiClient,
} from "@helia/delegated-routing-v1-http-api-client";
import { Multiaddr } from "@multiformats/multiaddr";
import first from "it-first";
import { peerIdFromString } from "@libp2p/peer-id";
import { identify } from "@libp2p/identify";
import { pubsubPeerDiscovery } from "@libp2p/pubsub-peer-discovery";
import { sha256 } from "multiformats/hashes/sha2";

const baseTopic = "/kukuri/main/";

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

  async init() {
    this.initialized = true;

    const delegatedClient = createDelegatedRoutingV1HttpApiClient(
      "https://delegated-ipfs.dev",
    );
    const { bootstrapAddrs, relayListenAddrs } =
      await getBootstrapMultiaddrs(delegatedClient);

    console.log(
      `start with ${bootstrapAddrs.length} bootstrapAddrs and ${relayListenAddrs.length} relayListenAddrs.`,
    );

    this.node = await createLibp2p({
      addresses: {
        listen: ["/webrtc", ...relayListenAddrs],
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
          discoverRelays: 1,
        }),
      ],
      peerDiscovery: [
        pubsubPeerDiscovery({
          interval: 3_000,
          topics: this.subscribes,
          listenOnly: false,
        }),
        bootstrap({
          list: bootstrapAddrs,
        }),
      ],
      connectionEncryption: [noise()],
      streamMuxers: [yamux()],
      connectionGater: {
        denyDialMultiaddr: async () => false,
      },
      services: {
        aminoDHT: kadDHT({
          protocol: "/ipfs/kad/1.0.0",
          peerInfoMapper: removePrivateAddressesMapper,
        }),
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

// Function which resolves PeerIDs of rust/go bootstrap nodes to multiaddrs dialable from the browser
// Returns both the dialable multiaddrs in addition to the relay
async function getBootstrapMultiaddrs(
  client: DelegatedRoutingV1HttpApiClient,
): Promise<BootstrapsMultiaddrs> {
  const peers = await Promise.all(
    BOOTSTRAP_PEER_IDS.map((peerId) =>
      first(client.getPeers(peerIdFromString(peerId))),
    ),
  );

  const bootstrapAddrs = [];
  const relayListenAddrs = [];
  for (const p of peers) {
    if (p && p.Addrs.length > 0) {
      for (const maddr of p.Addrs) {
        const protos = maddr.protoNames();
        if (
          (protos.includes("webtransport") ||
            protos.includes("webrtc-direct")) &&
          protos.includes("certhash")
        ) {
          if (maddr.nodeAddress().address === "127.0.0.1") continue; // skip loopback
          bootstrapAddrs.push(maddr.toString());
          relayListenAddrs.push(getRelayListenAddr(maddr, p.ID));
        }
      }
    }
  }
  return { bootstrapAddrs, relayListenAddrs };
}

interface BootstrapsMultiaddrs {
  // Multiaddrs that are dialable from the browser
  bootstrapAddrs: string[];

  // multiaddr string representing the circuit relay v2 listen addr
  relayListenAddrs: string[];
}

// Constructs a multiaddr string representing the circuit relay v2 listen address for a relayed connection to the given peer.
const getRelayListenAddr = (maddr: Multiaddr, peer: PeerId): string =>
  `${maddr.toString()}/p2p/${peer.toString()}/p2p-circuit`;
