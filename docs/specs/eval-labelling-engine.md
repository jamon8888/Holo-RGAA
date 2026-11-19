# Moteur d'évaluation et de préétiquetage (`rig`, sans boucle d'agent)

Statut : **proposition, décisions de conception prises (#358)**. Rien n'est implémenté. Carte : #330. Prolonge le protocole de
calibration (`docs/specs/calibration-protocol.md`) ; n'en change aucune règle.

## 1. Ce que c'est, et ce que ce n'est pas

Un moteur **déterministe** (carte → juges → agrégation → score), écrit en Rust sur `rig-core`,
qui automatise trois tâches du protocole :

1. **étiquettes par construction** : on part d'un texte correct, on y injecte un défaut connu ;
   l'étiquette est certaine sans jugement ;
2. **tri par désaccord** : plusieurs juges préétiquettent ; l'humain ne voit que les cartes où ils
   divergent ou doutent, plus un échantillon des cartes d'accord ;
3. **évaluation rejouable** : un juge est rejoué sur un lot gelé, et son score est recalculé.

**Ce n'est pas** un agent : aucune boucle outil/observation, aucune décision du système sur la
suite. **Ce n'est pas** un remplaçant de la référence humaine : un juge LLM ne se valide pas par
un autre juge LLM (voir §6).

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
| URL de base configurable | `.base_url(..)`, déjà utilisé par `rgaa-agent` | `rgaa-agent/src/agent.rs:338` |

## 3. Ce qui n'est PAS vérifié (à établir par le prototype, §8)

- l'envoi réel d'une image à Ollama et à MyIA **par `rig`** (lu dans le code, jamais exécuté) ;
- que la graine et `temperature = 0` aboutissent réellement au fournisseur ;
- que la sortie structurée native est honorée par chaque backend (sinon : repli sur le JSON
  extrait, comme `run_models.py`) ;
- le coût en appels du tri par désaccord sur un lot réel.

## 3 bis. Candidats juges (fiches NVIDIA NIM, non mesurés chez nous)

| modèle | annoncé | rôle visé |
|---|---|---|
| `swift-1.5-27b` (MyIA) | mesuré : D2 85 %, kappa 0,68, 19 % de faux « pertinent » | juge vision |
| GLM-5.3-Flash (NIM) | multimodal natif | juge vision, à mesurer |
| DeepSeek-V4.1-Flash (NIM) | texte et image en entrée | juge vision, à mesurer |
| DeepSeek-V4-Flash (NIM) | texte seul (aucune mention d'image) | générateur de défauts sur du texte |
| `gemma4:e2b` (local) | mesuré : D2 56 %, sous le repère trivial | référence faible du test de pouvoir discriminant |
| Kumo Tabular (NIM) | classe des données tabulaires en contexte | pas un juge ; agrégateur de votes envisageable **après** les mesures, non retenu d'avance |

Une fiche dit qu'une image est acceptée, pas que le modèle juge bien nos tableaux et nos libellés :
l'admission de chaque candidat se mesure (§8). Chaque appel NIM envoie nos données à un service
externe : accord de l'utilisateur par exécution.

## 4. Architecture proposée

Un nouveau crate `rgaa-eval` sur `rig` (décidé, §7) avec quatre briques indépendantes :

- **`Judge`** : un modèle, une consigne, une température fixe ; entrée = une carte (image
  optionnelle, contexte, texte, test) ; sortie typée `{verdict, cohérence, complétude, justification}`.
  Deux backends : Ollama et compatible OpenAI (MyIA).
- **`Lot`** : lecture de `items.json` / `key.json`, mêmes formats que `tools/calibration/`.
- **`Score`** : même sortie que `score.py` (accord, kappa, Wilson, repère trivial, faux
  « pertinent », ancres, contrôles de justification). Parité vérifiée par test (§8).
- **`Mutation`** : générateurs de défauts connus (valeur changée, détail supprimé, nom d'un autre
  élément, texte générique, consigne au lieu d'un libellé). Chaque carte produite porte son
  étiquette et le défaut injecté.

Format d'échange : les JSON existants, pour que `sheet.py` et `score.py` restent utilisables.

## 5. Garde-fous, à coder dès le premier jour

1. **Aucun envoi externe sans drapeau explicite** (équivalent de `--allow-external`) ; le moteur
   refuse sinon, il n'avertit pas.
2. **Clé d'API lue par nom de variable d'environnement**, jamais écrite ni journalisée.
3. **Un juge n'est admis que s'il passe le seuil d'admission du protocole (§4) sur le jeu humain
   gelé** ; le moteur refuse d'étiqueter avec un juge non admis.
4. **Trace par appel** : modèle, version, consigne (empreinte), carte, réponse brute tronquée.
5. **Pouvoir discriminant** : un lot n'est utilisable que si le modèle faible de référence y est
   nettement sous le fort (protocole §4) ; le moteur calcule et publie ce test.

## 6. Limites assumées

- Les étiquettes par construction établissent la **présence d'un défaut connu**, pas le critère
  de l'auditeur : les rejets « vrais mais incomplets » (ADR 0001) ne s'obtiennent pas ainsi.
- Un accord élevé entre juges sur des cas faciles ne prouve rien (lot G : 100 % contre 92 %).
- Le critère d'étiquetage est défini par l'humain et peut évoluer ; le moteur ne le détecte pas.
- La référence humaine ne disparaît pas : elle se réduit à un **jeu gelé** par famille, plus les
  cartes de désaccord. La taille de ce jeu est une hypothèse à mesurer, non un résultat.

## 7. Décisions (ticket de grilling #358)

Prises avec l'utilisateur ; Q1, Q3 et Q4 le 19/10/2026 (inscrites dans
`calibration-protocol.md` §4 bis), Q2 et Q5 le 19/11/2026. Principe commun : **un modèle juge ne
devient jamais la référence** ; il réduit le travail humain.

1. **Seuil d'admission d'un juge.** Le seuil du protocole §4 seul ne suffit pas (il a admis un
   modèle de 2 Md de paramètres sur le lot G) : l'admission exige en plus le **pouvoir
   discriminant** du lot (modèle faible nettement sous le fort, intervalles disjoints), et le taux
   d'erreur du juge est publié. Aucun juge n'est promu aujourd'hui.
2. **Emplacement du code.** Nouveau crate **`rgaa-eval`** sur `rig`, avec un seul chemin d'appel
   aux modèles pour l'éval ; `rgaa-holo` reste inchangé.
3. **Jeu humain gelé par famille.** Le lot calibré lui-même (≥ 36 cartes) plus ses cartes de
   re-contrôle ; pas de nombre plus petit sans mesure. C'est la seule référence.
4. **Juges et relecture.** Plusieurs juges (cible : trois, de fournisseurs différents) ; l'humain
   arbitre toutes les cartes en désaccord plus un échantillon tiré au sort des cartes d'accord.
5. **Étiquettes par construction.** **Complément** du jeu humain, **jamais référence** de
   calibration : elles établissent la présence d'un défaut connu, pas le critère de l'auditeur (§6).

## 8. Prototype (critère d'acceptation)

Un binaire qui lit le lot **D2** et appelle un juge par `rig` avec l'image.

**Étape 1, parité.** `swift-1.5-27b` retrouve **33/39 ± 1 carte** (bruit de reproductibilité à
température 0 : environ une carte) ; `gemma4:e2b` retrouve **22/39 ± 1**. Accord, kappa et faux
« pertinent » identiques à `score.py`.

**Étape 2, banc d'essai.** GLM-5.3-Flash et DeepSeek-V4.1-Flash passent sur D2 puis G. Pour chacun :
admis ou non selon le seuil du protocole (§4) *et* le pouvoir discriminant (modèle faible nettement
en dessous, intervalles disjoints). Résultat publié même négatif. Puis accord entre juges admis : le
bon duo est celui dont les erreurs diffèrent le plus, pas celui qui a le meilleur score isolé.

Aucun envoi externe sans l'accord de l'utilisateur pour cette exécution.

Hors périmètre : boucle d'agent, génération de pages par agent, pilotage d'Obscura, tout
changement du produit d'audit.
