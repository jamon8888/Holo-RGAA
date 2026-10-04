"""PROTOTYPE — throwaway. Engine-agnostic variant: does the page cancel a synthetic Tab keydown,
and if so can an exit key (Escape) release focus?  Usage: python3 probe_dp.py obscura|chromium"""
import asyncio, json, subprocess, sys, time, os, urllib.request, http.server, threading, socketserver
import websockets
HERE=os.path.dirname(os.path.abspath(__file__)); PH,PC=8785,9353
EXPECTED={"no-trap":"pass","single-focusable":"pass","no-focusable":"pass","negative-tabindex":"pass",
 "modal-legit-escape":"pass-by-exit-key","trap-keydown":"fail","trap-refocus":"fail","trap-escapable":"pass-by-exit-key","trap-no-autofocus":"fail"}
JS="""(()=>{const d=e=>!e||e===document.body?'body':e.tagName.toLowerCase()+(e.id?'#'+e.id:'');
const fire=(k,sh)=>{const t=document.activeElement||document.body;const ev=new KeyboardEvent('keydown',{key:k,shiftKey:!!sh,bubbles:true,cancelable:true});return {prevented:!t.dispatchEvent(ev),el:d(t)}};
const tab=fire('Tab',false);const before=d(document.activeElement);
if(!tab.prevented) return JSON.stringify({verdict:'ok',tab});
const esc=fire('Escape',false);const after=d(document.activeElement);
const stab=fire('Tab',true);
return JSON.stringify({verdict:(after!==before||!document.contains(document.getElementById(before.split('#')[1]||'x'))&&before!=='body')?'tab-blocked-but-exit-key':'TRAP',tab,esc:{after},shiftTab:stab.prevented})})()"""
SWEEP="(()=>{const f=[...document.querySelectorAll('a[href],button,input:not([type=hidden]),select,textarea,[tabindex]')].filter(e=>e.tabIndex>=0&&!e.disabled);\nconst hits=[];for(const e of f){e.focus();const ev=new KeyboardEvent('keydown',{key:'Tab',bubbles:true,cancelable:true});if(!e.dispatchEvent(ev)){const before=document.activeElement;\nconst esc=new KeyboardEvent('keydown',{key:'Escape',bubbles:true,cancelable:true});e.dispatchEvent(esc);hits.push({el:e.tagName.toLowerCase()+(e.id?'#'+e.id:''),exit:document.activeElement!==before||!document.contains(e)})}}\nconst bad=hits.filter(h=>!h.exit);return JSON.stringify({verdict:bad.length?'TRAP':(hits.length?'tab-blocked-but-exit-key':'ok'),focusables:f.length,hits})})()"
async def main(engine):
    socketserver.TCPServer.allow_reuse_address=True
    srv=socketserver.TCPServer(("127.0.0.1",PH),lambda *a,**k:type("Q",(http.server.SimpleHTTPRequestHandler,),{"log_message":lambda *a:None})(*a,directory=HERE+"/pages",**k))
    threading.Thread(target=srv.serve_forever,daemon=True).start()
    cmd=["obscura","serve","--port",str(PC),"--allow-private-network"] if engine=="obscura" else ["google-chrome","--headless=new",f"--remote-debugging-port={PC}","--no-sandbox","--user-data-dir=/tmp/proto-chrome2","about:blank"]
    p=subprocess.Popen(cmd,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    try:
        for _ in range(50):
            try: v=json.load(urllib.request.urlopen(f"http://127.0.0.1:{PC}/json/version"));break
            except Exception: time.sleep(0.2)
        async with websockets.connect(v["webSocketDebuggerUrl"],max_size=None) as ws:
            n=[0]
            async def s(m,pr=None,sid=None):
                n[0]+=1;d={"id":n[0],"method":m,"params":pr or {}}
                if sid:d["sessionId"]=sid
                await ws.send(json.dumps(d))
                while True:
                    r=json.loads(await ws.recv())
                    if r.get("id")==n[0]:return r.get("result",r.get("error"))
            print(f"engine={engine}\n{'page':22}{'expected':18}{'verdict':28}{'ms':>6}  raw")
            for name,exp in EXPECTED.items():
                t0=time.perf_counter()
                tid=(await s("Target.createTarget",{"url":f"http://localhost:{PH}/{name}.html"}))["targetId"]
                sid=(await s("Target.attachToTarget",{"targetId":tid,"flatten":True}))["sessionId"]
                await asyncio.sleep(0.4)
                r=await s("Runtime.evaluate",{"expression":SWEEP if len(sys.argv)>2 else JS,"returnByValue":True},sid)
                o=json.loads(r["result"]["value"]);await s("Target.closeTarget",{"targetId":tid})
                print(f"{name:22}{exp:18}{o['verdict']:28}{(time.perf_counter()-t0)*1000:6.0f}  {json.dumps(o)[:90]}")
    finally: p.terminate();srv.shutdown()
asyncio.run(main(sys.argv[1]))
