# Modèles API cibles (Holo 4, Qwen 3.8) — vision, computer use, conditions d'usage

Ticket : [#349](https://github.com/jamon8888/Holo-RGAA/issues/349) — carte parente [#330](https://github.com/jamon8888/Holo-RGAA/issues/330).
Recherche menée le 2026-10-12, sur `origin/master` `50b4271`. Glossaire : `CONTEXT.md` (branche `docs/human-criteria-automation`).

Légende :

- **[source]** : fait établi par la source citée juste après.
- **[inférence]** : déduction de l'auteur, non publiée par une source.
- **non vérifié** : fait qu'il n'a pas été possible de confirmer.
- Les scores de bancs sont ceux **publiés par les éditeurs eux-mêmes** (H Company, Qwen) ; aucun n'a été refait ici.

## Réponse courte

1. **Les deux modèles existent.** « Qwen 3.8 » n'apparaît pas dans le code, mais c'est une génération réelle d'Alibaba (voir §1). **Holo 4** est une famille de H Company, publiée le 2026-09-28, dont **Holo4-27B est construit sur Qwen3.8-27B** et Holo4-35B-A3B sur Qwen3.6-35B-A3B. Les deux ne sont donc pas des candidats indépendants : Holo 4 est un affinage « computer use » de Qwen.
2. **Le code est à un changement d'identifiant de Holo 4** : le fournisseur `holo3` de `rgaa-core` pointe déjà sur `https://api.hcompany.ai/v1` (API compatible OpenAI), la même base que la Models API de H. Que **MyIA** serve Holo 4 ou Qwen 3.8 est **non vérifié**.
3. **Capacités établies** : entrée image (jusqu'à 5 par requête), ancrage d'éléments (coordonnées normalisées [0, 1000], documenté), appel de fonctions natif, contexte 262 144 jetons (Holo 4). Le français n'est **pas documenté** pour Holo ; aucune mesure de qualité en français trouvée.
4. **Aucun test ne change de niveau grâce au computer use seul** — c'est la conclusion honnête. Le computer use sert à **provoquer des états** (survol, activation, saisie erronée, soumission) pour produire des preuves `review` plus riches sur 7.4, 7.5, 11.10–11.12, 12.11, 13.11 ; il ne produit pas de verdict `pass`. L'ancrage est **redondant** sous Obscura, qui donne déjà la géométrie réelle (`DOM.getBoxModel`).
5. **Sous Obscura 0.2.4, l'agent est aveugle sur le clavier et le focus** (faits mesurés de la carte) : pas de navigation Tab native, pas de repeinture de `:focus`, `Input.dispatchTouchEvent` sans effet. 12.8, 12.9, 10.7 et 13.10–13.12 (partie geste) ne gagnent rien.
6. **Données** : H affirme une rétention nulle des invites et captures sur la Models API ; sa politique de confidentialité publiée parle, pour l'extension, de 30 jours (EEE) pour les entrées et sorties. **Contradiction non levée**, région d'hébergement de la Models API **non documentée**. Voir §5.
7. **Licence** : Holo4-35B-A3B = Apache-2.0 ; **Holo4-27B = CC BY-NC 4.0** (poids non commerciaux). L'usage par API relève des conditions d'utilisation, pas de la licence des poids.

## 1. Existence, noms exacts, rapport avec le code

### Holo 4 (H Company)

- Annonce le 2026-09-28, deux tailles : 27B dense et 35B-A3B (MoE). [source : [billet de lancement](https://hcompany.ai/newsroom/holo4)]
- Identifiants API : `holo4-27b`, `holo4-35b-a3b` (palier payant), `holo3-1-35b-a3b` (palier gratuit), `holo3-122b-a10b`. [source : [Models API — introduction](https://hub.hcompany.ai/models-api/introduction#models)]
- Base : `holo4-27b` = `Qwen/Qwen3.8-27B` ; `holo4-35b-a3b` = `Qwen/Qwen3.6-35B-A3B`. [source : cartes Hugging Face [Hcompany/Holo4-27B](https://huggingface.co/Hcompany/Holo4-27B) et [Hcompany/Holo4-35B-A3B](https://huggingface.co/Hcompany/Holo4-35B-A3B)]
- Poids ouverts pour les deux (BF16, FP8, NVFP4, GGUF Q4). [source : [Local inference](https://hub.hcompany.ai/models-api/local-inference#holo4)]
- Holotron4-30B-A3B existe aussi (architecture NemotronH), hors sujet ici. [source : carte Holo4-35B-A3B]

### Qwen 3.8 (Alibaba)

« Qwen 3.8 » est une génération, pas un seul modèle. Noms exacts trouvés sur le Hub : `Qwen/Qwen3.8-27B` (14 août 2026), `Qwen3.8-Flash-Next`, `Qwen3.8-2.4T-A95B` (12 août 2026), et leurs variantes FP8. [source : Hub Hugging Face, organisation [Qwen](https://huggingface.co/Qwen)]

- **Qwen3.8-27B** : dense, modèle vision-langage natif (images et vidéos), Apache-2.0, contexte 262 144 natif extensible à 1 000 000. [source : [carte Qwen3.8-27B](https://huggingface.co/Qwen/Qwen3.8-27B)]
- **Qwen3.8-Max** : version API d'Alibaba Cloud (annoncée 2026-08-02) ; entrée texte, image, vidéo. Source **secondaire** (presse) : [The Batch](https://www.deeplearning.ai/the-batch/qwen3-8-max-lands-with-a-bang) ; la page éditeur n'a pas été relue.
- Le billet de Holo 4 cite « Qwen3.8 Max » comme comparatif à 86,1 % sur OSWorld « tel que rapporté par Qwen ». [source : billet Holo4]

### Rapport avec le code existant

- `rgaa-rs/crates/rgaa-core/src/provider.rs` : fournisseur `holo3` → `https://api.hcompany.ai/v1`, modèle par défaut `holo3-1-35b-a3b`, clé `HOLO3_API_KEY`. Le fournisseur `myia` → `https://api.medium.text-generation-webui.myia.io/v1`, modèle par défaut `swift-1.5-27b`, alias `qwen3.6-35b-a3b` accepté (`.env.example`).
- **[inférence]** Passer à Holo 4 direct chez H = changer `HOLO3_MODEL` en `holo4-35b-a3b` ou `holo4-27b`. Le format d'appel de Holo 4 diffère toutefois de Holo3.1 (voir §2, « appel de fonctions seulement »).
- **non vérifié** : le catalogue de MyIA. Aucune clé n'a été utilisée pour l'interroger ; rien dans le dépôt n'indique qu'il serve Holo 4 ou Qwen 3.8.
- Le défaut connu du tampon de 64 octets dans `multimodal_messages` bloque toute image réelle ; il est **traité ailleurs** et n'est pas corrigé ici.

## 2. Capacités

| Capacité | Holo 4 | Qwen3.8-27B |
|---|---|---|
| Entrée image | oui, ≤ 5 par requête, JPEG/PNG/WebP [[docs](https://hub.hcompany.ai/models-api/introduction#models)] | oui, natif [[carte](https://huggingface.co/Qwen/Qwen3.8-27B)] |
| Vidéo | non (`video: 0` dans la configuration vLLM recommandée) [[docs](https://hub.hcompany.ai/models-api/local-inference#holo4)] | oui [[carte](https://huggingface.co/Qwen/Qwen3.8-27B)] |
| Ancrage d'éléments | oui, page dédiée, un tour sans raisonnement, coordonnées entières [0, 1000] [[docs](https://hub.hcompany.ai/models-api/element-localization)] | pas de score ni de page d'ancrage dans la carte : **non vérifié** |
| Computer use | oui, objectif du modèle [[carte](https://huggingface.co/Hcompany/Holo4-27B)] | OSWorld-Verified 84,3, AndroidWorld 81,9, WebArena-Verified 64,8 (auto-déclarés) [[carte](https://huggingface.co/Qwen/Qwen3.8-27B)] |
| Appel de fonctions | oui ; **seul format accepté** par Holo4 (pas de sorties structurées) [[core concepts](https://hub.hcompany.ai/models-api/build-an-agent/core-concepts)] | oui sur l'API hébergée [[Qwen Cloud](https://www.qwencloud.com/models/qwen3.8-27b)] |
| Contexte | 262 144 jetons [[docs](https://hub.hcompany.ai/models-api/introduction#models)] | 1 000 000 hébergé [[Qwen Cloud](https://www.qwencloud.com/models/qwen3.8-27b)] |
| Français | **non documenté** ; la page OCR de H classe « anglais, numérique » comme « le plus fort » [[docs](https://hub.hcompany.ai/models-api/document-ocr)] | la carte ne chiffre pas le français : **non vérifié** |

Conventions de la boucle Holo (à respecter, sinon la qualité baisse) [source : [Core concepts](https://hub.hcompany.ai/models-api/build-an-agent/core-concepts)] :

- tâche en premier message utilisateur ; chaque capture revient dans un message `<observation>` ; **3 captures au maximum** gardées en contexte ;
- l'outil `answer` est le seul signal d'arrêt ;
- coordonnées en entiers [0, 1000], à remettre à l'échelle des octets **exacts** envoyés (tout redimensionnement après coup décale les clics) ;
- température 0,8 en boucle, 0,0 pour l'ancrage seul ; le raisonnement (`message.reasoning`) est lu, jamais rejoué.

Côté Models API, **c'est notre harnais qui définit les outils** (clic, saisie, défilement, touches…) ; le modèle ne fait que renvoyer l'appel. [source : [Browser agent](https://hub.hcompany.ai/models-api/cookbooks/browser-agent), exemple Playwright à cinq outils : `click`, `type`, `scroll`, `goto`, `answer`]. L'API Agents de H (navigateur hébergé) expose un jeu d'actions fixe qui inclut `press_keys_web`, `move_mouse_web`, `select_option`. [source : [Browser configuration](https://hub.hcompany.ai/agents-api/browser/configuration)]

## 3. Comment l'agent agirait sur une page rendue par Obscura

Boucle [inférence, calquée sur le cookbook de H] :

1. `Page.captureScreenshot` → octets PNG/JPEG ; dimensions connues (viewport fixe, par exemple 1280×800 comme dans le cookbook).
2. Envoi à l'API (`<observation>` + tâche), `tool_choice: required`.
3. Traduction de l'appel : `click(x, y)` → `Input.dispatchMouseEvent` (`mouseMoved`, `mousePressed`, `mouseReleased`) à `x/1000 × largeur` ; `type` → `Input.insertText` ou `Input.dispatchKeyEvent` ; `scroll` → `mouseWheel` ; touches → `dispatchKeyEvent`.
4. Nouvelle capture, retour à l'étape 2, jusqu'à `answer` ou plafond d'étapes.

Ce que les faits mesurés d'Obscura 0.2.4 changent [source : carte #330, Notes ; non remesurés ici] :

- **Navigation Tab native absente** (`keydown` livré, mais le focus ne bouge pas) : un agent qui « appuie sur Tab » ne verra rien bouger. Le harnais devrait **émuler** le Tab par script (calcul de l'ordre, `focus()`), ce qui teste notre émulation et non le navigateur. **[inférence]** → 12.8 et 12.9 n'y gagnent rien par rapport à un calcul déterministe de l'ordre.
- **`:focus` ne correspond pas et le focus n'est pas repeint** : une capture ne montre jamais l'indicateur de focus. Le modèle ne peut pas juger 10.7 sur ces captures, quelle que soit sa qualité.
- **`Input.dispatchTouchEvent` sans effet** : pas de multipoint ni de geste tactile → 13.10 (partie geste) hors de portée.
- Souris : l'effet réel de `Input.dispatchMouseEvent` sous Obscura (survol, clic, `mousedown`) n'est **pas dans les faits mesurés** de la carte → **non vérifié**, à mesurer avant tout prototype.
- Rendu : si les captures Obscura ne reflètent pas fidèlement les états dynamiques, l'agent hérite de ce défaut. **non vérifié** pour les états survol et menus.
- L'API Agents de H fait tourner **son propre** navigateur hébergé ; elle ne pilote pas Obscura. Un mode « contrôle local » attache le SDK à un Chrome en débogage distant (port 9222) [source : [Local control](https://hub.hcompany.ai/agents-api/browser/local-control)] ; qu'Obscura puisse jouer ce rôle est **non vérifié**. Envoyer nos pages à un navigateur hébergé chez H change aussi le périmètre des données (§5).

## 4. Réponse par famille de tests de l'inventaire

Base : [inventory.md](https://github.com/jamon8888/Holo-RGAA/blob/docs/human-criteria-automation/docs/wayfinder/human-criteria/inventory.md) (106 tests, 5 familles). « Niveau visé » = niveau atteignable **avec** computer use ou ancrage, par rapport aux conclusions déjà prises sur la carte.

| Famille | Effet du computer use / de l'ancrage | Niveau visé |
|---|---|---|
| **G1 axe** (8 tests : 5.4.1, 5.7.4, 6.1.5, 7.1.1, 9.1.3, 10.8.1, 12.8.1, 13.9.1) | Aucun : règles DOM déterministes. 12.8.1 (ordre de tabulation) : seule la sémantique est vérifiable ; l'ordre n'est pas observable sous Obscura (§3). | inchangé ; 12.8.1 reste Assisté |
| **G2 pertinence** (27 tests : 1.3.x, 1.4.x, 1.7.x, 5.2.1, 5.5.1, 8.6.1, 11.7.1, 11.8.3…) | Besoin d'un VLM, pas d'un agent : une capture ou une image suffit. L'ancrage n'apporte rien (l'élément est déjà connu par le DOM). Holo 4 contre Mistral ou Qwen3-VL locaux sur la pertinence d'alternatives : **aucune mesure trouvée**. 1.4.x (CAPTCHA) : les conditions d'utilisation de H interdisent de contourner les CAPTCHA avancés ([terms-of-use](https://hcompany.ai/terms-of-use)) → l'agent ne doit pas servir à les résoudre. | Assisté, comme #333 ; 1.4.x : décision de fond, pas d'automatisation par agent |
| **G3 multimédia** (15 tests : 4.x) | Holo 4 ne prend pas de vidéo ; Obscura ne lit pas les médias. Un agent pourrait manœuvrer les commandes d'un lecteur, mais rien ne se lit sous Obscura. Qwen3.8-27B accepte la vidéo mais sans score publié sur nos tests. | inchangé (#334) |
| **G4 visuels** (20 tests : 1.8.x, 3.1.x, 3.3.x, 10.10.x) + 10.7 hors périmètre | Mesure par pixels (#335) ; l'ancrage localiserait les éléments focalisables mais la géométrie est déjà disponible. 10.7 : le focus n'est pas repeint sous 0.2.4 → l'agent ne le voit pas. | inchangé ; 10.7 non résolu par ce biais |
| **G5 compréhension** (36 tests) | Seule famille où l'interaction compte, **pour la preuve, pas pour le verdict** : voir détail ci-dessous. | Assisté (preuve plus riche) ; aucune promotion établie |

Détail G5 (tout est **[inférence]**, rien n'a été essayé) :

- **7.4.1** (changement de contexte), **7.5.1** (messages de statut), **11.10 / 11.11** (indication d'erreur, format suggéré) : l'agent peut saisir une valeur, soumettre, et rapporter ce qui apparaît (captures avant/après). Le verdict de pertinence reste humain → **Assisté**. 7.4.1 est en grande partie observable sans modèle (événements de navigation CDP).
- **11.12.1** (formulaires qui modifient ou suppriment des données) : **risque d'effets de bord réels** sur le site client. À ne jamais soumettre pour de vrai ; l'agent ne peut que décrire le formulaire. Reste **Assisté** par inspection.
- **12.11.1** (contenus au survol ou au focus) : le survol peut être provoqué par la souris CDP (effet non vérifié, §3) ; la partie focus est aveugle sous 0.2.4.
- **13.11.1** (pointeur unique) : la détection d'une action sur `mousedown` est déjà possible de façon déterministe (#337 : « échec sur action à l'appui »). L'agent ne change rien au niveau.
- **13.10.x / 13.12.x** : gestes multipoints et capteurs hors de portée sous Obscura. 13.12 « non applicable » si aucun capteur n'est écouté (#337).
- **12.9** (piège clavier, cité par le ticket, absent de l'inventaire) : demande un vrai Tab ; même conclusion que 12.8.
- **12.10.1** (raccourcis à une touche) : les `keydown` sont livrés, donc détection de gestionnaires possible sans agent.
- **13.1.4** (limite de session), **13.6.1**, **12.3.1**, **8.7.1** : attente longue ou jugement de fond ; le computer use n'y change rien.

## 5. Conditions d'usage des données

### H Company

- **Rétention (docs)** : la Models API journalise l'heure, le modèle et le nombre de jetons ; les invites et réponses ne sont pas stockées (rétention nulle par défaut) ; H fait tourner ses propres modèles et ne partage pas entrées et sorties avec des tiers (traduction). [source : [Models API — Data retention](https://hub.hcompany.ai/models-api/introduction#data-retention) ; [Security and privacy](https://hub.hcompany.ai/holo-desktop-cli/security-and-privacy)]. H se dit certifié SOC 2 et ISO 27001 (même page ; certificats **non vérifiés**).
- **Rétention (politique de confidentialité)** : pour l'extension, les entrées et sorties sont supprimées 30 jours après l'opération pour les utilisateurs de l'EEE, 90 jours hors EEE ; les données techniques 6 mois, pseudonymisées [source : [privacy-policy](https://hcompany.ai/privacy-policy)]. Le même document liste « l'entraînement des modèles d'IA de H Company » comme finalité à base d'intérêt légitime, sauf opposition. **Contradiction avec la page de la Models API non levée** ; savoir laquelle s'applique à un contrat API est **non vérifié**.
- **Rôle RGPD** : H est responsable de traitement pour ses propres données de compte ; pour le contenu soumis par un utilisateur professionnel, H agit comme sous-traitant selon un « Personal Data Processing Addendum » [source : privacy-policy, [terms-of-use](https://hcompany.ai/terms-of-use)]. **Le texte de l'addendum n'a pas été lu** → clauses réelles non vérifiées. Liste des sous-traitants annoncée à `trust.hcompany.ai/subprocessors` (non consultée).
- **Hébergement** : l'API Agents a des régions UE (défaut, `agp.eu.hcompany.ai`) et US, isolées ([API reference](https://hub.hcompany.ai/agents-api/api-reference)). La **Models API** (`api.hcompany.ai`) ne documente **aucune région** ; la politique de confidentialité mentionne une « infrastructure basée aux États-Unis » avec clauses contractuelles types. **Hébergement UE de la Models API : non vérifié.**
- **Restrictions d'usage** : interdit de contourner les protections techniques de sites tiers (CAPTCHA avancés, limitations de débit, anti-bot) et de moissonner à grande échelle des données non publiques ; l'utilisateur garantit détenir les droits sur les entrées ; droit français [source : terms-of-use]. Auditer les pages de clients **avec leur accord** relève de cette garantie (point juridique à confirmer avec un juriste, non tranché ici).
- **Responsabilité** : plafonnée à 100 EUR (gratuit) ou au plus élevé de 100 EUR et des frais des 6 derniers mois (payant) [source : terms-of-use]. Crédits valables 12 mois [source : terms-of-use ; [API reference](https://hub.hcompany.ai/models-api/api-reference#rate-limits)].
- **Chez le client** : une capture peut contenir des données personnelles (noms, e-mails, champs saisis). **[inférence]** Il faut un accord de sous-traitance avec H, une minimisation (pas de pages authentifiées, masquage), et l'information du client.

### Alibaba Cloud (Qwen 3.8 hébergé)

- Documentation officielle du modèle : régions **Chine (Pékin)** et **Singapour** ; Francfort **absent** de la page lue [source : [Model Studio — Qwen3.8-27B](https://www.alibabacloud.com/help/en/model-studio/qwen3-8-27b)]. Un résultat de recherche secondaire évoque une région Francfort pour d'autres modèles : **non vérifié** pour 27B.
- Aucune clause RGPD, accord de traitement ni politique de rétention d'Alibaba Cloud n'a été lu → **non vérifié**. L'accès depuis Singapour ou Pékin impliquerait un transfert hors UE **[inférence]**.
- La carte Hugging Face indique que l'hébergement Qwen Cloud est « à venir » alors que la page Qwen Cloud affiche déjà un prix : incohérence **non levée**.

### Licence des poids

- Holo4-35B-A3B : Apache-2.0. Holo4-27B : **CC BY-NC 4.0**, non commercial (la doc de H écrit « usage de recherche »). Qwen3.8-27B : Apache-2.0 [source : cartes Hugging Face]. **[inférence]** Auto-héberger Holo4-27B dans un produit commercial n'est pas permis par sa licence ; utiliser son API relève des conditions d'utilisation de H.

## 6. Prix et quotas publiés

| Modèle | Entrée / sortie (par 1 M de jetons) | Entrée en cache | Accès | Source |
|---|---|---|---|---|
| `holo4-35b-a3b` | 0,30 $ / 2,00 $ | 0,03 $ | payant | [Models API](https://hub.hcompany.ai/models-api/introduction#models) |
| `holo4-27b` | 0,40 $ / 3,00 $ | 0,04 $ | payant | idem |
| `holo3-1-35b-a3b` | 0,25 $ / 1,80 $ | 0,025 $ | **gratuit, 10 requêtes/min** ; contexte 65 536 ; sortie ≤ 8 192 | idem |
| `qwen3.8-27b` (Singapour) | 0,5 $ / 3 $ | 0,1 $ implicite | non précisé | [Model Studio](https://www.alibabacloud.com/help/en/model-studio/qwen3-8-27b) ; [Qwen Cloud](https://www.qwencloud.com/models/qwen3.8-27b) : 5 M jetons/min, 5 000 requêtes/min |

- Palier payant de H : « limites plus élevées » **sans chiffre publié** [source : [API reference](https://hub.hcompany.ai/models-api/api-reference#rate-limits)] → quota exact non vérifié.
- Le coût par capture (jetons d'image) n'est pas publié à ma connaissance : **non vérifié**. Coût par tâche annoncé par H sur OSWorld : 0,08 $ (Holo4-27B) ; c'est une tâche de bureau, pas un audit RGAA.
- Le billet de H donne aussi des prix, mais les chiffres relus diffèrent de la page de documentation ; je retiens la **page de documentation** (source la plus directe).

## 7. Fiabilité publiée

Tous les chiffres sont **auto-déclarés** par l'éditeur [source : [billet Holo4](https://hcompany.ai/newsroom/holo4) ; [carte Qwen3.8-27B](https://huggingface.co/Qwen/Qwen3.8-27B)].

| Banc | Holo4-27B | Holo4-35B-A3B | Qwen3.8-27B |
|---|---|---|---|
| OSWorld | 85,2 % (0,08 $/tâche) | 80,8 % | 84,3 % |
| OSWorld 2.0 (flux longs) | 61,7 % de score, **41,5 % de réussite** | 30,9 % | non publié |
| AndroidWorld | 85,1 % | 77,6 % | 81,9 % |
| Agentic Task Factory web / MCP / bureau | 80,2 / 89,4 / 72,0 % | 70,8 / 85,4 / 64,8 % | — |

- Holo 4 est évalué dans le harnais de H (moyenne de 2 à 4 passages, un seul sur OSWorld 2.0). Les autres modèles viennent de classements publics, avec des harnais et efforts différents. Le billet reconnaît que sur les flux longs Holo 4 reste derrière les meilleurs modèles fermés (OSWorld 2.0 : 61,7 % contre 81,8 % pour Opus 5.5).
- **Aucun banc publié ne mesure** ce dont nous avons besoin : jugement d'accessibilité, français, pages réelles avec états de focus, ou usage via CDP d'Obscura. Les scores de **ScreenSpot-Pro / OSWorld-G** sont mentionnés pour Holo3.1 mais sous forme de graphique : **chiffres non extraits**.
- Les trajectoires d'évaluation de Holo 4 sont publiques ([Hcompany/trajectories](https://huggingface.co/datasets/Hcompany/trajectories)) : utile pour inspecter les échecs, non exploité ici.
- Limite connue : le mode rapide (`--fast`) est « moins précis sur les interfaces délicates » [source : [Quickstart HoloDesktop](https://hub.hcompany.ai/holo-desktop-cli/getting-started/quickstart#models)]. Les clics se décalent si la capture est redimensionnée après coup [source : [Element localization](https://hub.hcompany.ai/models-api/element-localization)].

## 8. Ce qu'il reste à mesurer avant de décider

1. Effet réel de `Input.dispatchMouseEvent` et fidélité des captures sous Obscura 0.2.4 (survol, menus, formulaires). Un prototype d'une boucle à un outil suffit.
2. Qualité en français d'un modèle Holo 4 ou Qwen 3.8 sur nos fixtures (alternatives, titres) : à comparer aux candidats locaux de #333.
3. Texte de l'addendum de traitement des données de H, région d'hébergement de la Models API, applicabilité de la rétention de 30 jours.
4. Si MyIA expose Holo 4 ou Qwen 3.8, et à quelles conditions.
5. Consommation réelle en jetons par capture et par étape, pour chiffrer un audit.
