/** React 桌面界面入口；样式按固定顺序集中加载，StrictMode 用于暴露副作用清理问题。 */
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";
ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
