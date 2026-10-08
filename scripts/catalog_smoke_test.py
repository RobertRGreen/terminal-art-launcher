#!/usr/bin/env python3
"""Opt-in real-program checks in isolated PTYs; never changes desktop windows.

Uses installed art and candy programs, including network and audio services.
Oneko requires a separate Xvfb display and is reported skipped without it.
Output logs and JSON results are written to the specified directory.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import pty
import select
import shutil
import signal
import struct
import subprocess
import tempfile
import termios
import time

STATIC = {'fastfetch', 'figlet', 'toilet', 'cowsay', 'ponysay', 'fortune', 'lolcat', 'wttr.in', 'starfetch'}
NAMES = ['cbonsai', 'cmatrix', 'unimatrix', 'pipes.sh', 'asciiquarium', 'aafire', 'oneko', 'fastfetch', 'btop', 'htop', 'nvtop', 'figlet', 'toilet', 'cowsay', 'ponysay', 'fortune', 'lolcat', 'cava', 'wttr.in', 'tty-clock', 'astroterm', 'globe', 'starfetch', 'terrascope']


def drain(fd, seconds):
    data = b''
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if select.select([fd], [], [], 0.05)[0]:
            try:
                data += os.read(fd, 65536)
            except OSError:
                break
    return data


def check(name, binary, logs, display=None, framebuffer=None):
    executable = 'curl' if name == 'wttr.in' else name
    if not shutil.which(executable):
        return {'program': name, 'status': 'MISSING'}
    if name == 'oneko' and display is None:
        return check_oneko(binary, logs)
    with tempfile.TemporaryDirectory(prefix='art-catalog-') as tmp:
        pid, fd = pty.fork()
        if pid == 0:
            os.environ.update(TERM='xterm-256color', XDG_CONFIG_HOME=tmp)
            # aalib must render on this PTY, never open a live X11 window.
            os.environ.pop('DISPLAY', None)
            if display:
                os.environ['DISPLAY'] = display
            os.environ.pop('WAYLAND_DISPLAY', None)
            os.execv(binary, [binary, '--no-splash'])
        fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack('HHHH', 40, 140, 0, 0))
        output = b''
        try:
            drain(fd, .4)
            os.write(fd, ('/' + name + '\r').encode())
            drain(fd, .3)
            initial_frame = framebuffer.read_bytes() if framebuffer else None
            os.write(fd, b'\r')
            output = drain(fd, 17 if name == 'wttr.in' else 4)
            if name == 'terrascope':
                os.write(fd, b'\x1b')
                output += drain(fd, 5)
            if name in STATIC:
                assert b'Program finished (exit status: 0)' in output, 'No successful exit status'
                os.write(fd, b'\r')
            else:
                assert b'Program finished' not in output, 'Program exited during startup'
                if framebuffer:
                    assert framebuffer.read_bytes() != initial_frame, 'No drawing on isolated X display'
                else:
                    assert len(output) > 150, 'No substantial terminal output'
                os.write(fd, b'\x03' if name in {'globe', 'oneko'} else b'q')
            returned = drain(fd, 1)
            if b'gallery' not in returned:
                os.write(fd, b'\x03')
                returned += drain(fd, .5)
                os.write(fd, b'\r')
                returned += drain(fd, .5)
            output += returned
            assert b'gallery' in returned, 'Did not return to gallery'
            assert b'Traceback (most recent call last)' not in output, 'Python traceback'
            os.write(fd, b'\x1b')
            drain(fd, .3)
            finished, status = os.waitpid(pid, os.WNOHANG)
            assert finished and status == 0, 'Launcher did not exit cleanly'
            pid = None
            result = {'program': name, 'status': 'PASS', 'bytes': len(output)}
        except AssertionError as error:
            result = {'program': name, 'status': 'FAIL', 'detail': str(error)}
        finally:
            logs.joinpath(name + '.log').write_bytes(output)
            if pid:
                try:
                    os.killpg(pid, signal.SIGTERM)
                    drain(fd, .3)
                    os.killpg(pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                os.waitpid(pid, 0)
            os.close(fd)
        return result


def check_oneko(binary, logs):
    if not shutil.which('Xvfb'):
        return {'program': 'oneko', 'status': 'SKIP', 'detail': 'Install xorg-server-xvfb for an isolated display'}
    with tempfile.TemporaryDirectory(prefix='art-oneko-') as tmp:
        read_fd, write_fd = os.pipe()
        server = subprocess.Popen(['Xvfb', '-displayfd', str(write_fd), '-screen', '0', '800x600x24', '-nolisten', 'tcp', '-fbdir', tmp], pass_fds=(write_fd,), stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        os.close(write_fd)
        try:
            assert select.select([read_fd], [], [], 10)[0], 'Xvfb startup timed out'
            display = ':' + os.read(read_fd, 100).decode().strip()
            assert display[1:].isdigit(), 'Xvfb did not allocate a display'
            return check('oneko', binary, logs, display, Path(tmp) / 'Xvfb_screen0')
        finally:
            os.close(read_fd)
            server.terminate()
            server.communicate(timeout=5)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--program', action='append', choices=NAMES)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    binary = shutil.which('art')
    assert binary, 'Install art first'
    results = []
    for name in args.program or NAMES:
        result = check(name, binary, args.output)
        results.append(result)
        print(json.dumps(result), flush=True)
        (args.output / 'results.json').write_text(json.dumps(results, indent=2) + '\n')
    raise SystemExit(any(r['status'] != 'PASS' for r in results))
