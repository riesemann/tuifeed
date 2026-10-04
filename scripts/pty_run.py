#!/usr/bin/env python3
"""Run tuifeed in a pty of a known size and print what it draws.

`script` leaves the pty at 0x0, so ratatui draws nothing. This harness sets a
window size with TIOCSWINSZ, sends a key sequence after a delay, then quits with
"q", and writes the terminal stream to stdout so a run can be checked without a
screen.

Usage: pty_run.py <XDG_CONFIG_HOME> <program> [wait_seconds] [keys]

`keys` are literal bytes sent after `wait_seconds` (e.g. an arrow-down then
Enter is "\\x1b[B\\r").
"""
import fcntl
import os
import pty
import select
import struct
import subprocess
import sys
import termios
import time

cfg_home = sys.argv[1]
program = sys.argv[2]
wait_s = float(sys.argv[3]) if len(sys.argv) > 3 else 10.0
keys = sys.argv[4] if len(sys.argv) > 4 else ""
cols, rows = 100, 30

master, slave = pty.openpty()
fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", rows, cols, 0, 0))
env = dict(os.environ)
env["XDG_CONFIG_HOME"] = cfg_home

proc = subprocess.Popen(
    [program], stdin=slave, stdout=slave, stderr=slave, env=env, close_fds=True
)
os.close(slave)

out = bytearray()
start = time.time()
keys_sent = False
quit_sent = False
while proc.poll() is None:
    ready, _, _ = select.select([master], [], [], 0.2)
    if ready:
        try:
            data = os.read(master, 65536)
        except OSError:
            break
        if not data:
            break
        out += data
    elapsed = time.time() - start
    if not keys_sent and elapsed >= wait_s:
        if keys:
            os.write(master, keys.encode())
        keys_sent = True
    if keys_sent and not quit_sent and elapsed >= wait_s + 1.0:
        os.write(master, b"q")
        quit_sent = True

while True:
    ready, _, _ = select.select([master], [], [], 0.1)
    if not ready:
        break
    try:
        data = os.read(master, 65536)
    except OSError:
        break
    if not data:
        break
    out += data

proc.wait()
sys.stdout.buffer.write(bytes(out))
