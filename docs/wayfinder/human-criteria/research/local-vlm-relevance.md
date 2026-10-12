# Modèles de vision locaux open source pour juger la pertinence (alternatives, titres, légendes)

Ticket : [#333](https://github.com/jamon8888/Holo-RGAA/issues/333) — carte parente [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
Recherche menée le 2026-10-12, sur `origin/master` `50b4271`.

Légende des marqueurs utilisés dans ce document :

- **[source]** : fait établi par la source citée juste après.
- **[inférence]** : déduction de l'auteur de ce document, non publiée par une source.
- **non vérifié** : fait qu'il n'a pas été possible de confirmer.

## Réponse courte

1. **Aucune étude publiée trouvée** ne mesure l'accord entre un VLM **open-weight exécuté localement** et des **experts** sur la pertinence d'alternatives textuelles, encore moins en **français** ou selon le **RGAA**. Les mesures existantes portent sur des modèles propriétaires (GPT-4o, Claude), en anglais, et contre des évaluateurs non experts.
2. La meilleure mesure trouvée (GPT-4o, alt de sites e-commerce, 157 images) donne 93 % d'exactitude en classement binaire « adéquat / inadéquat », avec un modèle **plus indulgent** que les humains et qui **ne propose jamais `alt=""`**. Cela justifie un niveau **Assisté**, pas Automatique.
3. Plusieurs familles de modèles locaux ont des **poids sous Apache-2.0** et sont **déjà appelables** par `rgaa-holo` via Ollama : Qwen3-VL, Gemma 4, Mistral Small 3.1 et 3.2, Qwen2.5-VL 7B et 32B. Llama 3.2 Vision est **exclu** : sa licence n'accorde pas les droits sur les modèles multimodaux aux personnes et entreprises domiciliées dans l'UE.
4. Verdict par test (détail au §6) :
   - **Assisté** : 1.3.x, 1.7.x, 5.5.1, 8.6.1, 11.7.1, ainsi que 4.2.1 (via ASR).
   - **Assisté faible, à prototyper, sinon Irréductible** : 4.2.2 et 4.2.3.
   - **Automatique : aucun**, faute de preuve publiée.
5. Constat annexe dans le code : le chemin multimodal actuel de `rgaa-holo` **rejette toute image réelle** (tampon de validation base64 de 64 octets). Voir §2.

## 1. Questions posées par le ticket

- Quels candidats VLM : licence des poids, taille, besoins mémoire, disponibilité Ollama ou llama.cpp, qualité en français ?
- Que dit la littérature sur l'évaluation automatique de la pertinence d'alternatives textuelles, notamment l'accord avec des experts ?
- Quels modes d'échec sont connus : hallucination, image décorative ou informative, contexte de la page ?
- Quel niveau atteindre pour chaque test RGAA : Automatique, Assisté ou Irréductible (définitions dans `CONTEXT.md`) ?

## 2. Ce que `rgaa-holo` sait déjà appeler

Constats établis par lecture du code sur `origin/master` `50b4271` :

- **Point d'accès.** `OllamaClient` vise l'endpoint compatible OpenAI `http://localhost:11434/v1/chat/completions`, avec un délai de 600 s (`rgaa-rs/crates/rgaa-holo/src/ollama.rs`).
- **Pas de modèle par défaut.** Le commentaire du code recommande « ~7B instruct » en interactif et « ~14B » en batch sur CPU.
- **Une seule image par appel.** `evaluate_multimodal(prompt, image_base64: Option<&str>)` envoie une seule image, sous forme de partie `image_url` avec une URL `data:` (`transport.rs`, `multimodal_messages`).
- **Type MIME forcé.** Le type est codé en dur à `image/png`, quel que soit le format réel. L'effet sur un JPEG ou un WebP est **non vérifié**.
- **Compatibilité côté Ollama.** L'API compatible OpenAI d'Ollama accepte les images en **base64** mais pas par URL distante. La documentation coche « Base64 encoded image » et laisse « Image URL » non cochée ([docs Ollama, openai-compatibility](https://github.com/ollama/ollama/blob/main/docs/api/openai-compatibility.mdx)). Le format envoyé par `rgaa-holo` est donc le bon.
- **Défaut bloquant.** `multimodal_messages` valide le base64 avec `decode_slice` dans un tampon fixe de **64 octets**. Avec `base64` 0.22.1, version épinglée par `Cargo.lock`, `decode_slice` renvoie une erreur dès que la sortie estimée dépasse le tampon. La documentation de la crate l'annonce : « an error if `output` is smaller than the estimated decoded length » (`base64-0.22.1/src/engine/mod.rs`).
  - Vérifié par exécution dans un projet jetable : une charge utile de 1 000 octets donne `Err(OutputSliceTooSmall)`, alors que le fixture de test de 16 octets passe (`Ok(16)`).
  - **Conséquence :** toute vraie capture d'écran est rejetée avec `INVALID_IMAGE`, et le test `multimodal_uses_same_wire_format` ne le détecte pas.
  - À corriger avant tout prototype VLM. C'est hors du périmètre de ce ticket de recherche.

## 3. Candidats

Le tableau ne retient que des modèles image→texte pouvant tourner localement.

Méthode :

- **Licence :** lue dans la carte Hugging Face (métadonnée `license`, et fichier LICENSE quand il a été lu).
- **Taille Ollama :** taille de téléchargement de l'étiquette par défaut, relevée sur ollama.com le 2026-10-12. C'est une **borne basse** de la mémoire nécessaire, pas une mesure de VRAM.
- **Français :** seule la déclaration de la carte est rapportée. Aucun banc VLM en français n'a été trouvé, donc la qualité réelle en français est **non vérifiée** pour tous les candidats.

| Modèle (carte HF) | Licence des poids | Usage commercial | Paramètres | Ollama (taille de l'étiquette) | llama.cpp (doc multimodale) | Français déclaré |
|---|---|---|---|---|---|---|
| [Qwen3-VL-2B / 4B / 8B-Instruct](https://huggingface.co/Qwen/Qwen3-VL-8B-Instruct) | Apache-2.0 | Oui | 8B : 8,8 G | `qwen3-vl` : 2b 1,9 Go ; 4b 3,3 Go ; latest 6,1 Go ([ollama](https://ollama.com/library/qwen3-vl)) | Non listé dans la doc multimodale (non vérifié par ailleurs) | La carte dit « OCR : 32 langues », sans liste lue |
| [Qwen3-VL-30B-A3B](https://huggingface.co/Qwen/Qwen3-VL-30B-A3B-Instruct) / [32B](https://huggingface.co/Qwen/Qwen3-VL-32B-Instruct) | Apache-2.0 | Oui | 31 G / 33 G | 30b 20 Go ; 32b 21 Go | Non listé | Idem |
| [Gemma 4 E2B / E4B / 12B / 26B-A4B / 31B](https://huggingface.co/google/gemma-4-12B-it) | Apache-2.0 (lien [gemma_4_license](https://ai.google.dev/gemma/docs/gemma_4_license)) | Oui (Apache-2.0) | E2B à 31B | `gemma4` : e2b 4,6 Go ; 12b 7,7 Go ; 26b 16 Go ; 31b 19 Go ([ollama](https://ollama.com/library/gemma4)) | Oui, GGUF ggml-org E2B, E4B, 26B, 31B ([doc](https://github.com/ggml-org/llama.cpp/blob/master/docs/multimodal.md)) | « 35+ langues » prises en charge ; le français n'a pas été lu explicitement |
| [Mistral-Small-3.2-24B-Instruct-2506](https://huggingface.co/mistralai/Mistral-Small-3.2-24B-Instruct-2506) | Apache-2.0 | Oui | 24 G | `mistral-small3.2` : 15 Go ([ollama](https://ollama.com/library/mistral-small3.2)) | La 3.1 est listée (quantification IQ2_M) | **`fr` dans les métadonnées** |
| [Mistral-Small-3.1-24B-Instruct-2503](https://huggingface.co/mistralai/Mistral-Small-3.1-24B-Instruct-2503) | Apache-2.0 | Oui | 24 G | `mistral-small3.1` : 15 Go | Oui | **`fr`** |
| [Pixtral-12B-2409](https://huggingface.co/mistralai/Pixtral-12B-2409) | Apache-2.0 | Oui | 12 G | **Absent** (ollama.com/library/pixtral renvoie 404) | Oui | **`fr`** |
| [Qwen2.5-VL-7B](https://huggingface.co/Qwen/Qwen2.5-VL-7B-Instruct) / [32B](https://huggingface.co/Qwen/Qwen2.5-VL-32B-Instruct) | Apache-2.0 | Oui | 8,3 G / 33 G | `qwen2.5vl` : latest 6,0 Go ; 32b 21 Go ([ollama](https://ollama.com/library/qwen2.5vl)) | Oui | Métadonnées `en` seulement |
| [MiniCPM-V 4.5](https://huggingface.co/openbmb/MiniCPM-V-4_5) | Apache-2.0 (section « Model License » de la carte) | Oui selon la carte | 8,7 G | **Non** : `minicpm-v:8b` est la version **2.6** (5,5 Go), dont la licence est **non vérifiée** (dépôt HF à accès restreint) | Non vérifié | La carte annonce « plus de 30 langues » |
| [InternVL3-8B](https://huggingface.co/OpenGVLab/InternVL3-8B) | Métadonnées `apache-2.0` **et** `license_name: qwen`. Le texte de la carte dit « MIT License » et indique que le composant Qwen2.5 est sous Apache-2.0 | **Ambigu, à clarifier** | 7,9 G | Non vérifié | Oui (GGUF ggml-org) | « multilingual » |
| [SmolVLM2-2.2B](https://huggingface.co/HuggingFaceTB/SmolVLM2-2.2B-Instruct) | Apache-2.0 | Oui | 2,2 G | Non vérifié | Oui | `en` seulement |
| [granite-vision-3.2-2b](https://huggingface.co/ibm-granite/granite-vision-3.2-2b) | Apache-2.0 | Oui | 3,0 G | `granite3.2-vision` : 2,4 Go | Non vérifié | `en` seulement |
| **Exclu** [Llama-3.2-11B-Vision](https://huggingface.co/meta-llama/Llama-3.2-11B-Vision-Instruct) | `llama3.2` | Pour les modèles multimodaux, les droits de la section 1(a) **ne sont pas accordés** aux personnes et entreprises domiciliées dans l'UE ([Acceptable Use Policy Llama 3.2](https://github.com/meta-llama/llama-models/blob/main/models/llama3_2/USE_POLICY.md)) | — | 11 G | `llama3.2-vision` : 7,8 Go | — | — |
| **Exclu** [Qwen2.5-VL-3B](https://huggingface.co/Qwen/Qwen2.5-VL-3B-Instruct) | `qwen-research` | **Non commercial** : « FOR NON-COMMERCIAL PURPOSES ONLY » ([LICENSE](https://huggingface.co/Qwen/Qwen2.5-VL-3B-Instruct/blob/main/LICENSE)) | 3,8 G | `qwen2.5vl:3b` | — | — |
| **Écarté** [Qwen2.5-VL-72B](https://huggingface.co/Qwen/Qwen2.5-VL-72B-Instruct) | `other` (licence non lue) | Non vérifié | 73 G | 72b 49 Go | — | — |
| **Écarté** Gemma 3 ([4B](https://huggingface.co/google/gemma-3-4b-it), [12B](https://huggingface.co/google/gemma-3-12b-it)) | `gemma` : conditions propres à Google, non lues en détail | Non vérifié dans ce document | 4 / 12 G | `gemma3` | Oui | — |

Remarques sur ce tableau :

- **[source]** Mistral indique pour Mistral-Small-3.2 « ~55 GB of GPU RAM in bf16 or fp16 » ([carte](https://huggingface.co/mistralai/Mistral-Small-3.2-24B-Instruct-2506)). Les étiquettes Ollama sont quantifiées. **La VRAM réelle en quantifié et le temps d'inférence sur CPU n'ont pas été mesurés.** Cela relève de #183 ou d'un `prototype`.
- **[source]** Gemma 4 E2B, E4B et 12B acceptent aussi l'**audio** en entrée ([carte Gemma 4](https://huggingface.co/google/gemma-4-12B-it)). En revanche, la prise en charge de l'audio par l'API compatible OpenAI d'Ollama est **non vérifiée**, et `rgaa-holo` n'envoie pas d'audio.
- **[inférence]** Compte tenu des licences, de l'appel déjà possible via Ollama et du français déclaré, trois candidats se détachent pour un prototype :
  - Mistral Small 3.2 (24B, 15 Go), le seul avec `fr` déclaré dans les métadonnées ;
  - Qwen3-VL 8B (6,1 Go) ;
  - Gemma 4 12B (7,7 Go).

  Aucun n'a de mesure publiée sur la tâche. Le choix final doit donc venir d'un banc de calibration en français (#183).

## 4. Littérature sur l'évaluation automatique de la pertinence d'une alternative

### 4.1 Mesures quantitatives trouvées

**Leonardi, Manca, Paternò — AVI 2026, « Leveraging Large Language Models for Alt-Text Evaluation in E-Commerce »** ([ACM](https://dl.acm.org/doi/full/10.1145/3811427.3811431))

- **Protocole [source] :**
  - Modèle : **GPT-4o** via Azure, température 0,7, avec le contexte DOM sur 5 niveaux de profondeur.
  - Tâche : note de 1 à 5 selon la technique WCAG G94.
  - Corpus : 157 images, 494 évaluations, 5 sites e-commerce internationaux (eBay, Amazon, Etsy, Decathlon, Nike).
  - Juges humains : 16 étudiants de master qui ont reçu un cours d'accessibilité de 1 h 30. Ce ne sont pas des experts.
- **Résultats [source] :**
  - Accord entre humains : Spearman 0,48, kappa pondéré 0,31.
  - Auto-cohérence du LLM : Spearman 0,66, kappa 0,65.
  - Accord LLM–humains : Spearman 0,59, kappa 0,55.
  - En classement binaire (notes 1–2 contre 3–5, comparées à la moyenne des notes humaines) : **exactitude 93,0 %**, précision 94,7 %, rappel 95,5 %, kappa 0,828 (VP 107, VN 39, FP 6, FN 5).
  - Le LLM est **plus indulgent** que les humains : moyenne 3,31 contre 2,65.
  - Le LLM **n'attribue jamais `alt=""`**, même aux images potentiellement décoratives.
  - Limite reconnue par les auteurs : la référence est construite par des étudiants et non par des experts.
- **Portée [inférence] :**
  - C'est la meilleure preuve disponible que le jugement de pertinence est *assistable*.
  - Elle ne transfère **pas** directement à un modèle local de 7 à 24B, au français, au RGAA, ni à des sites non marchands.

**Mähr & Twente, 2025, « Seeing History Unseen »** ([notice](https://exa.ai/library/publication/hfhz5k6sqpv))

- **Protocole [source] :** 100 documents patrimoniaux bâlois. Quatre VLM génèrent des alternatives, dont **Qwen3-VL-8B-Instruct**, seul modèle open-weight exécutable localement du lot, aux côtés de Gemini 2.5 Flash Lite, GPT-4o mini et Llama 4 Maverick.
- **Résultats [source] :**
  - 21 historiens classent les sorties ; les tests de Friedman et Wilcoxon ne montrent **aucune différence significative** entre modèles.
  - Erreurs récurrentes : mauvaise reconnaissance factuelle, omissions sélectives, reprise non critique d'une terminologie historique offensante.
- **Portée [inférence] :** l'étude porte sur la **génération** et non sur le **jugement**. C'est un indice, sans plus, que Qwen3-VL-8B n'est pas nettement inférieur aux API commerciales légères.

### 4.2 Études qualitatives et pistes sans mesure d'accord

- **Kreiss et al., 2022, « Context Matters for Image Descriptions for Accessibility »** ([arXiv 2205.10646](https://arxiv.org/abs/2205.10646)).
  - **[source]** Les métriques sans référence, de type CLIPScore, sont inadaptées parce qu'elles ignorent le contexte, alors que les personnes aveugles ou malvoyantes y tiennent. L'étude porte sur des participants aveugles ou malvoyants.
  - **[inférence]** Un score de similarité image–texte seul ne peut pas juger le test 1.3 ; il faut fournir le contexte de la page.
- **Noreskal & Feuilloley, W4A 2026, « Audit Website Images with LLMs »** ([ACM](https://dl.acm.org/doi/full/10.1145/3800424.3800428)).
  - C'est la seule source trouvée qui cible **explicitement le RGAA** (thème 1, distinction entre image informative et décorative).
  - **[source]** Les modèles sont propriétaires (Claude 3.7 et 4.5, GPT-5, Nova).
  - **[source]** Le classement binaire n'a été testé que sur **7 images, toutes informatives**. Les auteurs écrivent qu'il faut encore évaluer sur un jeu plus large.
  - **[source]** Un précédent pipeline des mêmes auteurs reposait sur des briques locales (YOLOv8, Florence-2, EasyOCR, BERT).
  - **[inférence]** Aucune mesure exploitable pour calibrer un niveau.
- **Leonardi, Manca, Paternò, GoodIT 2026, « A Hybrid Light-AI-based Framework for Accessibility Validation »** ([ACM](https://dl.acm.org/doi/10.1145/3794786.3830785)).
  - **[source]** Les auteurs préconisent une architecture « déterministe d'abord », où les petits modèles multimodaux locaux (exemple cité : Gemma 4) n'interviennent qu'en dernier recours, avec un humain en bout de chaîne.
  - **[source]** Ils estiment le palier multimodal à « minimum 32 GB of VRAM ». C'est leur hypothèse de dimensionnement, pas une mesure.
  - **[source]** Sur leur cas d'étude (WCAG 2.5.8), Claude Sonnet 4.6 et Gemma 4 E2B utilisés seuls produisent des faux positifs et des faux négatifs.
  - **[inférence]** Ce cadre est cohérent avec les niveaux Assisté et `review` retenus par la carte #330.
- **Aljedaani et al., W4A 2026, « Beyond Accuracy »** ([ACM](https://dl.acm.org/doi/10.1145/3800424.3800454)).
  - **[source]** Audit de GPT-4o, Gemini 1.5 Pro, Claude 3.5 Sonnet et Grok-1.5V.
  - **[source]** Les auteurs décrivent un « Plausibility Trap » : une hallucination complète paraît logique à qui ne voit pas l'image.
- **Mohanbabu & Pavel, ASSETS 2024, « Context-Aware Image Descriptions for Web Accessibility »** ([ACM](https://dl.acm.org/doi/10.1145/3663548.3675658)).
  - **[source]** Douze participants aveugles ou malvoyants préfèrent les descriptions GPT-4V tenant compte du contexte de la page, et les jugent meilleures (qualité, pertinence, plausibilité).
- **Retour d'expérience ANCT, « Espace sur demande »** ([billet](https://espacesurdemande.anct.gouv.fr/blog/ia-alternative-textuelle-image-llm-mistral-albert/)).
  - **[source]** Environ 6 000 alternatives d'images générées avec `mistralai/Mistral-Small-3.2-24B-Instruct-2506` via l'API Albert, suite au critère RGAA 1.1.
  - **[source]** L'auteur rapporte des résultats « supérieurs à ce que nous attendions », **sans mesure**. Il s'agit de génération, pas de jugement.

### 4.3 Ce qui n'a pas été trouvé (non vérifié)

- Aucune étude d'accord avec des **experts RGAA** pour un jugement de pertinence par VLM, open-weight ou propriétaire.
- Aucun banc sur la pertinence d'un **titre de page** (8.6), d'un **titre de tableau** (5.5) ou d'une **légende de `fieldset`** (11.7) jugée par un LLM.
- Aucun banc sur la pertinence d'une **audiodescription** ou d'une **transcription** (4.2) jugée par un modèle.
- Aucun banc VLM **en français** applicable à ces tâches.
- Des sources ont été repérées mais **non lues**, donc non utilisées :
  - Bassi et al. 2025, « An Assessment of LLM-Based Auditing and Validation for Web Accessibility » (doi:10.1145/3748699.3749805) ;
  - Paternò et al. 2025, « How an LLM Can Improve Automatic Web Accessibility Validation? » (GPT-4o) ;
  - Cardia et al., W4A 2026 (alternatives STEM) ;
  - un billet commercial de GotAlt du 2026-10-11 qui annonce des taux élevés (source non académique, modèle non identifié dans l'extrait).

## 5. Modes d'échec connus

| Mode d'échec | Établi par | Conséquence pour l'outil [inférence] |
|---|---|---|
| Indulgence systématique : le modèle note mieux que les humains | Leonardi et al. 2026 (moyenne 3,31 contre 2,65) | Ne jamais traduire une bonne note en `pass`. Le modèle sert à trier les cas probablement en échec. |
| Pas de détection du décoratif : jamais `alt=""` proposé | Leonardi et al. 2026, §6.5.1 | Le modèle ne doit pas trancher « porteuse d'information ou décorative ». Ce prérequis de 1.3 reste humain, ou passe par des heuristiques déterministes (critères 1.2 et 1.1, hors de ce ticket). |
| Hallucination plausible (« Plausibility Trap ») | Aljedaani et al. 2026 | Exiger une justification qui cite des éléments visibles et la montrer à l'auditeur avec la capture. Ne fait pas foi. |
| Erreurs factuelles, omissions, terminologie offensante | Mähr & Twente 2025 | Revue humaine obligatoire, surtout pour les images complexes (1.7). |
| Contexte de la page ignoré par les métriques sans référence | Kreiss et al. 2022 | Le prompt doit inclure le texte adjacent, le titre et la fonction (lien ou bouton). Un score CLIP seul ne suffit pas. |
| Faible accord entre humains : la tâche est subjective | Leonardi et al. 2026 (kappa 0,31 entre étudiants) | Une calibration exige une référence d'experts (#183), sinon le chiffre d'accord ne veut rien dire. |
| Injection d'instructions par du texte visible dans l'image | Anecdote du billet ANCT (déjouée dans ce cas-là) | Traiter le texte lu dans l'image comme une donnée ; schéma JSON strict en sortie. Aucune mesure publiée trouvée. |
| Non-déterminisme | Leonardi et al. 2026 (auto-cohérence kappa 0,65 à température 0,7) ; GoodIT 2026 | Température basse, plusieurs tirages ou vote, et provenance (modèle, version, prompt) dans la preuve. |

## 6. Niveau d'automatisation par test RGAA

Rappel de `CONTEXT.md` :

- **Automatique** : la machine rend seule un `fail` ou un `pass` déterministe.
- **Assisté** : la machine collecte la preuve et propose un verdict, rendu en statut `review`.
- **Irréductible** : rien d'utile au-delà de l'inventaire.

La carte #330 fixe en plus qu'un verdict de modèle reste Assisté tant qu'il n'est pas calibré.

| Test | Objet | Niveau atteignable | Technique et preuve [inférence, sauf mention] | Justification |
|---|---|---|---|---|
| 1.3.1 à 1.3.8 | Pertinence de l'alternative (`img`, `area`, `input type=image`, `object`, `embed`, `svg`, `canvas`) | **Assisté** | Recadrage de l'image ou de la zone à partir de la capture unique de la page, plus l'alternative, le texte adjacent, la fonction et le titre de la page, envoyés au VLM local (Mistral Small 3.2, Qwen3-VL 8B ou Gemma 4 12B, à départager). Sortie : un indice `probablement non pertinente` / `plausible` avec justification. Statut `review`. | Seule mesure publiée : GPT-4o, en anglais, contre des non-experts. Rien pour un modèle local ni pour le français. Indulgence et décoratif mal géré. |
| 1.3.9 | Alternative « courte et concise » | **Assisté** | Signal déterministe de longueur (au-delà de 80 caractères) et jugement du modèle sur la redondance avec le texte adjacent. Statut `review`. | **[source]** Le glossaire RGAA « Alternative courte et concise » indique : « Une longueur maximale de 80 caractères est fortement recommandée » ([glossaire](https://accessibilite.numerique.gouv.fr/methode/glossaire/#alternative-textuelle-courte-et-concise)). C'est une recommandation, pas un seuil d'échec, et la concision ne se réduit pas à la longueur. Le dépassement reste donc un indice à revoir, pas un `fail` automatique [inférence]. |
| 1.7.1 à 1.7.6 | Pertinence de la description détaillée | **Assisté**, avec un indice de moindre fiabilité | Image plus description détaillée liée, envoyées au VLM pour repérer les omissions et les contradictions. Statut `review`. | Les images qui demandent une description détaillée (schémas, graphiques) sont celles où les erreurs factuelles et les omissions sont documentées (Mähr & Twente). Aucune mesure d'accord. |
| 5.5.1 | Pertinence du titre d'un tableau de données | **Assisté** | Un **LLM texte** suffit : titre, `caption` et en-têtes de colonnes. Le VLM est inutile. Statut `review`. | Aucun banc publié trouvé. |
| 8.6.1 | Pertinence du `<title>` | **Assisté** | LLM texte avec `<title>`, `h1` et un résumé du contenu. Statut `review`. Les titres vides, génériques ou identiques d'une page à l'autre sont des signaux déterministes qui relèvent de la spec de couverture (#247), pas d'un VLM. | Aucun banc publié trouvé. |
| 11.7.1 | Pertinence de la `legend` d'un regroupement de champs | **Assisté** | LLM texte avec la `legend` et les libellés du `fieldset`. Statut `review`. | Aucun banc publié trouvé. |
| 4.2.1 | Transcription d'un média audio seul | **Assisté** (si le téléchargement des médias est activé) | Reconnaissance vocale locale, puis comparaison avec la transcription fournie : couverture et écarts. Le VLM ne sert pas. | Les briques existent sous licence compatible : poids [Whisper large-v3](https://huggingface.co/openai/whisper-large-v3) sous Apache-2.0 et [whisper.cpp](https://github.com/ggml-org/whisper.cpp) sous MIT [source]. Aucune mesure de fiabilité pour ce jugement. Par défaut, les médias ne sont pas téléchargés (décision #330), donc on reste à l'inventaire. |
| 4.2.2, 4.2.3 | Audiodescription ou alternative d'une vidéo, d'un média synchronisé | **Assisté faible, à prototyper ; sinon Irréductible** | Images clés extraites par le sidecar `ffmpeg`, plus le texte d'audiodescription, envoyés au VLM pour repérer des événements visuels non décrits. | Aucune source ne mesure ce jugement. Juger qu'une audiodescription est « pertinente » demande de comprendre le récit dans la durée [inférence]. À trancher par un `prototype` avant de promettre plus que l'inventaire. |
| — | Tous les tests ci-dessus | **Automatique exclu** | — | Aucune preuve publiée d'accord suffisant avec des experts pour un modèle local. Le ticket et la carte exigent une telle preuve pour passer en Automatique. |

## 7. Recommandations pour la spec (inférences)

1. **Corriger d'abord** la validation base64 de `multimodal_messages` (tampon de 64 octets, §2). Sinon aucun appel VLM réel ne part.
2. **Ne demander au VLM que ce qu'il sait faire** : vérifier la cohérence entre une image et un texte **dans le contexte de la page**. Ne pas lui confier la décision « image décorative ou porteuse d'information ».
3. **Sortie rendue en statut `review`**, avec une **priorité** déduite du verdict du modèle. Cela s'appuie sur l'asymétrie mesurée : le modèle est bon pour repérer les mauvaises alternatives et trop indulgent sur les autres.
4. **Utiliser un LLM texte** (déjà disponible via `rgaa-holo`) pour 5.5, 8.6 et 11.7. Un VLM est inutile pour ces tests.
5. **Shortlist pour le `prototype` et le banc de calibration de #183**, avec un corpus français annoté par des auditeurs RGAA et une mesure du kappa contre ces experts :
   - Mistral Small 3.2 24B ;
   - Qwen3-VL 8B ;
   - Gemma 4 12B ;
   - Gemma 4 E4B ou Qwen3-VL 4B comme option légère pour CPU.

   Tous sont sous Apache-2.0 et disponibles dans Ollama.
6. **Inscrire dans la preuve** : le modèle et son étiquette Ollama, le prompt, la température, le recadrage de l'image et le contexte transmis.

## 8. Incertitudes restantes

- **Qualité en français** des candidats sur ces tâches : non vérifiée, aucun banc trouvé.
- **VRAM et latence réelles** en quantifié, sur la machine d'audit : non mesurées.
- **Licences à confirmer :**
  - InternVL3 : métadonnées contradictoires ;
  - MiniCPM-V 2.6 sur Ollama : dépôt HF à accès restreint ;
  - Gemma 3 et Qwen2.5-VL-72B : conditions non lues.
- **Qwen3-VL dans llama.cpp** : absent de la liste de `docs/multimodal.md`. La prise en charge réelle n'a pas été vérifiée par ailleurs.
- **Audio en entrée de Gemma 4 via l'API compatible OpenAI d'Ollama** : non vérifié.
- **Transférabilité des chiffres de Leonardi et al.** (GPT-4o, anglais, e-commerce, juges non experts) à un modèle local, en français, sur des sites publics : inconnue. C'est précisément ce que le banc de #183 doit mesurer.
