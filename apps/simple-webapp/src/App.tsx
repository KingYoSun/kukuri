import {
  useEffect,
  useState,
  useContext,
  useReducer,
  useCallback,
} from "react";
import { Button } from "@/components/ui/button";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { cn } from "@/lib/utils";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
// import { DbContext } from "./context/db";
import { ConnContext } from "./context/conn";
import { z } from "zod";
import { multiaddr } from "@multiformats/multiaddr";
import { DEFAULT_PEER_POOL_URL } from "./lib/constraints";
import { type Peer } from "common/types/PeerPool";

type Message = {
  timestamp: number;
  sender: string;
  message: string;
};

interface MsgAction {
  type: "add" | "reset";
  payload?: Message;
}

const topic = "main";

function App() {
  function MsgReducer(state: Message[], action: MsgAction): Message[] {
    switch (action?.type) {
      case "add":
        if (!action?.payload) return state;
        return [...state, action.payload];
      case "reset":
        return [];
    }
  }

  const [messages, dispatchMessages] = useReducer(MsgReducer, []);
  const [started, setStarted] = useState<boolean>(false);
  const [peerCount, setPeerCount] = useState<number>(0);
  // const dbContext = useContext(DbContext);
  const connContext = useContext(ConnContext);

  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  function receiver(msgStr: string) {
    console.log("receive message!");
    if (!msgStr) return;
    const messageObj = JSON.parse(msgStr) as Message;
    console.log(messageObj);
    dispatchMessages({
      type: "add",
      payload: messageObj,
    });
  }

  const getPeerCount = useCallback(async () => {
    const query = new URLSearchParams({ topic: `kukuri-chat/${topic}` });
    const res = await fetch(`${DEFAULT_PEER_POOL_URL}/peers/count?${query}`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
      mode: "cors",
      credentials: "omit",
    });
    const data = await res.json();
    if (data) setPeerCount(data[0].count);
    return data[0].count;
  }, []);

  const getPeers = useCallback(async () => {
    if (peerCount == 0) {
      console.log("this topic has no peers");
      return;
    }

    const query = new URLSearchParams({ topic: `kukuri-chat/${topic}` });
    const res = await fetch(`${DEFAULT_PEER_POOL_URL}/peers?${query}`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
      mode: "cors",
      credentials: "omit",
    });
    return (await res.json()) as Peer[];
  }, [peerCount]);

  const reloadPeers = useCallback(async () => {
    if (
      !connContext.conn ||
      !connContext.conn.initialized ||
      !connContext.conn.node
    ) {
      console.log("conn has not been initialized");
      return;
    }

    const newCount = await getPeerCount();
    if (newCount == 0) {
      console.log("this topic has no peers");
      return;
    }

    const oldPeerIds = connContext.conn.node.getPeers();
    const oldPeers = oldPeerIds.map((peerId) => peerId.toString());
    const peers = await getPeers();
    if (!peers || peers.length == 0) {
      console.log("this topic has no peers");
      return;
    }

    const newPeers = peers.filter((peer) => {
      const matches = oldPeers.map((oldPeer) => {
        peer.maddr.endsWith(oldPeer);
      });
      return matches.length == 0;
    });
    if (newPeers.length > 0) {
      newPeers.map((newPeer) =>
        connContext.conn
          ? connContext.conn.dial(multiaddr(newPeer.maddr))
          : null,
      );
    }
  }, [connContext.conn, getPeerCount, getPeers]);

  useEffect(() => {
    if (!connContext.conn) {
      console.log("conn is null");
      return;
    }
    if (connContext.conn.initialized) {
      console.log("conn has been initialized");
      return;
    }
    getPeerCount();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    (async () => {
      if (peerCount == 0) {
        console.log(`${topic} topic has no peer`);
        return;
      }
      const peers = await getPeers();
      if (
        peers &&
        peers.length > 0 &&
        connContext.conn &&
        !connContext.conn.initialized
      ) {
        console.log("init conn");
        await connContext.conn.init(peers);
        await connContext.conn.subscribe(topic);
        setStarted(connContext.conn.initialized);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [peerCount, getPeers]);

  const msg = z.object({
    msg: z.string(),
  });

  const MsgForm = useForm({
    resolver: zodResolver(msg),
    defaultValues: {
      msg: "",
    },
  });

  async function onSubmitMsgForm(body: z.infer<typeof msg>) {
    if (!body.msg || body.msg.length === 0) {
      console.log("msg is empty");
      return;
    }
    if (!connContext.conn) {
      console.log("connection has not init yet");
      return;
    }

    const message: Message = {
      timestamp: new Date().getTime(),
      sender: "kingyosun",
      message: body.msg,
    };
    connContext.conn.send(topic, JSON.stringify(message));
    MsgForm.setValue("msg", "");
  }

  return (
    <>
      <h1>Kukuri Simple WebApp</h1>
      <div className="card">
        <Form {...MsgForm}>
          <form onSubmit={MsgForm.handleSubmit(onSubmitMsgForm)}>
            <div
              className={cn(
                "flex flex-row h-fit justify-center items-center my-2 space-x-3",
              )}
            >
              <FormField
                control={MsgForm.control}
                name="msg"
                render={({ field }) => (
                  <FormItem>
                    <FormControl>
                      <Input {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </div>
            <div
              className={cn(
                "flex flex-row h-fit justify-center items-center my-2 space-x-3",
              )}
            >
              <Button type="submit" variant="default" disabled={!started}>
                Send Message
              </Button>
              <Button
                type="button"
                variant="default"
                disabled={!started}
                onClick={() => connContext.conn?.status()}
              >
                Get Node Status
              </Button>
              <Button
                type="button"
                variant="default"
                disabled={!started}
                onClick={() => reloadPeers()}
              >
                Reload Peers
              </Button>
            </div>
          </form>
        </Form>
      </div>
      <div className={cn("my-2")}>
        {messages.map((message, idx) => {
          return (
            <p key={idx}>
              {message.sender}: {message.message} at{" "}
              {new Date(message.timestamp).toLocaleTimeString()}
            </p>
          );
        })}
      </div>
    </>
  );
}

export default App;
