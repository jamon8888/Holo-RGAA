# Moteur d'évaluation et de préétiquetage (`rig`, sans boucle d'agent)

Statut : **proposition, décisions de conception prises (#358)**. Rien n'est implémenté. Carte : #330. Prolonge le protocole de
calibration (`docs/specs/calibration-protocol.md`) ; n'en change aucune règle.

## 1. Ce que c'est, et ce que ce n'est pas

Une chaîne **rejouable et tracée** (carte → juges → tri → score), écrite en Rust sur `rig-core`.
Le pipeline, les règles de tri et les seuils sont déterministes ; les verdicts d'un juge LLM ne le
sont pas (environ une carte de bruit sur 39 à température 0, protocole §4). Elle automatise trois
tâches du protocole :

1. **étiquettes par construction** : on part d'un texte correct, on y injecte un défaut connu ;
   la présence du défaut est certaine sans jugement (pas le verdict de l'auditeur, §6) ;
2. **tri par désaccord** : plusieurs juges préétiquettent ; l'humain voit les cartes où ils
   divergent, où l'un d'eux est indécis ou ne répond pas, plus un échantillon tiré au sort des
   cartes d'accord ;
3. **évaluation rejouable** : un juge est rejoué sur un lot gelé, et son score est recalculé.

**Ce n'est pas** un agent : aucune boucle outil/observation, aucune décision du système sur la
suite. **Ce n'est pas** un remplaçant de la référence humaine : un juge LLM ne se valide pas par
un autre juge LLM (protocole §4 bis).

## 2. Faits vérifiés dans `rig-core` 0.42.0 (sources du registre cargo)

| besoin | constat | où |
|---|---|---|
| image en entrée | `UserContent::Image` avec `DocumentSourceKind::Base64` | `completion/message.rs` |
| Ollama | images **base64 uniquement** (une URL est refusée) | `providers/ollama.rs` |
| OpenAI et compatibles | `image_url` portant un data URI | `providers/openai/completion` |
| température | `CompletionRequest.temperature: Option<f64>` | `completion/request.rs:740` |
| paramètres libres (graine, etc.) | `additional_params: Option<Value>` | `completion/request.rs:746` |
| sortie structurée native | `output_schema` (schemars) | `completion/request.rs:749` |
| extraction typée | `Extractor` | crate `rig-agent`, `extractor.rs` |
| URL de base configurable | `.base_url(..)`, déjà utilisé par `rgaa-agent` | `rgaa-rs/crates/rgaa-agent/src/agent.rs:346` |

## 3. Ce qui n'est PAS vérifié (à établir par le prototype, §8)

- l'envoi réel d'une image à Ollama et à MyIA **par `rig`** (lu dans le code, jamais exécuté) ;
- que la graine et `temperature = 0` aboutissent réellement au fournisseur ;
- que la sortie structurée native est honorée par chaque backend (sinon : repli sur le JSON
  extrait, comme `run_models.py`) ;
- le coût en appels du tri par désaccord sur un lot réel.

Chacun a son contrôle au §8, étape 0.

## 3 bis. Candidats juges (non mesurés chez nous, sauf mention)

| modèle | annoncé | rôle visé | accord d'envoi |
|---|---|---|---|
| `swift-1.5-27b` (MyIA) | mesuré : D2 85 %, kappa 0,68, 19 % de faux « pertinent » | juge vision ; échoue le seuil §4 | par exécution |
| `nemotron-3.5-super-vl` (NIM) | multimodal (`candidates.toml`, rôle *reasoning*) | juge vision, à mesurer | permanent, cartes synthétiques |
| `mistral-large-4` (NIM) | `candidates.toml`, rôle *general* | juge, à mesurer | permanent, cartes synthétiques |
| `glm-5.3-flash` (NIM) | multimodal natif (`candidates.toml`, rôle *fast*) | juge vision, à mesurer | permanent, cartes synthétiques |
| DeepSeek-V4.1-Flash (NIM) | texte et image en entrée | juge vision, à mesurer ; absent de `candidates.toml` | par exécution |
| DeepSeek-V4-Flash (NIM) | texte seul (aucune mention d'image) | générateur de défauts sur du texte | par exécution |
| `gemma4:e2b` (local) | mesuré : D2 56 %, sous le repère trivial | **témoin** du pouvoir discriminant (`[witness]` de `candidates.toml`) | local, rien ne sort |
| Kumo Tabular (NIM) | classe des données tabulaires en contexte | pas un juge ; agrégateur de votes envisageable **après** les mesures, non retenu d'avance | par exécution |

La liste de référence est `tools/calibration/candidates.toml` ; le moteur la lit, il n'en tient pas
une seconde. Une fiche dit qu'une image est acceptée, pas que le modèle juge bien nos tableaux et nos
libellés : l'admission de chaque candidat se mesure (§8).

**Accord d'envoi.** L'accord permanent du 2026-11-04 (`candidates.toml`, `judge-promotion.yml`)
couvre les trois modèles NIM de `candidates.toml` et des cartes **synthétiques** uniquement ; le moteur
le reprend tel quel, y compris pour le job mensuel. Tout autre modèle externe (MyIA, DeepSeek, Kumo)
exige l'accord de l'utilisateur pour l'exécution. Aucune vraie capture de site audité ne part vers un
candidat, accord permanent ou non.

## 4. Architecture proposée

Un nouveau crate `rgaa-eval` sur `rig` (décidé, §7) avec cinq briques indépendantes :

- **`Judge`** : un modèle, une consigne, une température fixe ; entrée = une carte (image
  optionnelle, contexte, texte, test) ; sortie typée `{verdict, cohérence, complétude, justification}`,
  où `verdict ∈ {pertinent, non_pertinent, indécis}`. Une réponse illisible ou absente est consignée
  comme **manquante**, jamais devinée. Deux backends : Ollama et compatible OpenAI (MyIA, NIM).
- **`Lot`** : lit, aux formats de `tools/calibration/` :
  - `items.json` (cartes) et `key.json` (clé de construction : `pred`, `anchor`, `kind`) ;
  - l'**export d'étiquettes humaines** (`{labeler, labels: [{qid, verdict, comment}]}`, celui de
    `sheet.py`), et le second étiqueteur s'il existe ;
  - `recheck/key.json` et l'export de re-vérification : pour une carte qui bascule, le verdict
    retenu est celui de la **seconde passe** (protocole §3).

  Il produit la référence consolidée et son **empreinte** (version du jeu humain). Sans export humain,
  le moteur peut rejouer un juge mais ne calcule ni score ni admission.
- **`Score`** : sur la référence consolidée, les mêmes sorties que `score.py` :
  - cartes tranchées, cartes sans justification ;
  - collinéarité : phi(champ rempli, verdict) et phi(reproche, verdict), ce dernier indicatif ;
  - ancres de l'**étiqueteur** (verdict humain contre la clé) ;
  - répétition des justifications favorables (suspecte au-delà de 30 %) ;
  - repère trivial, avec IC de Wilson ;
  - accord inter-étiqueteurs et kappa, si un second export est fourni ;
  - modèle contre humain : accord et IC de Wilson, kappa, faux « pertinent » / rejets humains,
    faux « non pertinent » / acceptations humaines, cartes sans résultat du modèle ;
  - les quatre conditions de `score.py` : ancres, faux « pertinent » ≤ 10 %, faux « non pertinent »
    ≤ 25 %, IC du modèle disjoint du repère trivial.

  S'y ajoutent, pour l'admission (§5) : les **ancres du juge** (toutes justes : le §4 vise les ancres
  du modèle, alors que `score.py` ne vérifie que celles de l'étiqueteur), la **couverture** (cartes
  avec un verdict valide / cartes tranchées), la taille du lot (≥ 36, comme `promote.py`) et le
  **pouvoir discriminant** contre le témoin. La parité avec `score.py` se vérifie au §8.
- **`Tri`** : règles écrites, sans modèle. Une carte est en **accord** seulement si tous les juges
  rendent le même verdict valide, sans `indécis` ni réponse manquante ; tout le reste va à l'humain.
  Parmi les accords, un échantillon est **tiré au sort** (graine consignée, comme `recheck.py`) ; les
  cartes tirées sont conservées avec leur verdict humain pour publier l'erreur résiduelle (§4 bis,
  étape 4). Pas de vote majoritaire : un désaccord est arbitré, pas tranché par le nombre.
- **`Mutation`** : générateurs de défauts connus (valeur changée, détail supprimé, nom d'un autre
  élément, texte générique, consigne au lieu d'un libellé). Chaque carte produite porte le défaut
  injecté, pas un verdict d'auditeur (§6).

Format d'échange : les JSON existants, pour que `sheet.py` et `score.py` restent utilisables.

## 5. Garde-fous, à coder dès le premier jour

1. **Aucun envoi externe sans drapeau explicite** (équivalent de `--allow-external`) ; le moteur
   refuse sinon, il n'avertit pas. Hors accord permanent (§3 bis), le drapeau ne suffit pas sans
   l'accord de l'utilisateur pour l'exécution.
2. **Clé d'API lue par nom de variable d'environnement**, jamais écrite ni journalisée.
3. **Admission.** Un juge n'est admis que s'il passe le seuil du protocole §4 (pouvoir discriminant
   compris) sur le jeu humain gelé, plus les contrôles ajoutés par `Score` (§4 de cette spec). Une admission vaut pour un
   quadruplet **famille × version du jeu humain × modèle × empreinte de consigne** ; changer l'un des
   quatre la perd.
4. **Deux modes.** *Évaluation* : rejouer n'importe quel modèle, admis ou non, sur un lot étiqueté
   pour le scorer (c'est ce que fait le prototype avec `swift` et `gemma4:e2b`). *Préétiquetage* de
   cartes nouvelles : refusé avec un juge non admis pour ce quadruplet. Le refus porte sur le second
   mode seulement.
5. **Trace par appel** : modèle, version, consigne (empreinte), carte, paramètres envoyés, réponse
   brute tronquée, mode d'extraction (natif ou repli JSON).
6. **Pouvoir discriminant**, comme `promote.py` : le témoin (`gemma4:e2b`) est mesuré sur le même lot
   et la même référence que le candidat ; le candidat n'est retenu que si la borne basse de son IC
   dépasse la borne haute de celui du témoin. Le **comparateur fort est donc le candidat lui-même**.
   Sans témoin mesuré, aucune admission. Le moteur publie ce test.

## 6. Limites assumées

- Les étiquettes par construction établissent la **présence du défaut injecté**, pas le verdict de
  l'auditeur. Exemple : « détail supprimé » produit à coup sûr une alternative incomplète, mais l'ADR
  0001 montre que le rejet d'une alternative « vraie mais incomplète » dépend de ce qui manque et du
  contexte (`D007`, titre seul, rejetée), et le protocole §3 montre des cartes incomplètes validées
  à la première passe puis rejetées à la seconde.
  Ce verdict reste humain.
- Un accord élevé entre juges sur des cas faciles ne prouve rien (lot G : 100 % contre 92 %).
- Une erreur commune à tous les juges est invisible au tri par désaccord (protocole §4 : `Opus` et
  `gemma4:e2b` se trompent sur les mêmes quatre cartes) ; seul l'échantillon tiré au sort la voit.
- Le critère d'étiquetage est défini par l'humain et peut évoluer ; le moteur ne le détecte pas.
- La référence humaine ne disparaît pas : elle se réduit à un **jeu gelé** par famille, plus les
  cartes arbitrées. La taille de ce jeu est une hypothèse à mesurer, non un résultat.

## 7. Décisions (ticket de grilling #358)

Prises avec l'utilisateur : Q1, Q3 et Q4 le 19/10/2026 (le principe est écrit au §4 bis du
protocole), Q2 et Q5 le 19/11/2026. Principe commun : **un modèle juge ne devient jamais la
référence** ; il réduit le travail humain.

1. **Seuil d'admission d'un juge.** Celui du protocole §4, tel quel. Il comprend déjà le **pouvoir
   discriminant** : un lot où le témoin ne tombe pas nettement sous le candidat (intervalles
   disjoints) est déclaré non discriminant, et aucune admission n'y compte. C'est le cas du lot G
   (100 % contre 92 %) : sans cette ligne, un modèle de 2 Md y passerait. Le taux d'erreur du juge
   est publié. Aucun juge n'est promu aujourd'hui (§4 bis) ; le « admis en *Assisté* » de `swift` au
   tableau du §4 désigne son niveau dans le produit d'audit, pas une admission comme juge : il échoue
   le seuil (19 % de faux « pertinent »).
2. **Emplacement du code.** Nouveau crate **`rgaa-eval`** sur `rig`, avec un seul chemin d'appel
   aux modèles pour l'éval ; `rgaa-holo` reste inchangé.
3. **Jeu humain gelé par famille.** Le lot calibré lui-même (≥ 36 cartes) plus ses cartes de
   re-contrôle ; pas de nombre plus petit sans mesure. C'est la seule référence, à condition de
   satisfaire les contrôles du protocole §3 (ancres, repère trivial ≤ 60 %, re-vérification ≥ 12
   cartes équilibrée, test-retest ≥ 80 %, deux étiqueteurs en aveugle — réserve ouverte sur le
   second, ADR 0001).
4. **Juges et relecture.** Plusieurs juges (cible : trois, de fournisseurs différents) ; l'humain
   arbitre toutes les cartes en désaccord plus un échantillon tiré au sort des cartes d'accord.
5. **Étiquettes par construction.** **Complément** du jeu humain, **jamais référence** de
   calibration : elles établissent la présence d'un défaut connu, pas le critère de l'auditeur (§6).

## 8. Prototype (critère d'acceptation)

Un binaire qui lit le lot **D2** et appelle un juge par `rig` avec l'image.

**Étape 0, transport (§3).** Contre un serveur local qui enregistre les requêtes : l'image part en
base64 (Ollama) ou en data URI (compatible OpenAI), `temperature` et la graine figurent dans le corps
envoyé. Une réponse sans sortie structurée, rejouée depuis un fichier, passe par le repli JSON et le
mode d'extraction est tracé. Le nombre d'appels est compté et publié pour un passage de lot.

**Étape 1, parité.** En deux parties distinctes :

- **hors ligne, en CI** : `Score` (Rust) et `score.py` reçoivent les mêmes fichiers figés (clé, export
  humain, verdicts de modèle en JSON Lines) et produisent les mêmes valeurs pour **toutes** les
  sorties du §4, à l'arrondi d'affichage près. Aucun appel de modèle ;
- **campagne manuelle de reproductibilité**, avec l'accord du §3 bis : `swift-1.5-27b` retrouve
  **33/39 ± 1 carte** et `gemma4:e2b` **22/39 ± 1**. Tolérance par compte (accord, faux « pertinent »,
  faux « non pertinent », ancres du juge) : ± 1 carte, le bruit mesuré à température 0 (38/39 verdicts
  identiques, protocole §4). Le kappa n'a pas de tolérance propre : il découle des comptes.

**Étape 2, banc d'essai.** Les candidats juges passent sur D2 en mode *évaluation*, témoin compris.
Pour chacun : admis ou non selon le seuil du §4, pouvoir discriminant compris. Résultat publié même
négatif. Note : avec les mesures historiques, D2 échoue déjà le critère strict (`swift` [70 ; 93]
contre `gemma4:e2b` [41 ; 71], un point de recouvrement). Le lot G passe ensuite comme **contrôle
négatif** : on attend qu'il soit déclaré non discriminant ; aucune admission n'y est possible.

Puis le choix des juges (cible : trois) se fait sur des mesures publiées, pas sur le meilleur score
isolé : erreurs **communes** par type (celles que le tri ne voit pas), volume de cartes renvoyées à
l'humain, et erreur résiduelle mesurée sur l'échantillon tiré au sort.

Aucun envoi externe hors de l'accord du §3 bis.

Hors périmètre : boucle d'agent, génération de pages par agent, pilotage d'Obscura, tout
changement du produit d'audit.

## 9. Points à trancher à l'implémentation

Ce que les sources du dépôt ne tranchent pas ; à décider avec l'utilisateur, pas à inventer.

1. **Taille de l'échantillon tiré au sort** parmi les accords (§4 bis l'exige sans la fixer).
2. **Verdict manquant à l'admission** : compté comme erreur, ou couverture complète exigée ?
   `score.py` exclut aujourd'hui les cartes sans résultat, ce qui peut flatter un juge qui ne répond
   qu'aux cartes faciles.
3. **Lot d'admission pour l'alternative d'image** : D2 échoue le critère strict (§8, étape 2) et G
   est un contrôle négatif ; faut-il un lot plus difficile avant toute admission sur cette famille ?
4. **Forme du doute** : verdict `indécis` (proposé ici) ou confiance chiffrée avec seuil ; une
   confiance déclarée par un LLM n'est pas calibrée.
5. **DeepSeek** : les ajouter à `candidates.toml` et étendre l'accord permanent, ou les garder en
   accord par exécution.
6. **Pondération du choix des juges** entre erreurs communes, volume de relecture et erreur résiduelle.
7. **Ancres du modèle dans `score.py`** : l'écart avec le protocole §4 relevé dans `Score` (§4 de cette spec) est à corriger
   aussi côté Python, hors de cette spec.
