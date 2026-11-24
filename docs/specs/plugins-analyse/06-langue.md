# P-06 — Plugin `lang` (langue)

**Critères** : 8.4 (langue par défaut pertinente), 8.8 (changements de langue pertinents) · appui à 8.3 (langue présente) et 8.7 (changements signalés) · **Dépend de** : P-00 · **Modèle** : intégré au crate · **Feature** : `plugin-lang` (défaut) · **Forme** : natif

## 1. Bibliothèque

| Crate | Version | Usage |
|-------|---------|-------|
| `lingua` | 1.8 | détection de langue ; fiable sur les textes courts ; **restreinte aux 24 langues officielles de l'UE**, plus le catalan, le basque, le galicien, le norvégien, l'islandais, le turc, l'ukrainien, le russe, l'arabe et le chinois ; liste configurable |

`whatlang` est plus léger mais moins précis sous 50 caractères ; il a été écarté. Les modèles `lingua` sont embarqués dans le crate. Il n'y a rien à télécharger, d'où le plugin par défaut. Pour limiter la taille du binaire, on n'active que les features de langues listées.

## 2. Algorithme

1. **Segmentation.** On découpe le texte visible en blocs (DOM : `p`, `li`, `td`, `h*`, `blockquote`, `figcaption`, `button`, `a`, `alt` et `title`), chacun avec son `lang` hérité (attribut le plus proche, sinon `html[lang]`).
2. **Détection.** Pour chaque bloc d'au moins 20 caractères, on calcule `detect_language_of` et la confiance. Les blocs plus courts sont groupés par parent. **La confiance est la marge entre la meilleure langue et la deuxième** (valeurs de `compute_language_confidence_values`), et non la probabilité brute. Celle-ci se dilue avec le nombre de langues activées : sur « Stay hungry, stay foolish », avec 33 langues, la probabilité brute vaut 0,71 et la marge 0,95. Correction du 2026-11-22, PR #469.
3. **8.4.** On détecte la langue de la page en pondérant chaque bloc par sa longueur, puis on la compare au `html[lang]` normalisé (BCP 47, sous-étiquette primaire). Une différence avec une confiance > 0,8 donne `Suspect`, avec la raison « page détectée en `en`, déclarée `fr` ».
4. **8.8 et 8.7.** Un bloc dont la langue détectée diffère du `lang` hérité, avec une marge de confiance > 0,9 et au moins 4 mots (l'exemple de référence, « Stay hungry, stay foolish », en compte 4), est candidat à un changement de langue non signalé. Un bloc portant un `lang` explicite que la détection contredit relève d'un `lang` non pertinent (8.8).
5. **Exclusions.** Sont ignorés :
   - les noms propres, marques, sigles et mots passés dans l'usage (« week-end ») ;
   - les blocs à forte part de majuscules ou de chiffres.
   La liste d'exclusions est configurable (`rgaa.toml`).
6. **LLM.** Il n'est pas appelé par défaut. L'option `opts.llm` envoie seulement les candidats 8.7 à Mistral Large 4 en lot, avec la question « emprunt passé dans l'usage ou passage en langue étrangère ? ».

## 3. `mesures`

```json
{ "declaree": "fr", "detectee": "fr", "confiance": 0.97,
  "blocs_divergents": [{ "selecteur": "#quote-3", "lang_herite": "fr", "detectee": "en", "confiance": 0.95, "extrait": "Stay hungry, stay foolish" }] }
```

## 4. Tests

| Cas | Attendu |
|-----|---------|
| Page française, `lang="en"` | 8.4 `Suspect` |
| Citation anglaise sans `lang` dans une page FR | candidat 8.7 |
| Même citation avec `lang="en"` | aucun |
| `lang="de"` sur un paragraphe français | 8.8 `Suspect` |
| « Le week-end, je fais du jogging » | aucun candidat (exclusion) |

## 5. Critères d'acceptation

1. Le plugin traite une page de 2 000 mots en moins de 100 ms (`--release`).
2. Sur 3 pages réelles du corpus, il produit moins de 1 faux candidat 8.7 par page (relecture humaine notée dans le ticket).
3. L'augmentation de la taille de `rgaa-cli` est mesurée et notée. On réduit la liste de langues par défaut si elle dépasse 60 Mo.
