import {
  createLightNode,
  type LightNode,
  waitForRemotePeer,
  Protocols,
  createEncoder,
  createDecoder,
  type Encoder,
  type Decoder,
  type IFilterSubscription,
  type DecodedMessage,
  type SendResult,
} from "@waku/sdk";
import { contentTopicToPubsubTopic } from "@waku/utils";
import protobuf from "protobufjs";

export default class Connection {
  node: LightNode | undefined;
  topic: string;
  encoder: Encoder;
  decoder: Decoder;
  Message: protobuf.Type;
  subscription: IFilterSubscription | undefined;
  started: boolean;
  initialized: boolean;

  constructor() {
    this.node = undefined;
    this.topic = "/kukuri/1/message/proto";
    const contentTopic = this.topic;
    this.encoder = createEncoder({ contentTopic });
    this.decoder = createDecoder(contentTopic);
    this.Message = new protobuf.Type("Message")
      .add(new protobuf.Field("timestamp", 1, "uint64"))
      .add(new protobuf.Field("sender", 2, "string"))
      .add(new protobuf.Field("message", 3, "string"));
    this.subscription = undefined;
    this.started = false;
    this.initialized = false;
  }

  async init(subCb: (wakuMessage: DecodedMessage) => void) {
    try {
      this.initialized = true;
      this.node = await createLightNode({
        defaultBootstrap: true,
        contentTopics: [this.topic],
      });
      console.log("node has been defined");
      await this.node.start();
      console.log("node has been started");
      await waitForRemotePeer(this.node, [
        Protocols.LightPush,
        Protocols.Filter,
      ]);
      console.log("node has reached peer");
      const pubsubTopic = contentTopicToPubsubTopic(this.topic);
      this.subscription =
        await this.node.filter.createSubscription(pubsubTopic);
      console.log("subscription has been created");
      await this.subscription.subscribe([this.decoder], subCb);
      console.log("node started!");
      this.started = true;
      return true;
    } catch (e) {
      console.error(e);
      return false;
    }
  }

  async stop() {
    if (!this.node || !this.subscription) {
      console.log("Node is not started");
      return;
    }

    await this.subscription.unsubscribeAll();
    await this.node.stop();
    this.started = false;
  }

  async send(message: string) {
    if (!this.node) {
      console.log("Node is not started");
      return;
    }

    const protoMessage = this.Message.create({
      timestamp: Date.now(),
      sender: "kingyosun",
      message,
    });
    const serializedMessage = this.Message.encode(protoMessage).finish();

    const result: SendResult = await this.node.lightPush.send(this.encoder, {
      payload: serializedMessage,
    });
    console.log(result);
  }
}
