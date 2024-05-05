import { useEffect, useState } from "react";
import { AppId } from "./Params";
import { DataPayload, Room, joinRoom } from "trystero";
import { v4 as uuidv4 } from "uuid";
import { Button } from "@/components/ui/button";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { RoomSchema, Room as RoomType } from "common/types/Connection";
import { cn } from "@/lib/utils";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";

function App() {
  const [room, setRoom] = useState<Room | null>(null);
  const [messages, setMessages] = useState<string[]>([]);
  const [peers, setPeers] = useState<string[]>([]);

  const RoomForm = useForm<RoomType>({
    resolver: zodResolver(RoomSchema),
    defaultValues: {
      id: "",
      name: "",
    },
  });

  useEffect(() => {
    if (room == null) return;

    const [sendMsg, getMsg] = room.makeAction("message");
    room.onPeerJoin((peer: string) => {
      setPeers(peers.concat([peer]));
      sendMsg("Hello!", peer);
    });
    getMsg((msg: DataPayload, peer: string) => {
      setMessages([...messages, `"${msg}" from ${peer}`]);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room]);

  const createRoom = () => {
    RoomForm.setValue("id", uuidv4());
  };

  const leaveRoom = () => {
    if (!room) return;
    console.log(room);
    room.leave();
    setRoom(null);
  };

  function onSubmitRoomForm(roomBody: RoomType) {
    console.log("join room!");
    const room = joinRoom({ appId: AppId }, roomBody.id);
    setRoom(room);
  }

  return (
    <>
      <h1>Kukuri Simple WebApp</h1>
      <div className="card">
        <Form {...RoomForm}>
          <form onSubmit={RoomForm.handleSubmit(onSubmitRoomForm)}>
            <div
              className={cn(
                "flex flex-row h-fit justify-center items-center my-2 space-x-3",
              )}
            >
              <FormField
                control={RoomForm.control}
                name="id"
                render={({ field }) => (
                  <FormItem>
                    <FormControl>
                      <Input {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <Button
                type="button"
                variant="default"
                onClick={() => createRoom()}
              >
                Create Room
              </Button>
            </div>
            <div
              className={cn(
                "flex flex-row h-fit justify-center items-center my-2 space-x-3",
              )}
            >
              <Button type="submit" variant="default" disabled={!!room}>
                Join Room
              </Button>
              <Button
                type="button"
                variant="secondary"
                onClick={() => leaveRoom()}
                disabled={!room}
              >
                Leave Room
              </Button>
            </div>
          </form>
        </Form>
      </div>
      <div>
        {peers.map((peer, idx) => (
          <p key={`peer-${idx}`}>{peer}</p>
        ))}
      </div>
      <div>
        {messages.map((msg, idx) => (
          <p key={`message-${idx}`}>{msg}</p>
        ))}
      </div>
    </>
  );
}

export default App;
