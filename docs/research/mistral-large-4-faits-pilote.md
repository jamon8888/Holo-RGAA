# Mistral Large 4 — faits manquants pour le pilote

Ticket : #402 (carte parente #401). Questions : PRD §14 q. 1 à 5.
Toutes les sources ont été consultées le **2026-11-20**. Seules des sources
primaires sont retenues : docs.mistral.ai (fiche modèle, changelog, prix,
limites connues, glossaire d'erreurs, page vision, spécification OpenAPI),
mistral.ai/news et le dépôt de poids officiel `mistralai` sur Hugging Face.
La console (`admin.mistral.ai`, `console.mistral.ai`) exige une authentification :
elle n'a pas pu être lue.

## Déjà établi (non refait ici)

- Identifiants API : `mistral-large-4`, `mistral-large-4-0`, `mistral-large-latest`.
- `reasoning_effort` ∈ `low | medium | high` ; `none` → HTTP 400 ; défaut `high`
  (changelog du 2026-10-29).
- `content` de la réponse : liste de blocs `thinking` / `text`.

## Synthèse

| Question | Réponse | Confiance |
|---|---|---|
| Limites de débit | Par modèle et par organisation (tokens/minute et requêtes/seconde, appliqués indépendamment), visibles seulement dans Admin Panel › API › Limits. **Aucun chiffre public pour Large 4.** | Haute (mécanisme) / chiffres non trouvés |
| Sortie maximale | **Aucune limite de sortie publiée.** Les docs disent seulement que entrée + sortie partagent la fenêtre de contexte. | Non trouvé |
| Images | 8 images/requête, 10 Mo/fichier, PNG/JPEG/WEBP/GIF non animé, 10000×10000 max (FAQ vision). La page « limites connues » dit au contraire 20 Mo et un nombre « dépendant du modèle ». Rien de propre à Large 4. | Moyenne (sources contradictoires) |
| `response_format` `json_schema` | `{"type":"json_schema","json_schema":{"name":…,"description":…,"schema":{…},"strict":true}}` ; `strict` existe, booléen, **défaut `false`**. | Haute (OpenAPI) |
| Facturation des appels échoués | **Non documentée.** | Non trouvé |
| Prix après la promotion | 0,70 $ entrée / 0,07 $ entrée en cache / 2,10 $ sortie par M de tokens, « à partir d'aujourd'hui » dans l'annonce GA. | Haute |
| Taille du contexte | **1M** (fiche, vue d'ensemble, changelog) ; `max_position_embeddings = 1 048 576` dans les poids publiés. Aucune source primaire ne mentionne 512K / 524 288. | Haute |

## 1. Limites de débit

- « Completion rate limits are listed per model and include: Tokens per minute
  […] Requests per second » ; elles sont affichées dans « Admin Panel › API › Limits ».
  — https://docs.mistral.ai/admin/billing-usage/usage-limits
- « Rate limits vary by subscription tier and model. When exceeded, the API returns
  `429 Too Many Requests`. Requests per second and tokens per minute are enforced
  independently. Limits apply per organization. […] Batch processing does not count
  against real-time rate limits. » L'en-tête `X-RateLimit-Remaining` est conseillé.
  — https://docs.mistral.ai/resources/known-limitations
- Le glossaire d'erreurs précise : 429 = limite dépassée **ou** modèle à court de
  capacité (code `3505 backend_out_of_capacity`) ; lire `Retry-After` et les
  en-têtes `x-ratelimit-limit-*` / `x-ratelimit-remaining-*` ; les limites peuvent
  être par seconde, par minute ou par mois ; « A monthly limit or a limit of `0`
  doesn't clear by retrying ».
  — https://docs.mistral.ai/resources/error-glossary
- Le mode gratuit « doesn't include API access » ; l'API demande le
  paiement à l'usage (pay-as-you-go). — https://docs.mistral.ai/admin/billing-usage/usage-limits
- **Chiffres pour `mistral-large-4` : introuvables publiquement.** À lire sur
  https://admin.mistral.ai/plateforme/limits avec le compte du pilote.

## 2. Sortie maximale

- La fiche modèle ne donne aucune limite de sortie.
  — https://docs.mistral.ai/models/mistral-large-4-0
- « Requests exceeding the model's context window return a `400 Bad Request` error.
  Token counts include both input and output tokens. Plan your `max_tokens`
  accordingly. » — https://docs.mistral.ai/resources/known-limitations
- Dans l'OpenAPI, `max_tokens` est un entier `minimum: 0` ou `null`, sans maximum.
  — https://docs.mistral.ai/openapi.yaml
- Un dépassement de contexte renvoie `3059 invalid_request_prompt_too_long`
  (« Prompt <N> > <M> maximum context length ») ; des `max_tokens` élevés sont
  cités comme cause de 504. — https://docs.mistral.ai/resources/error-glossary

## 3. Images

- Large 4 accepte l'entrée `text, image` (sortie `reasoning, text`).
  — https://docs.mistral.ai/models/mistral-large-4-0
- La page vision cite Large 4 (`mistral-large-4-0`) parmi les modèles vision recommandés.
  Sa FAQ (contenu replié, lu dans le source de la page) donne :
  - « The maximum number images per request via API is 8. »
  - « The current file size limit is 10Mb. »
  - Formats : PNG (.png), JPEG (.jpeg, .jpg), WEBP (.webp), GIF non animé, une seule image (.gif).
  - « An error will be sent if the resolution is higher than 10000x10000. »
  - Prix : « same pricing as input tokens per image, with each image being tokenized ».
  - Le tableau résolution → tokens ne liste que **Mistral Large 3 (1540×1540)** ;
    aucune ligne pour Large 4.
  — https://docs.mistral.ai/inference/conversations/vision
- **Contradiction** : la page « limites connues » dit « Maximum image size: 20 MB per
  image. Supported formats: PNG, JPG, JPEG, GIF, WEBP. Maximum images per request
  depends on the model and total token budget. »
  — https://docs.mistral.ai/resources/known-limitations
  Pour le pilote, la prudence veut qu'on retienne 8 images et 10 Mo tant que ce
  n'est pas vérifié par un appel réel.
- Un excès d'images est rejeté par `3051 invalid_request_invalid_args` (HTTP 400).
  — https://docs.mistral.ai/resources/error-glossary
- Bloc image (OpenAPI, `ImageURLChunk`) : `{"type":"image_url","image_url":"<url ou data:image/png;base64,…>"}`
  ou `{"type":"image_url","image_url":{"url":"…","detail":"low|auto|high"}}`.
  — https://docs.mistral.ai/openapi.yaml
- Indice non contractuel : les poids publiés déclarent un encodeur vision
  `max_image_size: 1540`, `patch_size: 14` (`params.json`).
  — https://huggingface.co/mistralai/Mistral-Large-4-1T-A52B/blob/main/params.json

## 4. `response_format` de type `json_schema` et `strict`

Schémas `ResponseFormat` et `JsonSchema` de la spécification OpenAPI publique
(https://docs.mistral.ai/openapi.yaml) :

```json
{
  "response_format": {
    "type": "json_schema",
    "json_schema": {
      "name": "book",
      "description": "facultatif, string ou null",
      "schema": {
        "type": "object",
        "title": "Book",
        "properties": {
          "name": { "title": "Name", "type": "string" },
          "authors": { "title": "Authors", "type": "array", "items": { "type": "string" } }
        },
        "required": ["name", "authors"],
        "additionalProperties": false
      },
      "strict": true
    }
  }
}
```

- `type` ∈ `text | json_object | json_schema`, défaut `text`.
- `json_schema` : `name` et `schema` **requis** ; `description` facultatif ;
  `strict` booléen, **défaut `false`** ; `additionalProperties: false` sur
  l'objet `JsonSchema` (champ inconnu → rejet). Le SDK Python renomme `schema`
  en `schema_definition` (`x-speakeasy-name-override`), mais le JSON brut
  emploie bien `schema`.
- Description officielle : `json_schema` « guarantees the message the model
  generates is in JSON and follows the schema you provide ».
- La fiche Large 4 liste la capacité `structured-outputs`.
  — https://docs.mistral.ai/models/mistral-large-4-0
- **Non trouvé** : ce que `strict: true` change concrètement (sous-ensemble de
  JSON Schema accepté, comportement si le schéma n'est pas supporté). La page
  « custom structured outputs » ne parle pas de `strict` et ne montre que le SDK Python.
  La page « limites connues » affirme même « JSON mode does not guarantee adherence
  to a specific schema » (sur le mode `json_object`, pas `json_schema`).

## 5. Facturation des appels échoués

- Ni la page facturation, ni le glossaire d'erreurs, ni les limites connues ne
  disent si un appel en erreur (4xx, 5xx, 429, timeout) est facturé.
  — https://docs.mistral.ai/admin/billing-usage/billing ,
  https://docs.mistral.ai/resources/error-glossary ,
  https://docs.mistral.ai/resources/known-limitations
- Seul fait voisin : à crédit épuisé, les requêtes échouent en 402 (`2308
  billing_wallet_depleted`). — https://docs.mistral.ai/resources/error-glossary

## 6. Prix après la promotion de lancement

- Changelog du **2026-10-06** : Large 4 « is now available in Public Preview » ;
  « Launch pricing: 50% off for 2 weeks. »
  — https://docs.mistral.ai/resources/changelogs
- Annonce de la préversion (2026-10-06) : aucun prix chiffré dans le texte.
  — https://mistral.ai/news/mistral-large-4/
- Annonce des poids ouverts / GA : « Starting today, Mistral Large 4 is priced at
  $0.70 input / M, $0.07 cached input / M, and $2.10 output / M tokens. »
  — https://mistral.ai/news/mistral-large-4-oss/
- Fiche modèle : statut **GA**, date de sortie **2026-11-05**, version 26.10 ;
  prix : entrée 0,7 $/M, entrée en cache 0,07 $/M, sortie 2,1 $/M.
  — https://docs.mistral.ai/models/mistral-large-4-0
- Page de prix : mêmes montants, aucune promotion affichée.
  — https://docs.mistral.ai/inference/pricing
- Conclusion : **le tarif courant (après la promotion) est 0,70 / 0,07 / 2,10 $
  par M de tokens.** Non écrit : le prix exact pendant la promotion. Par calcul
  seulement, -50 % donnerait 0,35 / 0,035 / 1,05 $ ; ce n'est pas publié. On ne sait
  pas non plus si ces montants ont changé entre la préversion et la GA.

## 7. Taille du contexte

- Fiche modèle : « Context length: 1M ». — https://docs.mistral.ai/models/mistral-large-4-0
- Vue d'ensemble des modèles : Large 4, GA, **1M**. — https://docs.mistral.ai/models
- Changelog du 2026-10-06 : « a 1M context window ».
  — https://docs.mistral.ai/resources/changelogs
- Poids publiés : `max_position_embeddings: 1048576` dans `config.json` et
  `params.json`, obtenu par YaRN (`original_max_position_embeddings: 512`,
  `factor: 2048` ; 512 × 2048 = 1 048 576).
  — https://huggingface.co/mistralai/Mistral-Large-4-1T-A52B/blob/main/config.json
- **Aucune source primaire consultée ne mentionne 512K ni 524 288** (soit 512 × 1024,
  donc la moitié de la valeur publiée). Ce chiffre vient sans doute d'une source
  secondaire ou d'une limite propre à un hébergeur tiers. L'API peut appliquer
  une limite plus basse sans la documenter : le seul moyen de le savoir est
  l'erreur `3059`, qui renvoie « Prompt <N> > <M> maximum context length » avec
  la limite `<M>` réelle.

## Non trouvé

- Chiffres de débit (RPS, TPM, plafond mensuel) pour `mistral-large-4` : console seulement.
- Limite de sortie maximale (`max_tokens`) propre à Large 4.
- Facturation des requêtes en échec (4xx/5xx/429/timeouts).
- Prix exact pendant la promotion (seulement « 50% off for 2 weeks »).
- Sémantique de `strict: true` (sous-ensemble de JSON Schema accepté).
- Tokens par image et résolution maximale pour Large 4 (le tableau ne couvre que Large 3).
- Limites d'images propres à Large 4, et quelle page fait foi entre « 8 images / 10 Mo »
  et « 20 Mo / selon le modèle ».
- Toute source primaire pour « 512K / 524 288 ».

## Pour lever le doute

Ces points se règlent mieux par un appel réel que par la doc :

1. Lire Admin Panel › API › Limits avec la clé du pilote.
2. Envoyer un prompt de plus de 1M tokens et lire `<M>` dans l'erreur 3059.
3. Envoyer 9 images, puis une image de 11 Mo, et lire les erreurs.
4. Comparer le relevé d'usage avant et après une requête qui échoue en 400 ou 429.
