const { app, BrowserWindow, Menu } = require("electron");
const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");

let mainWindow = null;
let daemonProcess = null;
const daemonPort = process.env.VETTY_DAEMON_PORT || "9876";

async function startDaemonIfNeeded() {
  if (await isDaemonRunning()) {
    process.stdout.write(`[vetty-daemon] already running on 127.0.0.1:${daemonPort}\n`);
    return;
  }

  const daemonCommand = resolveDaemonCommand();
  daemonProcess = spawn(daemonCommand.command, daemonCommand.args, {
    env: {
      ...process.env,
      VETTY_DAEMON_PORT: daemonPort,
    },
    stdio: "pipe",
  });

  daemonProcess.stdout?.on("data", (chunk) => {
    process.stdout.write(`[vetty-daemon] ${chunk}`);
  });
  daemonProcess.stderr?.on("data", (chunk) => {
    process.stderr.write(`[vetty-daemon] ${chunk}`);
  });
  daemonProcess.on("error", (error) => {
    process.stderr.write(`[vetty-daemon] failed to start: ${error.message}\n`);
  });
  daemonProcess.on("exit", (code, signal) => {
    process.stdout.write(
      `[vetty-daemon] exited (code=${code ?? "null"}, signal=${signal ?? "null"})\n`,
    );
  });
}

async function isDaemonRunning() {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 500);

  try {
    const response = await fetch(`http://127.0.0.1:${daemonPort}/api/sandboxes`, {
      signal: controller.signal,
    });
    return response.ok;
  } catch {
    return false;
  } finally {
    clearTimeout(timeout);
  }
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1400,
    height: 900,
    minWidth: 1024,
    minHeight: 640,
    backgroundColor: "#0a0a0f",
    webPreferences: {
      contextIsolation: true,
      nodeIntegration: false,
      preload: path.join(__dirname, "preload.cjs"),
    },
  });

  if (process.env.NODE_ENV === "development") {
    mainWindow.loadURL("http://localhost:5173");
  } else {
    mainWindow.loadFile(path.join(__dirname, "..", "dist", "index.html"));
  }
}

app.whenReady().then(() => {
  Menu.setApplicationMenu(null);
  startDaemonIfNeeded().then(createWindow);
});

app.on("window-all-closed", () => {
  if (daemonProcess && daemonProcess.pid) {
    daemonProcess.kill("SIGTERM");
  }
  app.quit();
});

function resolveDaemonCommand() {
  if (process.env.VETTY_DAEMON_BIN) {
    return {
      command: process.env.VETTY_DAEMON_BIN,
      args: [],
    };
  }

  const executable = process.platform === "win32" ? "vetty-daemon.exe" : "vetty-daemon";
  const localDebugBin = path.join(__dirname, "..", "..", "target", "debug", executable);
  if (fs.existsSync(localDebugBin)) {
    return {
      command: localDebugBin,
      args: [],
    };
  }

  const localReleaseBin = path.join(__dirname, "..", "..", "target", "release", executable);
  if (fs.existsSync(localReleaseBin)) {
    return {
      command: localReleaseBin,
      args: [],
    };
  }

  if (process.env.NODE_ENV === "development") {
    return {
      command: "cargo",
      args: ["run", "-p", "vetty-daemon"],
    };
  }

  return {
    command: "vetty-daemon",
    args: [],
  };
}
