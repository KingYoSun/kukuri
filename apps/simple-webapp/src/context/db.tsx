import { type ReactNode, useReducer, createContext, Dispatch } from "react";

interface Action {
  type: "set" | "reset";
  payload?: null;
}

export const DbContext = createContext(
  {} as {
    db: null;
    dispatchDb: Dispatch<Action>;
  },
);

interface Props {
  children: ReactNode;
}

function reducer(state: null, action: Action): null {
  switch (action?.type) {
    case "set":
      if (action?.payload == undefined) return state;
      return action?.payload;
    case "reset":
      return null;
    default:
      return state;
  }
}

export default function DbProvider({ children }: Props) {
  const [db, dispatchDb] = useReducer(reducer, null);

  return (
    <DbContext.Provider value={{ db, dispatchDb }}>
      {children}
    </DbContext.Provider>
  );
}
