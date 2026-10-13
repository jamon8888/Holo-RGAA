#!/usr/bin/env python3
"""Feuille d'étiquetage pour la calibration d'une famille de preuve.

Le protocole est dans `docs/specs/calibration-protocol.md`. La contrainte que ce
générateur applique, et qui n'est pas négociable, est celle du §2 : **une justification
est obligatoire sur chaque carte, verdict favorable compris**.

Sans elle, le verdict ne porte aucune information indépendante du fait d'avoir écrit une
correction — mesuré à phi = -1,000 sur 39 cartes, avec une ancre factuellement fausse
acceptée sans un mot. La feuille marque donc les cartes incomplètes, les compte en tête,
et refuse l'export tant qu'il en reste une.

Entrée : un JSON de cartes `[{"qid", "image", "ctx", "alt"}, ...]`, l'image étant un
chemin relatif au répertoire du lot. La clé de correction n'entre jamais dans la page.

    python3 sheet.py lot/items.json --out lot/etiquetage.html --title "Lot E — descriptions détaillées"
"""
import argparse, base64, io, json, os, sys

MIN_JUSTIFICATION = 8

CSS = """:root{--bg:#fff;--fg:#1a1a1a;--mut:#555;--card:#f6f7f9;--bd:#c9ced6;--ac:#0b57d0;--warn:#c8690b}
@media(prefers-color-scheme:dark){:root:not([data-theme="light"]){--bg:#16181c;--fg:#eceff3;--mut:#aab2bd;--card:#20242a;--bd:#3a4049;--ac:#8ab4f8;--warn:#e8a04a}}
:root[data-theme="dark"]{--bg:#16181c;--fg:#eceff3;--mut:#aab2bd;--card:#20242a;--bd:#3a4049;--ac:#8ab4f8;--warn:#e8a04a}
body{font:16px/1.5 system-ui,sans-serif;background:var(--bg);color:var(--fg);margin:0;padding:0 16px 80px}
main{max-width:920px;margin:0 auto}h1{font-size:1.4rem}
.rules{background:var(--card);border:1px solid var(--bd);border-radius:10px;padding:12px 16px}
.card{background:var(--card);border:1px solid var(--bd);border-radius:12px;padding:14px 16px;margin:18px 0}
.card.todo{border-color:var(--warn);border-width:2px}
.card.todo h2::after{content:' — incomplète';color:var(--warn);font-weight:700}
.card h2{font-size:1rem;margin:0 0 8px;color:var(--mut)}
.card img{max-width:100%;height:auto;border-radius:8px;display:block;background:#fff}
.alt{font-size:1.05rem}.ctx{color:var(--mut)}.warn{color:var(--warn);font-weight:700}
fieldset{border:1px solid var(--bd);border-radius:8px;margin:10px 0}
fieldset label{display:inline-block;margin:4px 18px 4px 0;cursor:pointer}
.cm input{width:100%;box-sizing:border-box;padding:6px;border:1px solid var(--bd);border-radius:6px;background:var(--bg);color:var(--fg)}
.bar{position:sticky;top:0;background:var(--bg);border-bottom:1px solid var(--bd);padding:8px 0;z-index:2;display:flex;gap:12px;align-items:center;flex-wrap:wrap}
button{font:inherit;padding:8px 14px;border-radius:8px;border:1px solid var(--ac);background:var(--ac);color:#fff;cursor:pointer}
button.sec{background:transparent;color:var(--ac)}
input[type=radio]:focus-visible,button:focus-visible,input[type=text]:focus-visible{outline:3px solid var(--ac);outline-offset:2px}"""

JS = """const KEY=%(key)r, N=%(n)d, MIN=%(min)d;
const st=()=>{try{return JSON.parse(localStorage.getItem(KEY)||"{}")}catch(e){return {}}},
      save=o=>{try{localStorage.setItem(KEY,JSON.stringify(o))}catch(e){}};
function render(){const s=st();let n=0,t=0;
 document.querySelectorAll("article").forEach(a=>{const q=a.dataset.qid,v=(s[q]||{}).v,c=(s[q]||{}).c||"";
  a.querySelectorAll("input[type=radio]").forEach(r=>r.checked=(r.value===v));
  a.querySelector("input[type=text]").value=c;
  const ok=!!v&&c.trim().length>=MIN; if(ok)n++;
  const started=!ok&&(!!v||!!c.trim()); if(started)t++;
  a.classList.toggle("todo",started);});
 document.getElementById("who").value=s._who||"";
 document.getElementById("prog").textContent=n+" / "+N;
 document.getElementById("todo").textContent=(n<N)?("\\u26a0 "+(N-n)+" carte(s) incomplete(s)"+(t?" dont "+t+" entamee(s)":"")):"\\u2713 complet";}
document.addEventListener("change",e=>{const s=st(),t=e.target;
 if(t.id==="who"){s._who=t.value;save(s);return;}
 const a=t.closest("article"); if(!a)return;
 const q=a.dataset.qid; s[q]=s[q]||{};
 if(t.type==="radio")s[q].v=t.value; else s[q].c=t.value;
 save(s);render();});
document.getElementById("exp").onclick=()=>{const s=st();
 const labels=[...document.querySelectorAll("article")].map(a=>{const q=a.dataset.qid;
   return{qid:q,verdict:(s[q]||{}).v||null,comment:(s[q]||{}).c||""}});
 const bad=labels.filter(l=>!l.verdict||l.comment.trim().length<MIN);
 if(bad.length){const el=document.querySelector('article[data-qid="'+bad[0].qid+'"]');
  alert(bad.length+" carte(s) incomplete(s). Il faut un verdict ET une justification d'au moins "+MIN+
        " caracteres sur chacune, verdict favorable compris. Je vous emmene a la premiere.");
  el.scrollIntoView({block:"center"}); return;}
 const b=new Blob([JSON.stringify({sheet:KEY,labeler:s._who||"",exported_at:new Date().toISOString(),labels},null,1)],
   {type:"application/json"});
 const u=URL.createObjectURL(b),l=document.createElement("a");
 l.href=u; l.download="etiquettes-"+KEY+(s._who?"-"+s._who.replace(/[^A-Za-z0-9_-]+/g,"_"):"")+".json";
 l.click(); URL.revokeObjectURL(u);};
document.getElementById("rst").onclick=()=>{if(confirm("Effacer toutes vos reponses ?")){localStorage.removeItem(KEY);render();}};
render();"""


def b64(path, max_side=880):
    try:
        from PIL import Image
    except ImportError:
        sys.exit("Pillow requis : pip install pillow")
    im = Image.open(path).convert("RGB")
    im.thumbnail((max_side, max_side))
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=80)
    return base64.b64encode(buf.getvalue()).decode()


def esc(t):
    return (t.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;"))


def build(items, lot_dir, key, title, rubric):
    cache, cards = {}, []
    for x in items:
        img = x.get("image")
        if img and img not in cache:
            cache[img] = b64(os.path.join(lot_dir, img))
        pic = (f'<img src="data:image/jpeg;base64,{cache[img]}" '
               f'alt="Image à évaluer (voir l\'alternative proposée ci-dessous)">') if img else ""
        cards.append(f"""<article class="card" data-qid="{x['qid']}"><h2>{x['qid']} / {len(items)}</h2>
  {pic}
  <p class="ctx"><strong>Contexte de la page :</strong> {esc(x.get('ctx',''))}</p>
  <p class="alt"><strong>Alternative textuelle proposée :</strong> « {esc(x['alt'])} »</p>
  <fieldset><legend>Cette alternative est-elle pertinente ?</legend>
    <label><input type="radio" name="{x['qid']}" value="pertinent"> Pertinente</label>
    <label><input type="radio" name="{x['qid']}" value="non_pertinent"> Non pertinente</label>
    <label><input type="radio" name="{x['qid']}" value="ne_sait_pas"> Je ne sais pas</label></fieldset>
  <label class="cm"><span>Justification <strong>obligatoire</strong> — si pertinente : ce qui la rend
    suffisante ; si non pertinente : ce qui manque ou ce qui est faux</span>
    <input type="text" name="c_{x['qid']}" maxlength="300" placeholder="une phrase suffit"></label></article>""")
    js = JS % {"key": key, "n": len(items), "min": MIN_JUSTIFICATION}
    return f"""<!doctype html><html lang="fr"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{esc(title)}</title><style>{CSS}</style></head><body><main>
<h1>{esc(title)}</h1>
<div class="rules">{rubric}
<p class="warn">Une justification est obligatoire sur chaque carte, <strong>verdict favorable
compris</strong>. Sans elle, le verdict ne se distingue pas du simple fait d'avoir écrit une
correction : mesuré à phi = &minus;1,000 sur 39 cartes lors d'une campagne précédente, où une
alternative factuellement fausse est passée en « pertinente » sans un mot. L'export est bloqué
tant qu'une carte est incomplète.</p>
<p>Répondez <strong>carte par carte</strong>. Plusieurs cartes peuvent montrer la même image avec
un contexte ou une alternative légèrement différents : c'est voulu. Si vous êtes la seconde
personne, <strong>ne consultez pas</strong> les réponses de la première.</p></div>
<div class="bar" role="region" aria-label="Progression">
<span id="prog" aria-live="polite">0 / {len(items)}</span>
<span id="todo" class="warn" aria-live="polite"></span>
<label>Votre nom ou initiales <input id="who" type="text" maxlength="40"></label>
<button id="exp">Exporter mes étiquettes</button>
<button id="rst" class="sec">Tout effacer</button></div>
{chr(10).join(cards)}</main><script>{js}</script></body></html>"""


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("items", help="JSON des cartes : [{qid, image, ctx, alt}, ...]")
    ap.add_argument("--out", required=True)
    ap.add_argument("--title", default="Étiquetage — pertinence des alternatives textuelles")
    ap.add_argument("--key", default="", help="clé de stockage navigateur (défaut : nom du fichier de sortie)")
    ap.add_argument("--rubric", default="", help="fichier HTML de la règle à afficher en tête")
    a = ap.parse_args()

    items = json.load(open(a.items, encoding="utf-8"))
    for x in items:
        if "qid" not in x or "alt" not in x:
            sys.exit(f"carte sans qid ou sans alt : {x}")
    if len({x["qid"] for x in items}) != len(items):
        sys.exit("identifiants de cartes dupliqués")
    key = a.key or os.path.splitext(os.path.basename(a.out))[0]
    rubric = open(a.rubric, encoding="utf-8").read() if a.rubric else "<p>Jugez chaque alternative selon la règle en vigueur.</p>"
    html = build(items, os.path.dirname(os.path.abspath(a.items)), key, a.title, rubric)

    # La clé de correction ne doit jamais atteindre la page.
    for leak in ("pred", "\"axis\"", "anchor", "key.json"):
        if leak in html:
            sys.exit(f"fuite possible de la clé de correction dans la page : {leak!r}")
    open(a.out, "w", encoding="utf-8").write(html)
    print(f"{a.out} — {len(items)} cartes, clé de stockage {key!r}, "
          f"justification de {MIN_JUSTIFICATION} caractères minimum")


if __name__ == "__main__":
    main()
