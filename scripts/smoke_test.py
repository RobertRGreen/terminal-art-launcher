#!/usr/bin/env python3
"""Linux PTY integration test with isolated fake programs; never installs packages."""
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import tempfile
import termios
import time

BINARY = Path(__file__).resolve().parents[1] / "target/debug/art"


def drain(fd, duration=0.3):
    output = b""
    deadline = time.monotonic() + duration
    while time.monotonic() < deadline:
        ready, _, _ = select.select([fd], [], [], 0.03)
        if ready:
            try:
                output += os.read(fd, 65536)
            except OSError:
                break
    return output


with tempfile.TemporaryDirectory(prefix="art-smoke-") as tmp:
    root = Path(tmp)
    bindir = root / "bin"
    bindir.mkdir()
    log = root / "calls"
    pidfile = root / "effect.pid"

    def executable(name, body):
        path = bindir / name
        path.write_text("#!/bin/sh\n" + body)
        path.chmod(0o755)

    executable("figlet", 'printf "SMOKE_ART_OUTPUT\\n"\n')
    executable("cmatrix", f'echo $$ > "{pidfile}"\nexec /bin/sleep 60\n')
    executable("yay", f'echo install >> "{log}"\nexit 1\n')
    env = dict(os.environ, PATH=str(bindir), XDG_CONFIG_HOME=str(root / "config"), TERM="xterm-256color")
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 30, 110, 0, 0))
    before = termios.tcgetattr(slave)
    process = subprocess.Popen([str(BINARY), "--no-splash"], stdin=slave, stdout=slave, stderr=slave, env=env, start_new_session=True)
    output = drain(master)

    def send(keys, duration=0.3):
        global output
        os.write(master, keys)
        chunk = drain(master, duration)
        output += chunk
        return chunk

    try:
        assert b"terminal gallery" in output, output[-1000:]
        send(b"/figlet\rf")
        result = send(b"\r")
        assert b"SMOKE_ART_OUTPUT" in result
        send(b"\r")
        send(b"c\x1b[C\x1b[B\x1b[C")
        send(b"\x1b")
        send(b"/\x1b")  # clear accepted filter
        send(b"m")
        assert pidfile.exists(), "Matrix effect did not start"
        effect_pid = int(pidfile.read_text())
        send(b"\x1b")
        try:
            os.kill(effect_pid, 0)
        except ProcessLookupError:
            pass
        else:
            raise AssertionError("Effect leaked after Esc")
        send(b"/cbonsai\ri")
        assert log.read_text().count("install") == 1
        send(b"i")
        assert log.read_text().count("install") == 1, "Failed install was not suppressed"
        send(b"ui")
        assert log.read_text().count("install") == 2
        assert b"Workspace presets" in send(b"p")
        send(b"nNight")
        send(b"\x1b")  # cancel name entry
        send(b"\x1b")  # close presets
        send(b"\x1b")
        assert process.wait(timeout=3) == 0
        assert termios.tcgetattr(slave) == before, "Terminal settings not restored"
        config = (root / "config/terminal-art-launcher/config.toml").read_text()
        assert '"figlet"' in config and "theme = 1" in config and "speed = 2" in config, config
        print("PASS: search, launch, favorites, config persistence, matrix stop/cleanup, failed install/retry, terminal restoration")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        os.close(master)
        os.close(slave)
