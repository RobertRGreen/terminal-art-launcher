#!/usr/bin/env python3
"""Test preset loading against a simulated desktop. Never calls real Hyprland."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

BINARY = Path(__file__).resolve().parents[1] / "target/debug/art"
with tempfile.TemporaryDirectory(prefix="art-presets-test-") as temp:
    root=Path(temp)
    bins=root/"bin"
    bins.mkdir()
    config=root/"config/terminal-art-launcher/presets"
    config.mkdir(parents=True)
    runtime=root/"run"
    runtime.mkdir(mode=0o700)
    state=root/"clients.json"
    log=root/"operations.log"
    ready=root/"program-started"
    window=dict(label="test effect", class_="Console", source_pid=123, position=[173,68], size=[604,652], floating=True, terminal=True, command=[str(bins/"effect")], cwd=temp, kitty_config=None, cava_config=None)
    window["class"]=window.pop("class_")
    preset=dict(version=1,name="Spotify",workspace=3,monitor=dict(name="TEST",origin=[0,0],size=[3440,1440]),windows=[window],notes=[])
    (config/"spotify.json").write_text(json.dumps(preset))
    original=dict(address="0x123abc",pid=123,**{"class":"Console"},workspace={"id":3},floating=True,at=[173,68],size=[604,652])
    state.write_text(json.dumps([original]))
    def executable(name,code):
        file=bins/name
        file.write_text("#!/usr/bin/python3\n"+code)
        file.chmod(0o755)
    executable("hyprctl",f'''import json,sys,pathlib,re
state=pathlib.Path({str(state)!r})
args=sys.argv[1:]
if args[0]=="clients": print(state.read_text())
elif args[0]=="monitors": print(json.dumps([dict(name="TEST",width=3440,height=1440,scale=1,x=0,y=0,transform=0,focused=True)]))
elif args[0]=="activeworkspace": print('{{"id":3}}')
elif args[0]=="eval":
 code=args[1]
 with open({str(log)!r},"a") as f:f.write(code+"\\n")
 assert "action='set'" not in code and "action='unset'" not in code
 clients=json.loads(state.read_text())
 for c in clients:
  if c["address"] in code:
   if "action='enable'" in code:c["floating"]=True
   if "action='disable'" in code:c["floating"]=False
   match=re.search(r"window.resize.*?x=(\\d+),y=(\\d+)",code)
   if match:c["size"]=list(map(int,match.groups()))
   match=re.search(r"window.move.*?x=(\\d+),y=(\\d+)",code)
   if match:c["at"]=list(map(int,match.groups()))
 state.write_text(json.dumps(clients))
 print("ok")
else:raise AssertionError(args)
''')
    executable("kitty",f'''import sys,json,pathlib,os
args=sys.argv[1:]
state=pathlib.Path({str(state)!r})
cls=args[args.index("--class")+1]
client=dict(address="0x456def",pid=os.getpid(),workspace={{"id":3}},floating=True,at=[0,0],size=[100,100])
client["class"]=cls
state.write_text(json.dumps([client]))
command=args[args.index("--")+1:]
os.execv(command[0],command)
''')
    executable("effect",f'''import json,pathlib
client=json.loads(pathlib.Path({str(state)!r}).read_text())[0]
assert client["at"]==[173,68] and client["size"]==[604,652],client
pathlib.Path({str(ready)!r}).write_text("started after positioning")
''')
    env=dict(os.environ,PATH=str(bins),XDG_CONFIG_HOME=str(root/"config"),XDG_RUNTIME_DIR=str(runtime),HYPRLAND_INSTANCE_SIGNATURE="mock-only")
    def run(*args):
        result=subprocess.run([str(BINARY),*args],env=env,capture_output=True,text=True,timeout=12)
        assert result.returncode==0,(result.stdout,result.stderr)
        return result.stdout
    assert "Spotify" in run("preset","list")
    assert "1/1" in run("--preset","Spotify")
    assert not log.exists(),"Reapplying matching layout changed desktop state"
    original["at"]=[100,68]
    state.write_text(json.dumps([original]))
    run("preset","launch","Spotify")
    ops=log.read_text()
    assert "window.move" in ops and "window.resize" not in ops and "window.float" not in ops
    original["floating"]=False
    state.write_text(json.dumps([original]))
    run("--preset","Spotify")
    assert "action='enable'" in log.read_text()
    state.write_text("[]")
    run("--preset","Spotify")
    for _ in range(30):
        if ready.exists():break
        time.sleep(.05)
    assert ready.exists(),"New effect did not start after positioning"
    assert not list(runtime.glob("art-*.sock")),"Launch socket leaked"
    print("PASS: preset listing, direct launch, no-op restore, movement, floating state, delayed effect startup, socket cleanup")
