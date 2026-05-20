const { contextBridge } = require("electron");

const daemonPort = process.env.VETTY_DAEMON_PORT || "9876";
const restUrl =
  process.env.VETTY_DAEMON_REST_URL || `http://127.0.0.1:${daemonPort}/api`;
const wsUrl =
  process.env.VETTY_DAEMON_WS_URL || `ws://127.0.0.1:${daemonPort}/ws/events`;

contextBridge.exposeInMainWorld("vettyConfig", {
  daemon: {
    restUrl,
    wsUrl,
  },
});
