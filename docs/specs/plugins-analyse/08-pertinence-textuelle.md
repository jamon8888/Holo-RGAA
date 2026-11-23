# P-08 — Plugin `text` (pertinence textuelle)

**Critères** :

| Groupe | Critères |
|--------|----------|
| Alternatives et cadres | 1.5 (alternative au CAPTCHA), 2.2 (titre de cadre) |
| Tableaux | 5.2 (résumé de tableau), 5.5 (titre de tableau) |
| Liens et composants | 6.1 (lien explicite), 7.2 (alternative de script) |
| Titre de page | 8.6 |
| Formulaires | 11.2 (étiquette), 11.7 (légende), 11.9 (intitulé de bouton), 11.10 (contrôle de saisie), 11.11 (suggestion de correction), 11.12 (confirmation, modification, annulation), 11.13 (finalité du champ) |
| Navigation et contenus | 12.3 (plan du site), 13.4 (version alternative équivalente), 13.6 (alternative de contenu cryptique) |

**Dépend de** : P-00, P-06 · **Modèle** : plongements texte locaux · **Feature** : `plugin-text` · **Forme** : natif

## 1. Principe

Ces critères demandent tous si **un texte est pertinent par rapport à son contexte**. La machine ne peut pas trancher seule. En revanche, elle peut :

- écarter les cas évidents par des règles ;
- **regrouper** les textes identiques ;
- **classer** les autres par degré de suspicion.

Seuls les cas restants partent au LLM, **en lot**, avec un contexte réduit.

## 2. Bibliothèques

| Crate | Version | Usage |
|-------|---------|-------|
| `fastembed` | ≥ 4 (déjà au workspace) | plongements texte multilingues (modèle `multilingual-e5-small` ou équivalent proposé par fastembed) : similarité libellé ↔ contexte, regroupement |
| `strsim` | 0.11 | distances de Levenshtein et Jaro-Winkler : libellés quasi identiques, titre ↔ `h1` |
| `lingua` (via P-06) | 1.8 | langue du texte pour choisir les listes de mots |

## 3. Règles et signaux (avant le LLM)

| Critère | Règle déterministe (`Mesuree`) | Signal estimé (`Estimee`) |
|---------|-------------------------------|---------------------------|
| 6.1 | intitulé dans la liste générique FR/EN (« cliquez ici », « en savoir plus », « lire la suite », « ici », « more »), sans contexte AX (`aria-label`, `aria-describedby`, titre de liste) | **liens de même intitulé vers des cibles différentes** (URL normalisée) sur la page ; similarité intitulé ↔ titre de la page cible (si elle a été explorée) |
| 8.6 | titre vide, titre identique sur plus de 3 pages différentes, titre sous forme de nom de fichier | similarité `title` ↔ `h1` et ↔ contenu principal ; Jaro-Winkler `title` ↔ `h1` |
| 2.2 | `iframe` sans titre ou titre générique (« iframe », « frame », « widget ») | similarité titre ↔ texte du document du cadre |
| 11.2, 11.13 | étiquette vide, étiquette égale au `placeholder` seul, étiquette composée d'un seul symbole (« * ») | classe attendue de la finalité (`autocomplete` attendu : nom, e-mail, tél.…) déduite du libellé par similarité avec un lexique ; écart avec `autocomplete` présent |
| 11.9 | bouton sans nom, nom réduit à une icône | similarité nom ↔ action (texte proche, formulaire parent) |
| 11.10, 11.11 | champ `required` sans indication visible ni AX ; format attendu (date, téléphone) sans exemple ; on soumet le formulaire vide (**seulement** pour les formulaires sans effet de bord, règle P-07 §6) et on capture les messages d'erreur | message d'erreur générique (« champ invalide ») ou sans suggestion |
| 11.12 | formulaire à finalité financière, juridique ou de données personnelles (lexique : « payer », « commander », « supprimer mon compte ») | présence d'une étape de récapitulatif ou de confirmation repérée après soumission simulée (**sans** soumettre réellement un paiement : on s'arrête à la page récapitulative si elle existe) |
| 5.2, 5.5 | tableau complexe sans résumé ; titre (`caption`) absent | similarité `caption` ↔ en-têtes et contenu |
| 12.3 | plan du site absent | couverture du plan par rapport aux pages découvertes par `rgaa-spider` (part des URL du spider présentes dans le plan) |
| 13.4 | — | similarité et couverture du texte de la version alternative par rapport à l'original (mêmes calculs que P-04 §3.4) |
| 1.5, 7.2, 13.6 | alternative absente | regroupement et contexte pour le LLM |

## 4. LLM (Mistral Large 4)

- Les cas restants sont regroupés **par critère**, jusqu'à 40 éléments par requête. Pour chacun, on envoie l'intitulé, le contexte réduit (300 caractères, titre de section, cible du lien) et les signaux calculés.
- La réponse est un JSON structuré par élément, `{ "id", "pertinent": "oui|non|incertain", "explication" }`. Elle va dans `mesures.avis_modele`, **jamais** dans le statut (R2/R3).
- `reasoning_effort = low` par défaut. La mesure de coût du PRD §8.2 justifie ce choix : la sortie représente 83 % du coût.
- **Cache** : la clé est `(critère, intitulé normalisé, contexte_hash)`. Les liens « En savoir plus » du pied de page ne sont donc envoyés qu'une fois par audit.

## 5. `mesures` (exemple 6.1)

```json
{ "intitule": "En savoir plus", "nom_accessible": "En savoir plus", "cible": "/offres/pro",
  "generique": true, "meme_intitule_cibles_differentes": 7, "similarite_cible": 0.31,
  "avis_modele": { "pertinent": "non", "explication": "intitulé générique sans contexte de liste" } }
```

## 6. Tests

| Cas | Attendu |
|-----|---------|
| 7 « En savoir plus » vers 7 URL | `Mesuree` générique, plus regroupement : un seul appel LLM |
| `<title>Accueil</title>` sur 12 pages | 8.6, règle de titre répété |
| Champ « Courriel » avec `autocomplete="tel"` | 11.13, écart de finalité |
| Formulaire d'inscription vide soumis | messages capturés ; 11.10 et 11.11 évalués |
| Bouton « Payer » | aucune soumission ; seule la présence d'un récapitulatif est observée |

## 7. Critères d'acceptation

1. Sur les 3 sites du corpus, moins de 30 % des éléments inventoriés partent au LLM, chiffre mesuré et noté dans le ticket.
2. Aucune soumission de formulaire n'a d'effet de bord : test sur une fixture qui compte les requêtes `POST` reçues, avec 0 attendu pour les formulaires sensibles.
3. Les prompts sont versionnés dans le crate, et leur empreinte figure dans `version_modele`.
