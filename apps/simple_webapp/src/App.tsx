import { useEffect, useState, useContext, useReducer } from "react";
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
import { DecodedMessage } from "@waku/sdk";

type Message = {
  timestamp: string;
  sender: string;
  message: string;
};

interface MsgAction {
  type: "add" | "reset";
  payload?: Message;
}

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
  // const dbContext = useContext(DbContext);
  const connContext = useContext(ConnContext);

  function receiver(wakuMessage: DecodedMessage) {
    console.log("receive message!");
    if (!wakuMessage.payload) return;
    const messageObj = connContext.conn?.Message.decode(
      wakuMessage.payload,
    ).toJSON() as Message;
    console.log(messageObj);
    dispatchMessages({
      type: "add",
      payload: messageObj,
    });
  }

  useEffect(() => {
    (async () => {
      if (!connContext.conn) {
        console.log("conn is null");
        return;
      }
      if (connContext.conn?.initialized) {
        console.log("conn has been initialized");
        return;
      }
      if (connContext.conn?.started) {
        console.log("conn has been started");
        return;
      }
      console.log("init conn");
      const initialized = await connContext.conn.init(receiver);
      setStarted(initialized);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

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

    connContext.conn.send(body.msg);
    console.log("send message!");
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
            </div>
          </form>
        </Form>
      </div>
      <div className={cn("my-2")}>
        {messages.map((message, idx) => {
          return (
            <p key={idx}>
              {message.sender}: {message.message} at{" "}
              {new Date(Number(message.timestamp)).toLocaleTimeString()}
            </p>
          );
        })}
      </div>
    </>
  );
}

export default App;
