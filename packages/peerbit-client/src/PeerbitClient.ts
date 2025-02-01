import { Peerbit } from "peerbit";
import { noise } from "@chainsafe/libp2p-noise";
import { yamux } from "@chainsafe/libp2p-yamux";
import { Ed25519Keypair } from "@peerbit/crypto";
import { Multiaddr } from "@multiformats/multiaddr";
import { webRTC, webRTCDirect } from "@libp2p/webrtc";
import { webSockets } from "@libp2p/websockets";
import { webTransport } from "@libp2p/webtransport";
import * as filters from "@libp2p/websockets/filters";
import { circuitRelayTransport } from "@libp2p/circuit-relay-v2";
import { DirectSub } from "@peerbit/pubsub";
import { identify } from "@libp2p/identify";
import { createHost } from "@peerbit/proxy-window";

type NodeOptions = {
  type?: "node";
  network: "local" | "remote";
  waitForConnected?: boolean;
  keypair?: Ed25519Keypair;
  bootstrap?: (Multiaddr | string)[];
  host?: boolean;
  directory?: string;
};

export default class PeerbitClient {
  client: Peerbit;

  constructor() {
    this.client = undefined;
  }

  async init(nodeOptions: NodeOptions) {
    this.client = new Peerbit({
      libp2p: {
        addresses: {
          listen: ["p2p-circuit", "/webrtc"],
        },
        connectionEncryption: [noise()],
        connectionManager: {
          maxConnections: 100,
        },
        streamMuxers: [yamux()],
        services: {
          pubsub: (c) =>
            new DirectSub(c, {
              canRelayMessage: true,
            }),
          identify: identify(),
        },
        ...(nodeOptions.network === "local"
          ? {
              connectionGater: {
                denyDialMultiaddr: () => false,
              },
              transports: [
                webSockets({ filter: filters.all }),
                circuitRelayTransport(),
              ],
            }
          : {
              transports: [
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
                webSockets({ filter: filters.wss }),
                webTransport(),
                webRTCDirect(),
                circuitRelayTransport({ discoverRelays: 0 }),
              ],
            }),
      },
      ...(nodeOptions.directory ? { directory: nodeOptions.directory } : {}),
    });
    this.client.libp2p.node.addEventListener("peer:connect", (evt) => {
      console.log(`Connection established to: ${evt.detail.toString()}`);
    });
    this.client.libp2p.node.addEventListener("peer:discovery", (evt) => {
      console.log(`Peer is discovered: ${evt.detail.id.toString()}`);
    });
    const connectFn = async () => {
      try {
        if (nodeOptions.network === "local") {
          await this.client.dial(
            "/ip4/127.0.0.1/tcp/8002/ws/p2p/" +
              (await (await fetch("http://localhost:8002/peers")).text()),
          );
        } else {
          if (nodeOptions.bootstrap) {
            for (const addr of nodeOptions.bootstrap) {
              await this.client.dial(addr);
            }
          } else {
            await this.client["bootstrap"]?.();
          }
        }
      } catch (err) {
        console.error("Failed to resolve relay addresses. " + err?.message);
      }

      if (nodeOptions.host) {
        this.client = await createHost(this.client);
      }
    };

    console.log("Bootstrap start...");
    const promise = connectFn();
    promise.then(() => {
      console.log("Bootstrap done.");
    });

    if (nodeOptions.waitForConnected) {
      await promise;
    }
  }
}
