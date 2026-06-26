const { notStrictEqual } = require("assert/strict");
const net = require("net");
const pigpio = require("pigpio-client");

const PORT = 8000;
const ESC_PIN = 4;

const MIN = 1000;
const MAX = 2400;
const STOP = 0;

let esc;
let armed = false;

const pi = pigpio.pigpio({ host: "localhost", port: 8888 });

function safeWrite(socket, message) {
  if (socket && !socket.destroyed && socket.writable) {
    socket.write(message);
  }
}

pi.on("connected", () => {
  console.log("Connected to pigpiod");
  esc = pi.gpio(ESC_PIN);
  esc.setServoPulsewidth(STOP);
  console.log("ESC ready");
});

pi.on("error", (err) => {
  console.error("pigpio error:", err.message);
});

function setEsc(value) {
  if (!esc) {
    console.log("ESC not ready yet");
    return;
  }
  const pulse = Math.max(STOP, Math.min(MAX, value));
  esc.setServoPulsewidth(pulse);
  console.log("ESC set to", pulse);
}

function armEsc(socket) {
  if (!esc) {
    safeWrite(socket, "ESC not ready yet\n");
    return;
  }
  console.log("Arming ESC...");
  setEsc(MIN);
  setTimeout(() => {
    armed = true;
    console.log("ESC armed");
    safeWrite(socket, "ESC armed\n");
  }, 3000);
}

function start(socket) {
  if (!esc) {
    safeWrite(socket, "ESC not ready yet\n");
    return;
  }

  if (!armed) {
    safeWrite(socket, "Send arm first\n");
    return;
  }

  const target = 1800;
  const duration = 2000;
  const steps = 10;
  const intervalTime = duration / steps;
  const stepSize = (target - MIN) / steps;

  let currentStep = 0;

  safeWrite(socket, "Starting slow ramp...\n");

  const ramp = setInterval(() => {
    currentStep++;

    const pulse = Math.round(MIN + stepSize * currentStep);
    setEsc(pulse);

    if (currentStep >= steps) {
      clearInterval(ramp);
      setEsc(target);
      safeWrite(socket, "Ramp complete\n");
    }
  }, intervalTime);
}

function maxSpeed(socket) {
  if (!esc) {
    safeWrite(socket, "ESC not ready yet\n");
    return;
  }

  if (!armed) {
    safeWrite(socket, "Send arm first\n");
    return;
  }

  const target = MAX;
  const duration = 2000;
  const steps = 10;
  const intervalTime = duration / steps;
  const stepSize = (target - MIN) / steps;

  let currentStep = 0;

  safeWrite(socket, "Starting slow ramp...\n");

  const ramp = setInterval(() => {
    currentStep++;

    const pulse = Math.round(MIN + stepSize * currentStep);
    setEsc(pulse);

    if (currentStep >= steps) {
      clearInterval(ramp);
      setEsc(target);
      safeWrite(socket, "Ramp complete\n");
    }
  }, intervalTime);
}

const server = net.createServer((socket) => {
  console.log("Client connected");

  safeWrite(socket, "Commands: arm, start, max, throttle <1000-2400>, stop\n");

  socket.on("data", (data) => {
    const input = data.toString().trim();
    const parts = input.split(" ");

    const command = parts[0];
    const value = Number(parts[1]);

    if (command === "arm") {
      armEsc(socket);
    } else if (command === "start") {
      start(socket);
    } else if (command === "max") {
      maxSpeed(socket);
    } else if (command === "throttle") {
      if (!armed) {
        safeWrite(socket, "Send arm first\n");
        return;
      }

      setEsc(value);
      safeWrite(socket, "Throttle set\n");
    } else if (command === "stop") {
      setEsc(STOP);
    } else {
      safeWrite(socket, "Unknown command\n");
    }
  });

  socket.on("end", () => {
    console.log("Client ended connection");
    setEsc(STOP);
  });

  socket.on("close", () => {
    console.log("Client disconnected");
    setEsc(STOP);
  });

  socket.on("error", (err) => {
    console.log("Socket error:", err.message);
    setEsc(STOP);
  });
});

server.listen(PORT, "0.0.0.0", () => {
  console.log("Server running on port", PORT);
});
