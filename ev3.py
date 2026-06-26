#!/usr/bin/env python3

import socket

from ev3dev2.motor import (
    OUTPUT_A,
    OUTPUT_B,
    OUTPUT_C,
    LargeMotor,
    MediumMotor,
    SpeedPercent,
)

left_motor = LargeMotor(OUTPUT_B)
right_motor = LargeMotor(OUTPUT_C)
belt_motor = MediumMotor(OUTPUT_A)

speed = 50
factor = 7

HOST = "0.0.0.0"
PORT = 5000


def forward():
    left_motor.on(SpeedPercent(-speed))
    right_motor.on(SpeedPercent(-speed))


def backward():
    left_motor.on(SpeedPercent(speed))
    right_motor.on(SpeedPercent(speed))


def left():
    left_motor.on(SpeedPercent(-speed / factor))
    right_motor.on(SpeedPercent(speed / factor))


def right():
    left_motor.on(SpeedPercent(speed / factor))
    right_motor.on(SpeedPercent(-speed / factor))


def stop():
    left_motor.off()
    right_motor.off()
    belt_motor.off()


def pick():
    belt_motor.on(SpeedPercent(40))


def close():
    belt_motor.on(SpeedPercent(-40))


def safe_send(client_sock, message):
    try:
        client_sock.sendall(message.encode("utf-8"))
    except (BrokenPipeError, ConnectionResetError, OSError):
        pass


def handle_client(client_sock, address):
    print("Connected to:", address)

    buffer = ""

    try:
        safe_send(
            client_sock,
            "Commands: forward, backward, left, right, pick, close, stop, exit\n",
        )

        while True:
            data = client_sock.recv(1024)

            if not data:
                print("Client disconnected:", address)
                break

            buffer += data.decode("utf-8", errors="ignore")

            while "\n" in buffer:
                command, buffer = buffer.split("\n", 1)
                command = command.strip().lower()

                if not command:
                    continue

                print("Received:", command)

                if command == "forward":
                    forward()
                    safe_send(client_sock, "OK forward\n")

                elif command == "backward":
                    backward()
                    safe_send(client_sock, "OK backward\n")

                elif command == "left":
                    left()
                    safe_send(client_sock, "OK left\n")

                elif command == "right":
                    right()
                    safe_send(client_sock, "OK right\n")

                elif command == "pick":
                    pick()
                    safe_send(client_sock, "OK pick\n")

                elif command == "close":
                    close()
                    safe_send(client_sock, "OK close\n")

                elif command == "stop":
                    stop()
                    safe_send(client_sock, "OK stop\n")

                elif command == "exit":
                    stop()
                    safe_send(client_sock, "BYE\n")
                    print("Client requested exit:", address)
                    return

                else:
                    stop()
                    safe_send(client_sock, "ERR unknown command\n")

    except (ConnectionResetError, BrokenPipeError, OSError) as err:
        print("Connection error:", err)

    finally:
        stop()
        try:
            client_sock.close()
        except OSError:
            pass

        print("Motors stopped. Ready for next client.")


server_sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
server_sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
server_sock.bind((HOST, PORT))
server_sock.listen(1)

print("Server running on port", PORT)
print("Waiting for connection...")

try:
    while True:
        client_sock, address = server_sock.accept()
        handle_client(client_sock, address)
        print("Waiting for new connection...")

except KeyboardInterrupt:
    print("\nShutting down server...")

finally:
    stop()
    server_sock.close()
    print("Server closed safely.")
