import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App.tsx";
import "./index.css";
import DbProvider from "./context/db.tsx";
import ConnProvider from "./context/conn.tsx";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <ConnProvider>
      <DbProvider>
        <App />
      </DbProvider>
    </ConnProvider>
  </React.StrictMode>,
);
