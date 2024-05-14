import { type ReactNode, useReducer, createContext, Dispatch } from "react";
import Connection from "@/class/connection";

interface Action {
  type: "reset";
  payload?: Connection;
}

export const ConnContext = createContext(
  {} as {
    conn: Connection | null;
    dispatchConn: Dispatch<Action>;
  },
);

interface Props {
  children: ReactNode;
}

function reducer(state: Connection | null, action: Action): Connection | null {
  switch (action?.type) {
    case "reset":
      return new Connection();
    default:
      return state;
  }
}

export default function ConnProvider({ children }: Props) {
  const initConn = new Connection();
  const [conn, dispatchConn] = useReducer(reducer, initConn);

  return (
    <ConnContext.Provider value={{ conn, dispatchConn }}>
      {children}
    </ConnContext.Provider>
  );
}
