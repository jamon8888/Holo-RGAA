# Moteur d'évaluation et de préétiquetage (`rig`, sans boucle d'agent)

Statut : **proposition**. Rien n'est implémenté. Carte : #330. Prolonge le protocole de
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

## 4. Architecture proposée

Un crate `rgaa-eval` (placement à confirmer, §7) avec quatre briques indépendantes :

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

## 7. Décisions ouvertes (ticket de grilling)

1. Qui valide un juge, et sur quel seuil : le seuil du protocole §4 suffit-il, sachant qu'il a
   admis un modèle de 2 Md de paramètres sur le lot G ?
2. Où vit le code : nouveau crate `rgaa-eval` ou module de `rgaa-holo` (qui a déjà ses clients
   Ollama/OpenAI et n'utilise pas `rig`) ? Un seul chemin d'appel aux modèles doit servir l'éval.
3. Taille minimale du jeu humain gelé par famille.
4. Nombre de juges et échantillonnage des cartes d'accord relues par l'humain.
5. Les étiquettes par construction comptent-elles comme référence, ou comme complément du jeu humain ?

## 8. Prototype (critère d'acceptation)

Un binaire qui lit le lot **D2**, appelle `swift-1.5-27b` par `rig` avec l'image, et retrouve
**33/39 ± 1 carte** (le bruit de reproductibilité mesuré à température 0 est d'environ une carte).
`gemma4:e2b` en local doit retrouver 22/39 ± 1. Sortie comparée à `score.py` : mêmes accord, kappa
et faux « pertinent ». Aucun envoi externe sans accord de l'utilisateur pour cette exécution.

Hors périmètre : boucle d'agent, génération de pages par agent, pilotage d'Obscura, tout
changement du produit d'audit.
