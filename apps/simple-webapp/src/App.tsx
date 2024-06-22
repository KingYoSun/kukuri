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
  const [connected, setConnected] = useState<boolean>(false);
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

  async function getPeerCount() {
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
  }

  const getPeers = useCallback(async () => {
    if (peerCount == 0) {
      console.log("this topic has no peer");
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

  useEffect(() => {
    if (!connContext.conn) {
      console.log("conn is null");
      return;
    }
    if (connContext.conn?.initialized) {
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

  const maddr = z.object({
    maddr: z.string(),
  });

  const MaddrForm = useForm({
    resolver: zodResolver(maddr),
    defaultValues: {
      maddr: "",
    },
  });

  async function onSubmitMaddrForm(body: z.infer<typeof maddr>) {
    if (!body.maddr || body.maddr.length === 0) {
      console.log("maddr is empty");
      return;
    }
    if (!connContext.conn) {
      console.log("connection has not init yet");
      return;
    }

    await connContext.conn.dial(multiaddr(body.maddr));
    MaddrForm.setValue("maddr", "");
    setConnected(true);
  }

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
        <Form {...MaddrForm}>
          <form onSubmit={MaddrForm.handleSubmit(onSubmitMaddrForm)}>
            <div
              className={cn(
                "flex flex-row h-fit justify-center items-center my-2 space-x-3",
              )}
            >
              <FormField
                control={MaddrForm.control}
                name="maddr"
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
                Dial Peer
              </Button>
            </div>
          </form>
        </Form>
      </div>
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
              <Button type="submit" variant="default" disabled={!connected}>
                Send Message
              </Button>
              <Button
                type="button"
                variant="default"
                disabled={!connected}
                onClick={() => connContext.conn?.status()}
              >
                Get Node Status
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
