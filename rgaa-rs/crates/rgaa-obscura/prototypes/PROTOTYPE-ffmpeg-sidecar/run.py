"""PROTOTYPE — THROWAWAY. Wipe me.  Ticket #340 (carte #330).

Question : un sidecar ffmpeg/ffprobe extrait-il de façon fiable pistes, sous-titres,
audiodescription et séquences de flashs sur des médias, et à quel coût (temps, octets) ?

Hypothèse de travail (la question n'est ni « logique » ni « UI » : c'est une mesure d'outil) :
script CLI jetable, médias SYNTHÉTIQUES générés par ffmpeg lui-même, aucun téléchargement.

NON mesuré ici (à dire dans le verdict) : Whisper (absent), IRIS (non compilé), OCR français
(pack tesseract `fra` absent), médias réels de production, HLS/DASH, VBR/codecs exotiques.

    python3 run.py <dossier de travail>
"""
import json, os, re, shutil, socket, socketserver, statistics, subprocess, sys, threading, time
import http.server
import numpy as np

W = sys.argv[1] if len(sys.argv) > 1 else "/tmp/ffproto"
os.makedirs(W, exist_ok=True)
FF = ["ffmpeg", "-hide_banner", "-loglevel", "error", "-y"]
R = {}                                   # résultats JSON


def sh(cmd, **kw):
    t = time.perf_counter()
    p = subprocess.run(cmd, capture_output=True, **kw)
    return p, time.perf_counter() - t


def p(path): return os.path.join(W, path)


# ───────────────────────── 1. médias synthétiques ─────────────────────────
def gen():
    vid = ["-f", "lavfi", "-i", "testsrc2=size=640x360:rate=25:duration=10"]
    tone = lambda f: ["-f", "lavfi", "-i", f"sine=frequency={f}:duration=10"]
    vtt = p("sub.vtt")
    open(vtt, "w").write("WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nBonjour tout le monde\n\n00:00:05.000 --> 00:00:09.000\nDeuxieme ligne\n")
    srt = p("sub.srt")
    open(srt, "w").write("1\n00:00:01,000 --> 00:00:04,000\nBonjour tout le monde\n\n2\n00:00:05,000 --> 00:00:09,000\nDeuxieme ligne\n")
    # main audio + audio description (disposition visual_impaired) + sous-titres + sous-titres SDH
    maps = ["-map", "0:v", "-map", "1:a", "-map", "2:a", "-map", "3:s", "-map", "3:s"]
    meta = ["-metadata:s:a:0", "language=fra", "-metadata:s:a:1", "language=fra", "-metadata:s:a:1", "title=Audiodescription",
            "-disposition:a:1", "visual_impaired", "-metadata:s:s:0", "language=fra",
            "-metadata:s:s:1", "language=fra", "-metadata:s:s:1", "title=Sous-titres SDH", "-disposition:s:1", "hearing_impaired+captions"]
    base = FF + vid + tone(440) + tone(880) + ["-i", srt]
    sh(base + maps + meta + ["-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac", "-c:s", "mov_text", p("av_sub_ad.mp4")])
    sh(base + maps + meta + ["-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac", "-c:s", "srt", p("av_sub_ad.mkv")])
    sh(FF + vid + tone(440) + tone(880) + ["-i", vtt, "-map", "0:v", "-map", "1:a", "-map", "2:a", "-map", "3:s",
        "-metadata:s:a:1", "title=Audiodescription", "-disposition:a:1", "visual_impaired",
        "-c:v", "libvpx", "-b:v", "500k", "-c:a", "libvorbis", "-c:s", "webvtt", p("av_sub_ad.webm")])
    # médias sans/avec son pour 4.10 (≤3 s de son = OK)
    sh(FF + vid + ["-c:v", "libx264", "-preset", "ultrafast", p("silent.mp4")])
    sh(FF + tone(440)[:0] + ["-f", "lavfi", "-i", "sine=frequency=440:duration=2", "-c:a", "libmp3lame", p("audio_2s.mp3")])
    sh(FF + ["-f", "lavfi", "-i", "sine=frequency=440:duration=8", "-c:a", "libmp3lame", p("audio_8s.mp3")])
    sh(FF + ["-f", "lavfi", "-i", "sine=frequency=440:duration=10", "-f", "lavfi", "-i", "anullsrc=r=44100:cl=mono:d=10",
             "-filter_complex", "[0:a]atrim=0:2[a];[1:a]atrim=0:8[b];[a][b]concat=n=2:v=0:a=1", "-c:a", "libmp3lame", p("audio_2s_then_silence.mp3")])
    # flashs : plein cadre 6 Hz (échec), plein cadre 2 Hz (OK), petit carré 6 Hz (OK), rouge/noir 4 Hz (échec), sobre
    def flash(name, expr, size="640x360", dur=5):
        sh(FF + ["-f", "lavfi", "-i", f"color=c=black:size={size}:rate=25:duration={dur}", "-vf", expr,
                 "-c:v", "libx264", "-preset", "ultrafast", "-pix_fmt", "yuv420p", p(name)])
    flash("flash_fullframe_6hz.mp4", "geq=lum='255*lt(mod(floor(T*12),2),1)':cb=128:cr=128")
    flash("flash_fullframe_2hz.mp4", "geq=lum='255*lt(mod(floor(T*4),2),1)':cb=128:cr=128")
    flash("flash_small_6hz.mp4", "geq=lum='if(lt(X,40)*lt(Y,40),255*lt(mod(floor(T*12),2),1),0)':cb=128:cr=128")
    flash("flash_red_4hz.mp4", "geq=lum='76*lt(mod(floor(T*8),2),1)':cb='85*lt(mod(floor(T*8),2),1)+128*gte(mod(floor(T*8),2),1)':cr='255*lt(mod(floor(T*8),2),1)+128*gte(mod(floor(T*8),2),1)'")
    flash("no_flash.mp4", "geq=lum='128+60*sin(T)':cb=128:cr=128")
    # bornes : 3 flashs/s (OK) vs 4 flashs/s (échec) ; surface juste sous / juste au-dessus de 21 824 px
    flash("b_3flashes_per_s.mp4", "geq=lum='255*lt(mod(floor(T*6),2),1)':cb=128:cr=128")
    flash("b_4flashes_per_s.mp4", "geq=lum='255*lt(mod(floor(T*8),2),1)':cb=128:cr=128")
    flash("b_sq_144.mp4", "geq=lum='if(lt(X,144)*lt(Y,144),255*lt(mod(floor(T*12),2),1),0)':cb=128:cr=128")
    flash("b_sq_152.mp4", "geq=lum='if(lt(X,152)*lt(Y,152),255*lt(mod(floor(T*12),2),1),0)':cb=128:cr=128")
    # variation lente (rampe) : ne doit PAS compter comme flash
    flash("b_slow_ramp.mp4", "geq=lum='128+127*sin(T*6)':cb=128:cr=128")
    # même chose en 720p pour mesurer le coût de décodage réaliste
    flash("flash_fullframe_6hz_720p.mp4", "geq=lum='255*lt(mod(floor(T*12),2),1)':cb=128:cr=128", size="1280x720", dur=10)
    # texte incrusté (OCR) — anglais seulement, pas de pack fra
    sh(FF + ["-f", "lavfi", "-i", "color=c=black:size=640x360:rate=25:duration=4", "-vf",
             "drawtext=fontfile=/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf:text='Hello everybody this is a test':fontsize=36:fontcolor=white:x=(w-text_w)/2:y=h-80",
             "-c:v", "libx264", "-preset", "ultrafast", p("burned_sub.mp4")])
    # gros fichier pour le test HTTP : 720p bruité 30 s, moov à la fin (défaut) vs faststart
    big = FF + ["-f", "lavfi", "-i", "testsrc2=size=1280x720:rate=25:duration=30", "-f", "lavfi", "-i", "sine=frequency=300:duration=30",
                "-c:v", "libx264", "-preset", "ultrafast", "-crf", "28", "-c:a", "aac"]
    sh(big + [p("big_moov_end.mp4")])
    sh(big + ["-movflags", "+faststart", p("big_faststart.mp4")])


# ───────────────────────── 2. ffprobe : pistes & marqueurs ─────────────────────────
def probe(path):
    out, dt = sh(["ffprobe", "-v", "error", "-show_streams", "-show_format", "-of", "json", path])
    return json.loads(out.stdout or b"{}"), dt


def summarize(j):
    rows = []
    for s in j.get("streams", []):
        d = {k for k, v in s.get("disposition", {}).items() if v}
        t = s.get("tags", {})
        rows.append(f"{s['codec_type'][:1]}:{s['codec_name']} lang={t.get('language','-')} title={t.get('title','-')!r} disp={sorted(d - {'default'}) or '-'}")
    return rows


def part_probe():
    print("\n=== 2. ffprobe — pistes et marqueurs (le marqueur survit-il au conteneur ?) ===")
    R["probe"] = {}
    for f in ("av_sub_ad.mp4", "av_sub_ad.mkv", "av_sub_ad.webm"):
        times = [probe(p(f))[1] for _ in range(5)]
        j, _ = probe(p(f))
        ad = [s for s in j["streams"] if s["codec_type"] == "audio" and s.get("disposition", {}).get("visual_impaired")]
        sdh = [s for s in j["streams"] if s["codec_type"] == "subtitle" and (s.get("disposition", {}).get("hearing_impaired") or s.get("disposition", {}).get("captions"))]
        titled = [s for s in j["streams"] if "audiodescription" in s.get("tags", {}).get("title", "").lower()]
        R["probe"][f] = dict(ms=round(statistics.median(times) * 1000, 1), n_streams=len(j["streams"]),
                             ad_by_disposition=len(ad), ad_by_title=len(titled), sdh_flag=len(sdh), size=os.path.getsize(p(f)))
        print(f"\n  {f}  ({os.path.getsize(p(f))/1024:.0f} Ko)  ffprobe médiane {R['probe'][f]['ms']} ms")
        for r in summarize(j): print("     ", r)
        print(f"      → AD détectée par disposition={len(ad)}  par titre={len(titled)}  SDH/captions={len(sdh)}")


# ───────────────────────── 3. extraction de sous-titres ─────────────────────────
def part_subs():
    print("\n=== 3. extraction des sous-titres vers WebVTT ===")
    R["subs"] = {}
    for f in ("av_sub_ad.mp4", "av_sub_ad.mkv", "av_sub_ad.webm"):
        r, dt = sh(["ffmpeg", "-v", "error", "-i", p(f), "-map", "0:s:0", "-f", "webvtt", "-"])
        txt = r.stdout.decode("utf8", "replace")
        cues = len(re.findall(r"-->", txt))
        R["subs"][f] = dict(ms=round(dt * 1000, 1), cues=cues, non_empty=bool(re.search(r"[A-Za-z]{3}", re.sub(r"WEBVTT|[0-9:.>\- ]", "", txt))))
        print(f"  {f:18s} {R['subs'][f]['ms']:6.1f} ms  cues={cues}  contenu_non_vide={R['subs'][f]['non_empty']}")


# ───────────────────────── 4. 4.10 : durée de son audible ─────────────────────────
def audible(path):
    r, dt = sh(["ffmpeg", "-hide_banner", "-nostats", "-i", path, "-vn", "-af", "silencedetect=noise=-50dB:d=0.3", "-f", "null", "-"])
    log = r.stderr.decode("utf8", "replace")
    dur = float(re.search(r"Duration: (\d+):(\d+):([\d.]+)", log).group(1)) * 3600 + float(re.search(r"Duration: (\d+):(\d+):([\d.]+)", log).group(2)) * 60 + float(re.search(r"Duration: (\d+):(\d+):([\d.]+)", log).group(3))
    sil = sum(float(x) for x in re.findall(r"silence_duration: ([\d.]+)", log))
    has_audio = "Audio:" in log
    return has_audio, dur, max(dur - sil, 0.0), dt


def part_audio():
    print("\n=== 4. critère 4.10 — durée de son audible (silencedetect) ===")
    R["audio"] = {}
    for f, attendu in (("silent.mp4", "pas de piste audio"), ("audio_2s.mp3", "≈2 s"), ("audio_8s.mp3", "≈8 s"), ("audio_2s_then_silence.mp3", "≈2 s sur 10 s")):
        has, dur, aud, dt = audible(p(f))
        R["audio"][f] = dict(has_audio=has, duration=round(dur, 2), audible=round(aud, 2), ms=round(dt * 1000, 1))
        verdict = "pas de piste → pass déterministe" if not has else ("≤3 s audible → pass déterministe" if aud <= 3.0 else ">3 s audible → review (le fail reste humain)")
        shown = f"audible={aud:5.2f}s sur {dur:5.2f}s" if has else "aucune piste audio        "
        print(f"  {f:28s} attendu {attendu:22s} mesuré {shown}  [{dt*1000:5.0f} ms]  {verdict}")


# ───────────────────────── 5. flashs (WCAG 2.3.1, version simplifiée) ─────────────────────────
GAMMA = np.array([((i / 255) / 12.92 if i / 255 <= 0.03928 else (((i / 255) + 0.055) / 1.055) ** 2.4) for i in range(256)], dtype=np.float32)
RENDER = (640, 360)          # taille « rendue » supposée dans la page
THRESH_PX = 21824            # RGAA 13.7.2 / 13.7.3
GW, GH, FPS = 160, 90, 25


def flashes(path, render=RENDER):
    t0 = time.perf_counter()
    r, _ = sh(["ffmpeg", "-v", "error", "-i", path, "-an", "-vf", f"scale={GW}:{GH}:flags=area,fps={FPS}", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
    t_dec = time.perf_counter() - t0
    buf = np.frombuffer(r.stdout, dtype=np.uint8)
    n = buf.size // (GW * GH * 3)
    if n < FPS + 2:
        return None
    fr = buf[: n * GW * GH * 3].reshape(n, GH, GW, 3)
    t1 = time.perf_counter()
    lin = GAMMA[fr]
    lum = 0.2126 * lin[..., 0] + 0.7152 * lin[..., 1] + 0.0722 * lin[..., 2]       # luminance relative
    # Un flash = paire de changements OPPOSÉS de luminance relative ≥ 10 %, état sombre < 0,8.
    # On suit, par pixel, le dernier extremum : un changement n'est compté que lorsqu'il INVERSE la
    # direction précédente d'au moins 0,10 (hystérésis). Une rampe continue donne 1 inversion par demi-période.
    T = lum.shape[0]
    ref = lum[0].copy()
    direction = np.zeros(lum.shape[1:], dtype=np.int8)
    trans = np.zeros(lum.shape, dtype=np.int16)
    for t in range(1, T):
        L = lum[t]
        und = direction == 0
        move = np.abs(L - ref) >= 0.10
        start = und & move
        direction[start] = np.sign(L - ref)[start].astype(np.int8)
        ref[start] = L[start]
        up, dn = direction == 1, direction == -1
        ext_up = up & (L > ref); ext_dn = dn & (L < ref)             # on prolonge l'extremum
        ref[ext_up] = L[ext_up]; ref[ext_dn] = L[ext_dn]
        rev = (up & (ref - L >= 0.10)) | (dn & (L - ref >= 0.10))   # inversion franche
        rev &= np.minimum(ref, L) < 0.80                             # l'état sombre doit être < 0,80
        trans[t][rev] = 1
        direction[rev] *= -1
        ref[rev] = L[rev]
    cs = np.cumsum(trans, axis=0)
    win = cs[FPS:] - cs[:-FPS]                                                      # inversions par fenêtre d'1 s
    flashing = (win >= 8)                                                           # > 3 flashs (paires) dans 1 s
    px_per_cell = (render[0] * render[1]) / (GW * GH)
    area = flashing.reshape(flashing.shape[0], -1).sum(axis=1).max() * px_per_cell
    # rouge saturé : r/(r+g+b) ≥ 0.8 et transition de r
    rr = fr[..., 0].astype(np.float32) / np.maximum(fr.sum(axis=3), 1)
    red = (rr >= 0.8) & (fr[..., 0] > 100)
    red_tr = (np.diff(red.astype(np.int8), axis=0) != 0).astype(np.int16)
    rcs = np.cumsum(red_tr, axis=0)
    rwin = (rcs[FPS:] - rcs[:-FPS]) >= 8
    red_area = rwin.reshape(rwin.shape[0], -1).sum(axis=1).max() * px_per_cell
    t_cpu = time.perf_counter() - t1
    dur = n / FPS
    return dict(area_px=float(area), red_area_px=float(red_area), fail=bool(area >= THRESH_PX), red_fail=bool(red_area >= THRESH_PX),
                dur=round(dur, 1), t_decode=round(t_dec, 2), t_compute=round(t_cpu, 2), x_realtime=round(dur / (t_dec + t_cpu), 1))


def part_flash():
    print(f"\n=== 5. critère 13.7 — détection de flashs (seuil {THRESH_PX} px à la taille rendue {RENDER[0]}x{RENDER[1]}) ===")
    R["flash"] = {}
    attendu = {"flash_fullframe_6hz.mp4": "FAIL", "flash_fullframe_2hz.mp4": "pass (2 flashs/s)", "flash_small_6hz.mp4": "pass (petite surface)",
               "flash_red_4hz.mp4": "FAIL (rouge)", "no_flash.mp4": "pass"}
    for f, a in attendu.items():
        r = flashes(p(f))
        R["flash"][f] = r
        print(f"  {f:26s} attendu {a:22s} → surface={r['area_px']:8.0f} px  fail={r['fail']!s:5s} | rouge={r['red_area_px']:8.0f} px red_fail={r['red_fail']!s:5s}")
    print("\n  --- bornes (le détecteur doit discriminer, pas seulement détecter les cas extrêmes) ---")
    bornes = {"b_3flashes_per_s.mp4": "pass (3 flashs/s = limite)", "b_4flashes_per_s.mp4": "FAIL (4 flashs/s)",
              "b_sq_144.mp4": "pass (144² = 20 736 px < 21 824)", "b_sq_152.mp4": "FAIL (152² = 23 104 px ≥ 21 824)",
              "b_slow_ramp.mp4": "pass (rampe lente, pas un flash)"}
    for f, a in bornes.items():
        r = flashes(p(f)); R["flash"][f] = r
        print(f"  {f:26s} attendu {a:36s} → surface={r['area_px']:8.0f} px  fail={r['fail']!s}")
    r = flashes(p("flash_fullframe_6hz_720p.mp4"), render=(1280, 720))
    R["flash"]["720p"] = r
    print(f"\n  coût (720p, {r['dur']} s de vidéo) : décodage+réduction {r['t_decode']} s, calcul {r['t_compute']} s  → {r['x_realtime']}× temps réel"
          f"  (≈ {60 / r['x_realtime']:.1f} s de CPU par minute de vidéo)")


# ───────────────────────── 6. OCR du texte incrusté ─────────────────────────
def part_ocr():
    print("\n=== 6. texte incrusté — extraction d'image + tesseract (ANGLAIS seulement : pas de pack fra) ===")
    t = time.perf_counter()
    sh(FF + ["-ss", "1", "-i", p("burned_sub.mp4"), "-frames:v", "1", p("frame.png")])
    t_f = time.perf_counter() - t
    t = time.perf_counter()
    r, _ = sh(["tesseract", p("frame.png"), "-", "-l", "eng"])
    t_o = time.perf_counter() - t
    txt = r.stdout.decode().strip()
    ok = "everybody" in txt.lower()
    R["ocr"] = dict(text=txt, ok=ok, ms_frame=round(t_f * 1000), ms_ocr=round(t_o * 1000))
    print(f"  attendu : 'Hello everybody this is a test'   lu : {txt!r}   exact={ok}   [{t_f*1000:.0f} ms image + {t_o*1000:.0f} ms OCR]")
    print("  NON mesuré : français (pack `fra` absent), fonds complexes, sous-titres anti-aliasés réels")


# ───────────────────────── 7. octets lus via HTTP (faut-il télécharger ?) ─────────────────────────
class RangeHandler(http.server.BaseHTTPRequestHandler):
    served = 0
    reqs = 0

    def log_message(self, *a): pass

    def setup(self):
        super().setup()
        # tampon d'envoi minuscule : sinon le noyau avale 2-4 Mo d'avance et « octets envoyés » ≠ « octets lus par le client »
        self.request.setsockopt(socket.SOL_SOCKET, socket.SO_SNDBUF, 4096)

    def do_GET(self):
        path = os.path.join(W, self.path.lstrip("/"))
        size = os.path.getsize(path)
        rng = self.headers.get("Range")
        a, b = 0, size - 1
        if rng:
            m = re.match(r"bytes=(\d*)-(\d*)", rng)
            if m.group(1): a = int(m.group(1))
            if m.group(2): b = min(int(m.group(2)), size - 1)
            if not m.group(1) and m.group(2): a = max(size - int(m.group(2)), 0); b = size - 1
            self.send_response(206)
            self.send_header("Content-Range", f"bytes {a}-{b}/{size}")
        else:
            self.send_response(200)
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(b - a + 1))
        self.end_headers()
        RangeHandler.reqs += 1                      # compter la requête même si le client coupe
        try:
            with open(path, "rb") as fh:
                fh.seek(a)
                left = b - a + 1
                while left > 0:
                    chunk = fh.read(min(65536, left))
                    self.wfile.write(chunk)
                    self.wfile.flush()
                    RangeHandler.served += len(chunk)   # octets réellement envoyés
                    left -= len(chunk)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def do_HEAD(self):
        size = os.path.getsize(os.path.join(W, self.path.lstrip("/")))
        self.send_response(200); self.send_header("Content-Length", str(size)); self.send_header("Accept-Ranges", "bytes"); self.end_headers()


def part_http():
    print("\n=== 7. ffprobe / silencedetect / flashs SUR HTTP : combien d'octets faut-il vraiment lire ? ===")
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), RangeHandler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    port = srv.server_address[1]
    R["http"] = {}
    for f in ("big_moov_end.mp4", "big_faststart.mp4"):
        size = os.path.getsize(p(f))
        row = dict(size=size)
        for label, cmd in (("ffprobe (pistes/marqueurs)", ["ffprobe", "-v", "error", "-show_streams", "-of", "json"]),
                           ("silencedetect (audio seul)", ["ffmpeg", "-hide_banner", "-nostats", "-vn", "-af", "silencedetect=d=0.3", "-f", "null", "-"]),
                           ("flashs (vidéo entière)", None)):
            RangeHandler.served = 0; RangeHandler.reqs = 0
            url = f"http://127.0.0.1:{port}/{f}"
            t = time.perf_counter()
            if cmd is None:
                sh(["ffmpeg", "-v", "error", "-i", url, "-an", "-vf", "scale=160:90,fps=25", "-f", "null", "-"])
            elif cmd[0] == "ffprobe":
                sh(cmd + [url])
            else:
                sh(cmd[:2] + ["-i", url] + cmd[2:])
            dt = time.perf_counter() - t
            row[label] = dict(bytes=RangeHandler.served, reqs=RangeHandler.reqs, pct=round(100 * RangeHandler.served / size, 1), s=round(dt, 2))
        R["http"][f] = row
        print(f"\n  {f}  ({size/1e6:.2f} Mo)")
        for k, v in row.items():
            if k != "size": print(f"     {k:28s} {v['bytes']/1024:9.0f} Ko lus = {v['pct']:5.1f} % du fichier, {v['reqs']:3d} requêtes, {v['s']:5.2f} s")


def part_probesize():
    print("\n=== 8. ffprobe avec probesize réduit — inventaire des pistes sans lire 15-20 % du fichier ===")
    srv = socketserver.ThreadingTCPServer(("127.0.0.1", 0), RangeHandler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    port = srv.server_address[1]
    R["probesize"] = {}
    for f in ("big_moov_end.mp4", "big_faststart.mp4"):
        for label, extra in (("défaut (probesize 5 Mo)", []), ("probesize 64 Ko", ["-probesize", "65536", "-analyzeduration", "0"])):
            RangeHandler.served = 0; RangeHandler.reqs = 0
            t = time.perf_counter()
            r, _ = sh(["ffprobe", "-v", "error", "-show_streams", "-of", "json"] + extra + [f"http://127.0.0.1:{port}/{f}"])
            dt = time.perf_counter() - t
            try: ns = len(json.loads(r.stdout)["streams"])
            except Exception: ns = -1
            R["probesize"][f"{f}|{label}"] = dict(bytes=RangeHandler.served, reqs=RangeHandler.reqs, streams=ns, s=round(dt, 2))
            print(f"  {f:18s} {label:24s} {RangeHandler.served/1024:8.0f} Ko lus, {RangeHandler.reqs} req., pistes trouvées={ns}, {dt:5.2f} s")
    # fidélité de l'inventaire en probesize réduit sur les fichiers à marqueurs (local)
    print("  fidélité (pistes + dispositions identiques au mode complet ?) :")
    for f in ("av_sub_ad.mp4", "av_sub_ad.mkv", "av_sub_ad.webm"):
        full = json.loads(sh(["ffprobe", "-v", "error", "-show_streams", "-of", "json", p(f)])[0].stdout)["streams"]
        small = json.loads(sh(["ffprobe", "-v", "error", "-show_streams", "-of", "json", "-probesize", "65536", "-analyzeduration", "0", p(f)])[0].stdout)["streams"]
        key = lambda ss: [(x["codec_type"], x["codec_name"], tuple(sorted(k for k, v in x.get("disposition", {}).items() if v and k != "default"))) for x in ss]
        print(f"     {f:16s} identique={key(full) == key(small)}  ({len(full)} pistes)")


if __name__ == "__main__":
    t0 = time.perf_counter()
    PARTS = os.environ.get("PARTS", "all")
    if PARTS != "all" and os.path.exists(p("av_sub_ad.mp4")):
        for name, fn in (("7", part_http), ("8", part_probesize)):
            if name in PARTS.split(","): fn()
        sys.exit(0)
    print("PROTOTYPE jetable — ticket #340 — médias synthétiques, ffmpeg", subprocess.run(["ffmpeg", "-version"], capture_output=True, text=True).stdout.splitlines()[0])
    gen(); print(f"(médias générés en {time.perf_counter()-t0:.1f} s ; {sum(os.path.getsize(p(x)) for x in os.listdir(W) if x.endswith(('.mp4','.mkv','.webm','.mp3')))/1e6:.1f} Mo)")
    part_probe(); part_subs(); part_audio(); part_flash(); part_ocr(); part_http(); part_probesize()
    json.dump(R, open(p("results.json"), "w"), indent=1, default=str)
    print(f"\nrésultats bruts : {p('results.json')}   (durée totale {time.perf_counter()-t0:.0f} s)")
