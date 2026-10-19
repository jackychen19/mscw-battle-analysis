import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/nunito/700.css";
import "@fontsource/nunito/800.css";
import "@fontsource/nunito/900.css";
import "./styles/tokens.css";
import "./styles/panel.css";
import App from "./App";
import { inTauri } from "./api";

document.documentElement.classList.toggle("in-tauri", inTauri);

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
