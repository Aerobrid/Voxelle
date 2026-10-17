import React from "react";
import ReactDOM from "react-dom/client";
import { Overlay } from "./Overlay";
import "@fontsource-variable/inter";
import "./overlay.css";

const root = document.getElementById("overlay-root");
if (!root) throw new Error("#overlay-root not found");
ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <Overlay />
  </React.StrictMode>,
);
