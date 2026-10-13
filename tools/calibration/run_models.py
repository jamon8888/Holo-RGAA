"""Exécute un modèle sur un lot de calibration.

Protocole : `docs/specs/calibration-protocol.md`. Sortie en JSON Lines, à dépouiller avec
`score.py`.

Deux garde-fous, et ils sont là pour de bonnes raisons :

  * le backend `openai` **envoie les images d'un site audité à un service externe** : il
    refuse de partir sans `--allow-external`, et annonce l'hôte destinataire sur stderr ;
  * la clé n'est jamais passée en argument, seulement par **nom de variable
    d'environnement** (`--key-env`), pour qu'elle n'apparaisse ni dans `ps` ni dans un
    historique de shell. Elle n'est jamais journalisée.

La politique d'envoi est celle du §2.7 de `human-criteria-automation.md` : API désactivée
par défaut, activable par audit, jamais de page authentifiée vers un service externe.
"""
import argparse, base64, json, os, sys, time, urllib.error, urllib.request
HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "out"); ITEMS_FILE, KEY_FILE = "items.json", "key.json"

RUBRIC = """Tu évalues l'alternative textuelle d'une image, comme le ferait un auditeur RGAA (critère 1.3).
- PERTINENTE : une personne qui ne voit pas l'image obtient, dans ce contexte, l'information que l'image apporte — sans erreur, sans promotion, sans mots inutiles.
- NON PERTINENTE : trop vague, fausse, redondante, hors sujet, ou elle ne dit pas ce que l'image dit dans ce contexte.
- INCERTAIN : en cas de vrai doute.
Toutes les images sont considérées comme informatives dans leur contexte."""
SCHEMA = {"type": "object", "properties": {
    "verdict": {"type": "string", "enum": ["pertinent", "non_pertinent", "incertain"]},
    "justification": {"type": "string"}, "confiance": {"type": "number"}},
    "required": ["verdict", "justification", "confiance"]}

SCHEMA_V2 = {"type": "object", "properties": {
    "ce_que_dit_le_texte": {"type": "string"},
    "information_que_l_image_apporte_ici": {"type": "string"},
    "le_texte_la_transmet": {"type": "string", "enum": ["oui", "en_partie", "non"]},
    "justification": {"type": "string"},
    "verdict": {"type": "string", "enum": ["pertinent", "non_pertinent", "incertain"]},
    "confiance": {"type": "number"}},
    "required": ["ce_que_dit_le_texte", "information_que_l_image_apporte_ici", "le_texte_la_transmet", "justification", "verdict", "confiance"]}

def prompt_v2(item):
    return (f"{RUBRIC}\n\nContexte de la page : {item['ctx']}\nAlternative textuelle proposée : « {item['alt']} »\n\n"
            "IMPORTANT : tu juges le TEXTE de l'alternative, pas l'image. L'image est ce qu'elle est ; la question est de savoir si CE TEXTE, lu seul par "
            "quelqu'un qui ne voit pas l'image, lui transmet l'information que l'image apporte dans ce contexte.\n"
            "Procède dans l'ordre : (1) résume ce que dit le texte, en ne regardant que lui ; (2) dis quelle information l'image apporte dans ce contexte ; "
            "(3) indique si le texte la transmet (oui / en_partie / non) ; (4) conclus.\n"
            "Réponds uniquement par un objet JSON avec les champs : ce_que_dit_le_texte, information_que_l_image_apporte_ici, le_texte_la_transmet, justification, verdict, confiance.")

RUBRIC_V3 = """Tu évalues l'alternative textuelle d'une image, comme le ferait un auditeur RGAA (critère 1.3).
Une alternative est PERTINENTE si, lue seule par quelqu'un qui ne voit pas l'image :
1. elle décrit ce que l'image montre, de façon concise ;
2. elle nomme le sujet précis quand le contexte l'identifie (lieu, organisation, espèce, produit) ;
3. pour un graphique ou un schéma : elle donne les valeurs ou la tendance clé, SAUF si le texte voisin les donne déjà (un titre court suffit alors) ;
4. elle ne reprend PAS le contenu du texte voisin (prix, dates, durée, thème de la page) ;
5. elle ne contient ni jugement de valeur, ni promotion, ni « photo de », ni liste de mots-clés.
Elle est NON PERTINENTE si l'une de ces conditions manque, ou si elle est fausse, hors sujet, un nom de fichier, ou trop vague pour identifier le sujet.
INCERTAIN : en cas de vrai doute. Toutes les images sont considérées comme informatives dans leur contexte."""

def prompt_v3(item):
    return (f"{RUBRIC_V3}\n\nContexte de la page : {item['ctx']}\nAlternative textuelle proposée : « {item['alt']} »\n\n"
            "IMPORTANT : tu juges le TEXTE de l'alternative, pas l'image. L'image est ce qu'elle est.\n"
            "Procède dans l'ordre : (1) résume ce que dit le texte, en ne regardant que lui ; (2) dis ce que l'image montre et ce que le contexte permet d'identifier ; "
            "(3) indique si le texte transmet cette information en respectant les cinq conditions (oui / en_partie / non) ; (4) conclus.\n"
            "Réponds uniquement par un objet JSON avec les champs : ce_que_dit_le_texte, information_que_l_image_apporte_ici, le_texte_la_transmet, justification, verdict, confiance.")

RUBRIC_V4 = """Tu évalues l'alternative textuelle d'une image, comme le ferait un auditeur RGAA (critère 1.3).
Une alternative est PERTINENTE lorsqu'elle est à la fois COHÉRENTE et COMPLÈTE au regard de DEUX sources :
  (A) ce que l'image montre, et
  (B) ce que la page dit de l'image (contexte).
Il lui manque l'une ou l'autre et elle est NON PERTINENTE.

(A) Complétude vis-à-vis de l'image :
  - pour une image de données — graphique, tableau de chiffres, organigramme, frise, plan coté — l'alternative doit restituer TOUTES les grandeurs
    (chaque catégorie et sa valeur), et non la seule tête de classement ni le seul titre. Cette exigence vaut MÊME SI les mêmes valeurs figurent
    déjà dans le texte voisin.
  - pour une autre image, l'alternative doit dire ce qui est montré, sans se tromper.

(B) Cohérence vis-à-vis du contexte :
  - l'alternative doit reprendre le sujet que la page donne à l'image : légende d'une galerie, raison d'être de l'illustration, activité de
    l'organisation derrière un logo. Cette exigence vaut MÊME SI ces mots figurent déjà à côté de l'image.
  - une alternative qui décrit fidèlement ce qui est visible mais ne reprend rien de ce que la page dit de l'image est NON PERTINENTE,
    quelle que soit la qualité de la description.

Est également NON PERTINENTE une alternative fausse, hors sujet, un nom de fichier, un mot générique (« photo », « image »), un jugement de
valeur, une accroche promotionnelle, ou une liste de mots-clés.
INCERTAIN : en cas de vrai doute. Toutes les images sont considérées comme informatives dans leur contexte."""

def prompt_v4(item):
    return (f"{RUBRIC_V4}\n\nContexte de la page : {item['ctx']}\nAlternative textuelle proposée : « {item['alt']} »\n\n"
            "IMPORTANT : tu juges le TEXTE de l'alternative, pas l'image. L'image est ce qu'elle est.\n"
            "Procède dans l'ordre : (1) résume ce que dit le texte, en ne regardant que lui ; (2) dis ce que l'image montre, et ce que la page dit "
            "de l'image ; (3) indique si le texte est complet vis-à-vis de l'image ET cohérent avec le contexte (oui / en_partie / non) ; (4) conclus.\n"
            "Réponds uniquement par un objet JSON avec les champs : ce_que_dit_le_texte, information_que_l_image_apporte_ici, le_texte_la_transmet, justification, verdict, confiance.")

RUBRIC_V5 = """Tu évalues l'alternative textuelle d'une image, comme le ferait un auditeur RGAA (critère 1.3).

Une alternative est PERTINENTE seulement si DEUX conditions sont remplies EN MÊME TEMPS. Ce sont des conditions
CUMULATIVES, pas des options : si l'une manque, le verdict est NON PERTINENT, même si l'autre est parfaitement remplie.

CONDITION A — complétude vis-à-vis de l'image
  - Si l'image est une IMAGE DE DONNÉES (graphique, camembert, tableau de chiffres, organigramme, frise, plan coté) :
    l'alternative doit restituer TOUTES les grandeurs, chaque catégorie et sa valeur. Ni la seule tête de classement,
    ni le seul titre. Cette exigence vaut MÊME SI les mêmes valeurs figurent déjà dans le texte voisin.
  - Si l'image n'est PAS une image de données (photo, logo, bannière) : il suffit que l'alternative dise sans se
    tromper ce qui est montré. La restitution chiffrée est alors SANS OBJET.

CONDITION B — cohérence avec le contexte
  - L'alternative doit reprendre le sujet que la page donne à l'image : légende de la galerie, raison d'être de
    l'illustration, activité de l'organisation derrière un logo. Cette exigence vaut MÊME SI ces mots figurent déjà
    à côté de l'image.
  - ATTENTION, C'EST LE PIÈGE LE PLUS FRÉQUENT : une PHOTO ou un LOGO n'est PAS dispensé de la condition B.
    Le fait qu'une image ne soit pas une image de données allège la condition A, et ELLE SEULE. Une photo décrite
    avec exactitude mais qui ne reprend rien de ce que la page dit de l'image échoue à la condition B, donc elle est
    NON PERTINENTE — quelle que soit la qualité de la description. Ne te laisse pas convaincre par une description
    fidèle : demande-toi toujours, séparément, si le sujet donné par la page s'y retrouve.

Est aussi NON PERTINENTE une alternative fausse, hors sujet, un nom de fichier, un mot générique (« photo », « image »),
un jugement de valeur, une accroche promotionnelle, ou une liste de mots-clés.
INCERTAIN : en cas de vrai doute. Toutes les images sont considérées comme informatives dans leur contexte."""

def prompt_v5(item):
    return (f"{RUBRIC_V5}\n\nContexte de la page : {item['ctx']}\nAlternative textuelle proposée : « {item['alt']} »\n\n"
            "IMPORTANT : tu juges le TEXTE de l'alternative, pas l'image. L'image est ce qu'elle est.\n"
            "Procède dans cet ordre, sans sauter d'étape :\n"
            " (1) ce_que_dit_le_texte : résume l'alternative en ne regardant qu'elle ;\n"
            " (2) ce_que_montre_l_image : décris l'image, et dis si c'est une image de données ;\n"
            " (3) ce_que_la_page_dit_de_l_image : cite ce que le contexte dit du sujet de l'image ;\n"
            " (4) complet_vis_a_vis_de_l_image : « oui », « non », ou « sans_objet » si l'image n'est pas une image de données ;\n"
            " (5) coherent_avec_le_contexte : « oui » ou « non » — réponds à CETTE question seule, sans tenir compte de l'étape 4 ;\n"
            " (6) verdict : « pertinent » UNIQUEMENT si l'étape 4 vaut oui ou sans_objet ET que l'étape 5 vaut oui. Sinon « non_pertinent ».\n"
            "Réponds uniquement par un objet JSON avec les champs : ce_que_dit_le_texte, ce_que_montre_l_image, "
            "ce_que_la_page_dit_de_l_image, complet_vis_a_vis_de_l_image, coherent_avec_le_contexte, justification, verdict, confiance.")

PROMPTS = {"v1": None, "v2": None, "v3": None, "v4": None, "v5": None}   # rempli plus bas (les fonctions v1/v2 sont définies après)

def prompt(item):
    return (f"{RUBRIC}\n\nContexte de la page : {item['ctx']}\nAlternative textuelle proposée : « {item['alt']} »\n\n"
            "Regarde l'image, puis réponds uniquement par un objet JSON : "
            '{"verdict": "pertinent" | "non_pertinent" | "incertain", "justification": "une phrase", "confiance": 0 à 1}.')

PROMPTS.update({"v1": prompt, "v2": prompt_v2, "v3": prompt_v3, "v4": prompt_v4, "v5": prompt_v5})

def b64(path): return base64.b64encode(open(path, "rb").read()).decode()

def image_b64(item):
    """Image de la carte, ou None pour une famille sans image (titre de page, légende...).

    `image` est un chemin relatif au répertoire du lot, avec extension — le même contrat que
    `sheet.py`. L'ancien format (nom nu, `img/<nom>.jpg` implicite) reste lu pour rejouer les
    lots du prototype.
    """
    rel = item.get("image")
    if not rel:
        return None
    path = os.path.join(OUT, rel)
    if not os.path.exists(path):
        path = os.path.join(OUT, "img", rel + ".jpg")
    return b64(path)

def call_ollama(model, item, ver='v1', host="http://127.0.0.1:11434", timeout=900):
    body = {"model": model, "stream": False, "think": False, "format": (SCHEMA if ver == "v1" else SCHEMA_V2),
            "options": {"temperature": 0, "num_predict": 500},
            "messages": [dict({"role": "user", "content": PROMPTS[ver](item)}, **({"images": [image_b64(item)]} if image_b64(item) else {}))]}
    req = urllib.request.Request(host + "/api/chat", json.dumps(body).encode(), {"Content-Type": "application/json"})
    r = json.load(urllib.request.urlopen(req, timeout=timeout))
    return r["message"]["content"]

LAST_USAGE = {}

def call_openai(base_url, model, key_env, item, ver='v1', max_tokens=1200, timeout=240):
    key = os.environ.get(key_env)
    if not key: raise SystemExit(f"variable d'environnement {key_env} absente")
    content = [{"type": "text", "text": PROMPTS[ver](item)}]
    img = image_b64(item)
    if img:
        content.append({"type": "image_url", "image_url": {"url": "data:image/jpeg;base64," + img}})
    msg = [{"role": "user", "content": content}]
    url = base_url.rstrip("/") + "/chat/completions"
    hdr = {"Content-Type": "application/json", "Authorization": "Bearer " + key}
    for use_fmt in (True, False):                       # certains modèles/passerelles refusent response_format
        body = {"model": model, "temperature": 0, "max_tokens": max_tokens, "messages": msg}
        if use_fmt: body["response_format"] = {"type": "json_object"}
        try:
            r = json.load(urllib.request.urlopen(urllib.request.Request(url, json.dumps(body).encode(), hdr), timeout=timeout))
            break
        except urllib.error.HTTPError as e:
            detail = e.read().decode("utf8", "replace")[:300]
            if use_fmt and e.code in (400, 422) and "response_format" in detail.lower(): continue
            raise RuntimeError(f"HTTP {e.code}: {detail}")
    LAST_USAGE.clear(); LAST_USAGE.update(r.get("usage") or {})
    m = r["choices"][0]["message"]
    return m.get("content") or ""

import re, unicodedata
CANON = {"pertinent": "pertinent", "pertinente": "pertinent", "non_pertinent": "non_pertinent", "non_pertinente": "non_pertinent",
         "incertain": "incertain", "incertaine": "incertain"}

def norm_verdict(v):
    """Tolère casse, accents, espaces et tirets ; ne devine rien d'autre (« partiellement pertinente » reste invalide)."""
    if not isinstance(v, str): return None
    t = unicodedata.normalize("NFD", v.lower()); t = "".join(c for c in t if unicodedata.category(c) != "Mn")
    return CANON.get(re.sub(r"[^a-z]+", "_", t).strip("_"))

def parse(raw):
    """Renvoie aussi les champs de conditions de la v5 : le ET mecanique doit etre calculable
    sans dependre de `raw`, qui est tronque au stockage (bug corrige : les booleens arrivaient apres la coupe)."""
    try:
        s = raw[raw.index("{"): raw.rindex("}") + 1]; j = json.loads(s)
        vb = j.get("verdict"); v = norm_verdict(vb) or "invalide"
        cond = {k: str(j[k]) for k in ("complet_vis_a_vis_de_l_image", "coherent_avec_le_contexte") if k in j}
        return v, str(j.get("justification", ""))[:300], j.get("confiance"), vb, cond
    except Exception:
        return "invalide", raw[:200], None, None, {}

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--backend", choices=["ollama", "openai"], required=True)
    ap.add_argument("--model", required=True)
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--base-url", default="")
    ap.add_argument("--key-env", default="")
    ap.add_argument("--allow-external", action="store_true", help="OBLIGATOIRE pour le backend openai : confirme l'envoi d'images à l'extérieur")
    ap.add_argument("--tag", default="")
    ap.add_argument("--lot-dir", required=True, help="répertoire du lot : items.json, key.json, img/")
    ap.add_argument("--prompt", default="v5", choices=["v1", "v2", "v3", "v4", "v5"],
                    help="v5 porte le critere en vigueur (ADR 0001). v1 a v3 sont conservees "
                         "pour l'historique : elles enoncent une regle que l'etiquetage a infirmee")
    ap.add_argument("--prompt-file", help="consigne propre a la famille (remplace --prompt). Marqueurs : "
                    "{label} {test} {question} {ctx} {alt}. Voir tools/calibration/prompts/")
    ap.add_argument("--max-tokens", type=int, default=1200, help="plafond de jetons de sortie (backend openai) ; consigné dans chaque résultat")
    a = ap.parse_args()
    if a.backend == "openai":
        if not a.allow_external:
            raise SystemExit("REFUS : le backend openai envoie des images à un service externe. Relancer avec --allow-external après accord explicite.")
        host = a.base_url.split("/")[2] if "//" in a.base_url else a.base_url
        print(f"[ATTENTION] envoi d'images vers : {host}", file=sys.stderr)
    OUT = os.path.abspath(a.lot_dir)
    ITEMS_FILE, KEY_FILE = "items.json", "key.json"
    if a.prompt_file:
        template = open(a.prompt_file, encoding="utf-8").read()
        def _from_file(item):
            out = template
            for k in ("label", "test", "question", "ctx", "alt"):
                out = out.replace("{" + k + "}", str(item.get(k, "")))
            return out
        PROMPTS["file"] = _from_file
        a.prompt = "file"
    items = json.load(open(os.path.join(OUT, ITEMS_FILE)))
    key = json.load(open(os.path.join(OUT, KEY_FILE)))
    sheet_ids = {v.get("item", v.get("id")) for v in key.values()}   # lot B/C : "item" ; lot D2 : "id"
    del sheet_ids  # la selection par sous-ensemble appartenait aux lots A/B du prototype
    if a.limit: items = items[: a.limit]
    name = (a.tag or a.model).replace("/", "_").replace(":", "_") + ("" if a.prompt == "v1" else "_" + a.prompt)
    path = os.path.join(OUT, f"results_{a.backend}_{name}.jsonl")
    done = {json.loads(l)["id"] for l in open(path)} if os.path.exists(path) else set()
    todo = [x for x in items if x["id"] not in done]
    print(f"{len(todo)} à faire ({len(done)} déjà faits) -> {path}", flush=True)
    with open(path, "a") as f:
        for n, x in enumerate(todo, 1):
            t = time.perf_counter()
            try:
                raw = call_ollama(a.model, x, a.prompt) if a.backend == "ollama" else call_openai(a.base_url, a.model, a.key_env, x, a.prompt, a.max_tokens)
            except Exception as e:
                raw = f"ERREUR: {type(e).__name__}: {str(e)[:150]}"
            dt = time.perf_counter() - t
            v, why, conf, vb, cond = parse(raw)
            f.write(json.dumps(dict(id=x["id"], verdict=v, verdict_brut=vb, justification=why, confiance=conf, conditions=cond, s=round(dt, 1), usage=dict(LAST_USAGE), max_tokens=(a.max_tokens if a.backend == 'openai' else None), raw=raw[:2000]), ensure_ascii=False) + "\n"); f.flush(); LAST_USAGE.clear()
            print(f"[{n}/{len(todo)}] {x['id']} {v:14s} {dt:5.1f}s  {why[:70]}", flush=True)
