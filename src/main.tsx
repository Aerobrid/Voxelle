import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
// Fonts are bundled (SIL OFL 1.1) and served from the app itself: nothing is fetched.
import "@fontsource-variable/inter";
import "@fontsource-variable/jetbrains-mono";
import "./styles/theme.css";

const root = document.getElementById("root");
if (!root) throw new Error("#root not found");

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
