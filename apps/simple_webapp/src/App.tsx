import { useEffect, useState } from "react";
import reactLogo from "./assets/react.svg";
import viteLogo from "/vite.svg";
import "./App.css";
import { AppId } from "./Params";
import { DataPayload, Room, joinRoom } from "trystero";
// eslint-disable-next-line @typescript-eslint/no-unused-vars
import { v4 as uuidv4 } from "uuid";

function App() {
  const [roomId, setRoomId] = useState("");
  const [room, setRoom] = useState<Room | null>(null);
  const [messages, setMessages] = useState<string[]>([]);

  useEffect(() => {
    setRoomId("36d483ef-fcf1-4750-abde-f6454f6c5281"); // setRoomId(uuidv4());
  }, []);

  useEffect(() => {
    if (room == null) return;

    const [sendMsg, getMsg] = room.makeAction("message");
    room.onPeerJoin((peer: string) => sendMsg("Hello!", peer));
    getMsg((msg: DataPayload, peer: string) => {
      setMessages([...messages, `"${msg}" from ${peer}`]);
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [room]);

  const initRoom = () => {
    const room = joinRoom({ appId: AppId }, roomId);
    setRoom(room);
  };

  return (
    <>
      <div>
        <a href="https://vitejs.dev" target="_blank">
          <img src={viteLogo} className="logo" alt="Vite logo" />
        </a>
        <a href="https://react.dev" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <h1>Vite + React</h1>
      <div className="card">
        <button onClick={() => initRoom()}>roomId is {roomId}</button>
        {messages.map((msg, idx) => (
          <p key={idx}>{msg}</p>
        ))}
      </div>
      <p className="read-the-docs">
        Click on the Vite and React logos to learn more
      </p>
    </>
  );
}

export default App;
