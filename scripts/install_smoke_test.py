#!/usr/bin/env python3
"""Bulk package planning and execution with a fake manager; no system changes."""
import os
from pathlib import Path
import subprocess
import tempfile

BINARY=Path(__file__).resolve().parents[1]/"target/debug/art"
with tempfile.TemporaryDirectory(prefix="art-install-test-") as temp:
    root=Path(temp)
    log=root/"calls"
    manager=root/"yay"
    manager.write_text(f'#!/bin/sh\nprintf "%s\\n" "$@" > "{log}"\nprintf "#!/bin/sh\\nexit 0\\n" > "{root}/cmatrix"\n/bin/chmod +x "{root}/cmatrix"\n')
    manager.chmod(0o755)
    env=dict(os.environ,PATH=temp)
    def run(*args):
        return subprocess.run([str(BINARY),"install",*args],env=env,capture_output=True,text=True,timeout=5)
    result=run("--all","--dry-run")
    assert result.returncode==0,result.stderr
    assert "unimatrix-git" in result.stdout and "starfetch: skipped" in result.stdout
    assert not log.exists(),"Dry run invoked package manager"
    # Root must not invoke AUR helpers; CI's root container verifies this guard.
    result=run("cmatrix")
    if os.geteuid()==0:
        assert result.returncode!=0 and "regular user" in result.stderr
    else:
        assert result.returncode==0,result.stderr
        assert log.read_text().splitlines()==["-S","--needed","--","cmatrix"]
    manager.rename(root/"pacman")
    result=run("space","--dry-run")
    assert result.returncode==0,result.stderr
    assert "astroterm" in result.stdout and "requires yay or paru" in result.stdout
    assert run("not-a-category","--dry-run").returncode!=0
    print("PASS: bulk dry run, AUR/source handling, manager argv, root guard, invalid selection")
