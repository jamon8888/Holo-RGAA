"""PROTOTYPE — throwaway. Answers one question (ticket "sonde comportementale clavier 12.9"):
is a Tab-driven keyboard-trap probe reliable and cheap enough under Obscura?
Not production code. Wipe me.

Usage: python3 probe.py obscura|chromium [runs]
Starts a local static server for ./pages, starts the engine, drives N Tab presses per page over CDP,
prints focus trace, verdict, wall time, and run-to-run agreement.
"""
import asyncio, json, subprocess, sys, time, os, urllib.request, http.server, threading, socketserver, statistics
import websockets

HERE = os.path.dirname(os.path.abspath(__file__))
PORT_HTTP, PORT_CDP = 8775, 9343
TABS = 25  # presses per page (cost knob)
# page -> what a human auditor would say about RGAA 12.9
EXPECTED = {
    "no-trap": "pass", "single-focusable": "pass", "no-focusable": "pass",
    "negative-tabindex": "pass", "modal-legit-escape": "pass-by-exit-key",
    "trap-keydown": "fail", "trap-refocus": "fail", "trap-escapable": "pass-by-exit-key",
}

DESCRIBE = """(()=>{const e=document.activeElement;if(!e||e===document.body)return 'body';
return e.tagName.toLowerCase()+(e.id?'#'+e.id:'')+':'+((e.textContent||e.getAttribute('aria-label')||'').trim().slice(0,15));})()"""


class Cdp:
    def __init__(self, ws): self.ws, self.n = ws, 0
    async def send(self, method, params=None, session=None):
        self.n += 1
        msg = {"id": self.n, "method": method, "params": params or {}}
        if session: msg["sessionId"] = session
        await self.ws.send(json.dumps(msg))
        while True:
            r = json.loads(await asyncio.wait_for(self.ws.recv(), 20))
            if r.get("id") == self.n:
                if "error" in r: raise RuntimeError(f"{method}: {r['error']}")
                return r.get("result", {})


async def probe_page(cdp, name):
    t0 = time.perf_counter()
    url = f"http://localhost:{PORT_HTTP}/{name}.html"
    tid = (await cdp.send("Target.createTarget", {"url": url}))["targetId"]
    sid = (await cdp.send("Target.attachToTarget", {"targetId": tid, "flatten": True}))["sessionId"]
    await asyncio.sleep(0.4)  # crude load wait: part of the cost being measured
    trace = []
    for _ in range(TABS):
        for kind in ("keyDown", "keyUp"):
            await cdp.send("Input.dispatchKeyEvent",
                           {"type": kind, "key": "Tab", "code": "Tab", "windowsVirtualKeyCode": 9}, sid)
        r = await cdp.send("Runtime.evaluate", {"expression": DESCRIBE, "returnByValue": True}, sid)
        trace.append(r["result"].get("value"))
    await cdp.send("Target.closeTarget", {"targetId": tid})
    return trace, time.perf_counter() - t0


def verdict(trace):
    """Naive trap rule (same family as rgaa-obscura lib.rs: N consecutive tabs on one element)."""
    moved = len(set(trace)) > 1
    run = best = 1
    for a, b in zip(trace, trace[1:]):
        run = run + 1 if a == b else 1
        best = max(best, run)
    uniq = len(set(trace))
    if not moved and trace[0] == "body": return "no-focus-movement"
    if best >= 5 and uniq > 1: return "TRAP?"            # stuck after having moved
    if best >= 5 and uniq == 1: return "TRAP?(single)"  # indistinguishable from 1 focusable
    return "ok"


async def main(engine, runs):
    socketserver.TCPServer.allow_reuse_address = True
    srv = socketserver.TCPServer(("127.0.0.1", PORT_HTTP),
                                 lambda *a, **k: type("Q", (http.server.SimpleHTTPRequestHandler,), {"log_message": lambda *a: None})(*a, directory=HERE + "/pages", **k))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    if engine == "obscura":
        cmd = ["obscura", "serve", "--port", str(PORT_CDP), "--allow-private-network"]
    else:
        cmd = ["google-chrome", "--headless=new", f"--remote-debugging-port={PORT_CDP}", "--no-sandbox",
               "--user-data-dir=/tmp/proto-chrome", "about:blank"]
    p = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try:
                ver = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT_CDP}/json/version"))
                break
            except Exception: time.sleep(0.2)
        else: sys.exit("engine did not start")
        async with websockets.connect(ver["webSocketDebuggerUrl"], max_size=None) as ws:
            cdp = Cdp(ws)
            print(f"engine={engine} tabs/page={TABS} runs={runs}")
            print(f"{'page':22}{'expected':18}{'verdict (runs)':34}{'ms/page':>8}  trace")
            for name, exp in EXPECTED.items():
                vs, ts, tr = [], [], None
                for _ in range(runs):
                    try: tr, dt = await probe_page(cdp, name)
                    except Exception as e: tr, dt = [f"ERR {e}"], 0
                    vs.append(verdict(tr) if not tr[0].startswith("ERR") else "error"); ts.append(dt * 1000)
                stable = "stable" if len(set(vs)) == 1 else "FLAKY"
                print(f"{name:22}{exp:18}{vs[0]+' '+stable:34}{statistics.mean(ts):8.0f}  {' > '.join(tr[:8])}")
    finally:
        p.terminate(); srv.shutdown()

asyncio.run(main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 3))
