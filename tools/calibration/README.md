# Outillage de calibration

Mesure l'accord d'un modèle avec un étiquetage humain sur une **famille de preuve**
(`CONTEXT.md`). Le protocole, les seuils et le budget sont dans
[`docs/specs/calibration-protocol.md`](../../docs/specs/calibration-protocol.md) ; ces
scripts l'appliquent.

Ce n'est pas du code de production : rien ici n'est appelé par un audit. C'est l'outillage
qui permet d'affirmer, chiffres en main, qu'un test est *Assisté calibré* plutôt que
*non calibré*.

## Chaîne

```bash
# 1. feuille d'étiquetage, à remplir par DEUX personnes en aveugle l'une de l'autre
python3 sheet.py lot/items.json --out lot/etiquetage.html --title "Lot E — descriptions détaillées"
python3 -m http.server 8766 --bind 127.0.0.1 --directory lot   # Chrome bloque le JS d'un file://

# 2. le même lot, passé au modèle
export MA_CLE=...        # jamais en argument : seul le NOM de la variable est passé
python3 run_models.py --backend openai --model <modèle> \
    --base-url https://exemple/v1 --key-env MA_CLE --allow-external \
    --lot-dir lot --prompt v5 --max-tokens 3000

# 3. dépouillement
python3 score.py --key lot/key.json --labels etiquettes-...json \
    --labels2 etiquettes-...json --model lot/results_*.jsonl
```

## Ce que chaque script garantit

**`sheet.py`** applique la contrainte du §2 du protocole : **justification obligatoire sur
chaque carte, verdict favorable compris**, export bloqué tant qu'il en manque une. Il refuse
aussi de générer la page si un champ de la clé de correction s'y est glissé.

**`run_models.py`** refuse de partir sur le backend externe sans `--allow-external`, annonce
l'hôte destinataire, et ne lit la clé que par nom de variable d'environnement — elle
n'apparaît ni dans `ps`, ni dans un historique de shell, ni dans les résultats. Les consignes
`v1` à `v3` sont conservées pour l'historique : elles énoncent une règle que l'étiquetage a
infirmée. `v5` est celle en vigueur.

**`score.py`** affiche trois contrôles **avant** l'accord, parce qu'un accord mesuré sur une
passe viciée ne vaut rien :

1. **collinéarité** `phi(champ rempli, verdict)`. À −1, le verdict ne porte aucune
   information propre. Le protocole rend ce chiffre non mesurable par construction, ce qui
   est l'état attendu ; s'il est calculable, c'est que des cartes tranchées n'ont pas été
   justifiées et la passe est inexploitable ;
2. **ancres**, dont une factuellement fausse. Moins de 4/4 et la passe est suspecte, quel que
   soit l'accord ;
3. **repère trivial**, le score du meilleur verdict constant.

Puis l'accord, le kappa de Cohen, les intervalles de Wilson, les faux « pertinent » rapportés
aux rejets humains, et le verdict du **seuil d'admission** du §4.

## Format d'un lot

```
lot/
├── items.json   [{"qid": "e01", "image": "img/chart.jpg", "ctx": "...", "alt": "..."}, ...]
├── key.json     {"e01": {"id": "E001", "pred": "P"|"N", "kind": "...", "anchor": false}, ...}
└── img/
```

`items.json` alimente la feuille ; `key.json` ne doit **jamais** y entrer. `pred` est
l'application attendue de la règle, pas une vérité : c'est l'étiquetage humain qui fait
référence, et un écart entre les deux est une information sur la règle.

## Mesure de référence

Famille « alternative d'image », `swift-1.5-27b`, 39 cartes, température 0 :
**33/39 = 85 %**, kappa 0,68, ancres 4/4, repère trivial 67 %, **5 faux « pertinent » sur
26 rejets = 19 %** — donc **refusé** par le seuil des 10 %, et admis en *Assisté* seulement.

Exécuter `score.py` sur ces données doit redonner ces chiffres.
