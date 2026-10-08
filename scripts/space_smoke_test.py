#!/usr/bin/env python3
"""Exercise installed Space effects under a real controlling PTY (Linux only)."""
import fcntl
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import struct
import tempfile
import termios
import time

BINARY = Path(__file__).resolve().parents[1] / "target/debug/art"
PROGRAMS = {name: shutil.which(name) for name in ("astroterm", "globe", "starfetch")}
assert all(PROGRAMS.values()), "Install astroterm, globe, and starfetch before this optional test"


def drain(fd, duration):
    deadline = time.monotonic() + duration
    output = b""
    while time.monotonic() < deadline:
        if select.select([fd], [], [], 0.04)[0]:
            try:
                output += os.read(fd, 65536)
            except OSError:
                break
    return output


with tempfile.TemporaryDirectory(prefix="art-space-test-") as temp:
    root = Path(temp)
    bins = root / "bin"
    bins.mkdir()
    for name, path in PROGRAMS.items():
        (bins / name).symlink_to(path)
    pid, fd = pty.fork()  # sets controlling terminal and foreground process group
    if pid == 0:
        os.environ.update(PATH=str(bins), XDG_CONFIG_HOME=str(root / "config"), TERM="xterm-256color")
        os.execv(str(BINARY), [str(BINARY), "--no-splash"])
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", 35, 120, 0, 0))

    def send(keys, duration=0.4):
        os.write(fd, keys)
        return drain(fd, duration)

    try:
        drain(fd, 0.3)
        send(b"/astroterm\r")
        assert len(send(b"\r", 0.8)) > 1000, "Astroterm did not draw"
        assert b"gallery" in send(b"q", 0.5), "Astroterm did not return"
        send(b"/\x1b")
        send(b"/globe\r")
        assert len(send(b"\r", 0.8)) > 1000, "Globe did not draw"
        output = send(b"\x03", 0.5)
        if b"gallery" not in output:
            output += send(b"\r", 0.5)
        assert b"gallery" in output, "Globe did not return"
        send(b"/\x1b")
        output = send(b"a", 0.8)
        assert b"astroterm" in output and len(output) > 1000, "Background astroterm did not render"
        output = send(b" ", 0.8)
        assert b"globe" in output and len(output) > 1000, "Background globe did not render"
        assert b"gallery" in send(b"\x1b", 0.6), "Playback did not stop"
        send(b"\x1b", 0.2)
        for _ in range(30):
            finished, status = os.waitpid(pid, os.WNOHANG)
            if finished:
                pid = None
                assert status == 0, status
                break
            time.sleep(0.1)
        assert pid is None, "Launcher did not exit"
        print("PASS: real Space rendering, normal launch/return, background playback, skip, Esc stop")
    finally:
        if pid:
            os.kill(pid, signal.SIGTERM)
            drain(fd, 1)
            finished, _ = os.waitpid(pid, os.WNOHANG)
            if not finished:
                os.killpg(pid, signal.SIGKILL)
                os.waitpid(pid, 0)
        os.close(fd)
