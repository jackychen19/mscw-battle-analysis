import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/nunito/600.css";
import "@fontsource/nunito/700.css";
import "@fontsource/nunito/800.css";
import "@fontsource/nunito/900.css";
import "./styles/tokens.css";
import "./styles/panel.css";
import "./styles/calibrate.css";
import App from "./App";
import { Calibrate } from "./calibrate/Calibrate";
import { inTauri, windowLabel } from "./api";

document.documentElement.classList.toggle("in-tauri", inTauri);

void windowLabel().then((label) => {
  const calibrate = label === "calibrate";
  document.documentElement.classList.toggle("calibrate", calibrate);
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>{calibrate ? <Calibrate /> : <App />}</React.StrictMode>,
  );
});
