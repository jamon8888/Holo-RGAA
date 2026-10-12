# Addendum de traitement des données et garanties réelles des fournisseurs de modèles (H Company, Alibaba Cloud)

Recherche du ticket #351 (carte #330). Lecture documentaire de textes contractuels, **pas un avis juridique** : aucune conclusion de conformité n'est tirée. Les passages qui demandent une interprétation sont regroupés au §7 pour relecture par une personne compétente.

Date de consultation : 2026-10-12. Les pages ont été récupérées en texte brut (HTML intégral), puis lues ; les extraits entre guillemets sont courts et volontairement limités.

Légende : **[cité]** = écrit dans le texte consulté (URL donnée) ; **[inférence]** = déduction de l'auteur, à confirmer ; **[inconnu]** = non trouvé, non lisible ou non consulté.

## 1. Synthèse

1. **H Company : un DPA existe et a été lu** (page publique, « Last updated: June 2026 »). H y est **sous-traitant** pour les données personnelles du client professionnel. Il prévoit aussi une **réutilisation des données pour l'entraînement des modèles de H, sauf opposition expresse du client** (§ « Réutilisation des Données »). Ce n'est pas un engagement de non-entraînement ; le mécanisme d'opposition pour un client de la Models API par clé API **n'est pas documenté** dans les textes lus.
2. **La contradiction sur la rétention n'est pas levée, elle s'élargit** : trois énoncés différents pour les entrées/sorties (zéro rétention ; 30 jours EEE / 90 jours hors EEE ; durée du contrat dans la sous-annexe A du DPA), plus 30 jours par défaut pour l'API Agents. Les textes ne disent pas lequel s'applique à la Models API utilisée par clé API (§5).
3. **Hébergement de H** : la Models API n'a toujours **aucune région documentée**. Le DPA cite AWS (« Union Européenne (Région Europe) ») comme hébergeur principal, avec du résiduel « États-Unis » pour certaines grappes GPU d'inférence ; la politique de confidentialité parle d'« infrastructure basée aux États-Unis ». Huit sous-traitants sont nommés dans le DPA (§2.5). La liste officielle du Trust Center est **illisible sans JavaScript**.
4. **Alibaba Cloud (Model Studio)** : engagement écrit de non-entraînement sans consentement séparé **[cité]** (conditions produit §4.48.1(e), politique de rétention §5.2), DPA publiés (rôle de sous-traitant), rétention par défaut « no longer than 30 days », option ZDR réservée aux clients entreprise du site international sur demande.
5. **Mais `qwen3.8-27b` n'est listé ni à Francfort ni dans la liste des modèles ZDR** (pages du 2026-10-11 et 2026-10-12) : seules Pékin (Chine continentale) et Singapour (International) sont listées pour ce modèle. Francfort existe comme région Model Studio (portées Global ou EU), mais ce modèle n'y figure pas. Sur les textes lus, **l'exigence « région UE documentée » n'est donc pas remplie pour `Qwen3.8-27B` chez Alibaba**.
6. **Hébergeur UE tiers (MyIA)** : le site public myia.io ne publie ni DPA, ni politique de confidentialité, ni région d'hébergement, ni catalogue Holo/Qwen3.8 ; ces textes sont **introuvables** (§4).

## 2. H Company

### 2.1 Textes lus (tous publics, aucun derrière authentification)

| Texte | URL | Date affichée | Remarque |
|---|---|---|---|
| Accord de traitement de données (DPA, en français) | https://hcompany.ai/dpa | « Last updated: June 2026 » | Annexe de l'article 28 RGPD, renvoie à l'article « Données personnelles » des CGV |
| Conditions Générales de Services (CGV) | https://hcompany.ai/cgv | « Last updated: June 2026 » | Contient l'article « Droit de réutilisation des Données » (le DPA le cite comme 12.3) |
| Conditions Générales d'Utilisation (CGU) | https://hcompany.ai/cgu | « Last updated: June 2026 » | Renvoie aux CGV pour les données chargées ou générées |
| Terms of Service (anglais, HoloTab + « Inference API ») | https://hcompany.ai/terms-of-use | « Last updated: April 14, 2026 » | Cité par #349 comme « terms-of-use » |
| Privacy Policy | https://hcompany.ai/privacy-policy | « Last updated 13-04-2026 » **et** « Last Updated: June 24, 2026 » (les deux en tête de page) | Deux dates dans le même document |
| Docs Models API, § Data retention | https://hub.hcompany.ai/models-api/introduction#data-retention | non affichée | Lue via l'outil MCP de la doc H |
| Docs HoloDesktop CLI, Security and privacy | https://hub.hcompany.ai/holo-desktop-cli/security-and-privacy | non affichée | |
| Docs Agents API, référence et changelog 1.0.7 | https://hub.hcompany.ai/agents-api/api-reference ; https://hub.hcompany.ai/agents-api/changelog | changelog : 16 juillet 2026 | Régions, rétention des sessions |
| Trust Center / sous-traitants | https://trust.hcompany.ai/subprocessors | n/a | **[inconnu]** : page rendue en JavaScript (hébergée par Vanta), aucun contenu lisible récupéré. La Sous-annexe B du DPA renvoie à https://trust.hcompany.ai pour les mesures de sécurité : **non lues** |

### 2.2 Rôle (responsable / sous-traitant)

- DPA, « Qualification des Parties » **[cité]** : H Company est le « Sous-traitant » du Client, lui-même responsable de traitement ou sous-traitant de son propre client (H devient alors « Sous-traitant ultérieur »). Le Client détermine finalités, catégories de données et durées de conservation « à sa seule discrétion ».
- DPA, « Données Personnelles des collaborateurs » **[cité]** : les données des contacts/utilisateurs du client sont traitées par H **en tant que responsable de traitement**, selon la politique de confidentialité.
- DPA, « Réutilisation des Données » **[cité]** : pour la réutilisation (dont entraînement), H agit **en qualité de responsable de traitement**, sur intérêt légitime, après pseudonymisation.
- Politique de confidentialité §1 **[cité]** : la politique couvre H comme responsable ; elle **exclut** le traitement des données personnelles contenues dans le contenu chargé par un utilisateur professionnel, renvoyé à l'addendum.
- Périmètre du DPA **[cité]** : « dans la mesure où le Client commande le Service en qualité de professionnel ».
- Restriction **[cité]** (DPA, « Assistance ») : le Client n'est pas autorisé à traiter via le Service des données de santé ; le traitement d'autres données « particulières » peut exiger des mesures renforcées à commander.

### 2.3 Entraînement sur les données clients

| Source | Ce qu'elle dit | Statut |
|---|---|---|
| CGV, « Droit de réutilisation des Données » | Licence gratuite, mondiale, sans limitation de durée pour la sécurité, l'amélioration du Service, la maintenance, le respect des CGU. **Puis**, pour l'entraînement des modèles IA et la R&D : permis « sauf en cas d'opposition expresse du Client » indiquée via le Service | **[cité]** |
| DPA, « Réutilisation des Données » | Même logique (renvoi à l'art. 12.3) ; pour les données personnelles, H = responsable, intérêt légitime, après pseudonymisation ; **c'est au Client d'informer les personnes concernées** | **[cité]** |
| Politique de confidentialité §2.2, §2.4 | Finalité « AI models training » ; données : Input, Output, Technical ; base : intérêt légitime, sauf opt-out du client. §3 : pour l'entraînement, données techniques conservées 6 mois pseudonymisées ; l'entraînement est dit « primarily » mené sur données synthétiques | **[cité]** |
| Docs Models API | Ne mentionne pas l'entraînement. Dit que les entrées/sorties ne sont jamais partagées avec des tiers (partage externe, pas usage interne par H) | **[cité]** ; **pas d'engagement de non-entraînement dans la doc** |
| Terms of Service (anglais, avril 2026) | Aucune occurrence de « train » dans le texte | **[cité par absence]** |
| Comment exercer l'opposition pour un client API | Les docs Models API et plateforme consultées ne décrivent aucun réglage d'opposition ; les contrats parlent d'une opposition « via le Service » ou « via son espace client » | **[inconnu]** |

Constat documentaire : sur les textes lus, il n'existe **pas d'engagement écrit de non-entraînement** par défaut chez H ; il existe un mécanisme d'opposition dont la mise en oeuvre pour la Models API n'est pas décrite. **[inférence]** Obtenir un engagement de non-entraînement passerait par une opposition effective et documentée, ou par un avenant négocié (le « Bon de Commande » figure dans la hiérarchie contractuelle, §2.7).

### 2.4 Rétention

| # | Source | Énoncé | Produit visé |
|---|---|---|---|
| R1 | Docs Models API § Data retention | Seuls horodatage, modèle et nombre de jetons sont journalisés ; « zero data retention by default » | Models API |
| R2 | Docs HoloDesktop CLI | Idem en mode hébergé : invites et captures traitées puis supprimées ; certifications SOC 2 et ISO 27001 affirmées (certificats non vérifiés) | Models API via le CLI |
| R3 | Terms of Service (anglais) §4.2.2, §6.2.1 | « No retention of user instructions (prompts) » ; seules des métadonnées techniques non identifiantes sont persistées ; traitement en mémoire | « Inference API » (nom du texte ; correspondance avec « Models API » **non écrite**) |
| R4 | Politique de confidentialité §3 | Entrées/sorties supprimées 30 jours après l'opération pour l'EEE ; 90 jours hors EEE ; captures supprimées immédiatement hors EEE ; données techniques 6 mois pseudonymisées (jusqu'à 1 an après fin de contrat pour la recherche interne) | Rédigé autour de l'**extension** (« The extension captures… ») ; se déclare applicable à tous les utilisateurs des Services |
| R5 | DPA, Sous-annexe A, « Durée(s) de conservation » | « Durée du Contrat », sauf suppression anticipée par le Client, sans préjudice de la conservation par H comme responsable de traitement. Catégories traitées : « Traces (métadonnées), Input, Output ». Fin de contrat : suppression, sous réserve des durées légales de preuve et prescription | Service (tous produits sous contrat) |
| R6 | Docs Agents API, changelog 1.0.7 et « Create a session » | Sessions et captures supprimées 30 jours après la fin **par défaut** (`delete_after_min`, `delete_screenshot_after_min`) ; `null` = conservation indéfinie | API Agents, **pas** la Models API |

Observations : R1 à R3 (zéro) et R5 (durée du contrat) sont difficiles à concilier telles quelles pour les mêmes données d'entrée/sortie ; R4 se présente comme celle de l'extension ; R6 montre qu'au moins un produit H (Agents) conserve les captures 30 jours par défaut. Le §3 de la politique ajoute, après le « 30 jours », que les entrées/sorties ne sont « in any case » pas conservées sous forme persistante : phrase à relire avec ce qui précède.

### 2.5 Hébergement, sous-traitants, transferts

**Régions**
- API Agents **[cité]** : H exploite des régions EU et US isolées ; EU par défaut (`agp.eu.hcompany.ai`), US `agp.hcompany.ai`.
- Models API (`https://api.hcompany.ai/v1/`) **[inconnu]** : les pages docs consultées (introduction, quickstart, référence) ne mentionnent **aucune région**. Pas d'analyse réseau/DNS (hors périmètre, non probante pour un contrat).
- DPA, AWS **[cité]** : « Union Européenne (Région Europe) pour l'infrastructure de production et le stockage de données » et, « de manière résiduelle et transitoire », États-Unis si des grappes GPU d'inférence spécifiques ne sont pas disponibles en UE.
- Politique de confidentialité §5-6 **[cité]** : « U.S.-based infrastructure » pour les transmissions (passage qui vise aussi les utilisateurs hors EEE).
- **[inférence]** L'inférence GPU de la Models API pourrait relever de l'exception américaine du DPA ; aucun texte ne le confirme ni ne l'infirme.

**Sous-traitants ultérieurs nommés dans la Sous-annexe A du DPA [cité]**

| Sous-traitant | Finalité (résumé) | Localisation déclarée | Transfert |
|---|---|---|---|
| Amazon Web Services, Inc. | Hébergement principal, bases de production, analytique, base d'entraînement | UE ; US résiduel/transitoire pour GPU d'inférence | DPF / SCC |
| Stripe, Inc. | Paiement, facturation | US et infra globale | DPF / SCC |
| PostHog, Inc. | Analyse comportementale, identifiants pseudonymes | UE (Allemagne) | DPF / SCC |
| Brevo (Sendinblue SAS) | E-mails transactionnels et marketing | UE | aucun |
| Datadog, Inc. | Supervision technique, hors invites utilisateurs | UE ; US temporaire pour historique, purgé sous 90 jours | DPF / SCC |
| Agilitation SAS (Axeptio) | Gestion des consentements | UE (Irlande via AWS) | aucun |
| HubSpot, Inc. | CRM | UE si option « EU Data Residency » activée / US | DPF / SCC |
| Y3A | Prestataire marketing et pricing | France | aucun |

Le DPA indique un DPO (contact dpo@hcompany.ai). **Aucun fournisseur de modèle tiers n'est listé**, ce qui est cohérent avec la doc (H fait tourner ses propres modèles) **[inférence]**. Mécanisme de changement : approbation préalable expresse du client pour les sous-traitants ; pour un nouveau, notification écrite, délai d'objection de 8 jours calendaires, sinon le client peut résilier avec un préavis d'un mois. La liste en ligne du Trust Center n'a pas pu être lue : la **version à jour est inconnue**.

**Transferts hors EEE [cité]** : H « privilégie » le traitement dans l'EEE ; en cas de transfert, décision d'adéquation ou SCC (module 3) avec le sous-traitant. Client hors EEE : SCC module 4, droit français. **Divergence à relire** : la politique de confidentialité indique, pour les utilisateurs hors EEE, que H n'a pas conclu de SCC spécifiques avec ses prestataires, alors que le DPA évoque DPF et SCC.

### 2.6 Autres engagements du DPA utiles à la décision

- Violation de données : alerte « dans les meilleurs délais » (pas de délai chiffré).
- Audit : une fois par an, préavis d'au moins 15 jours, aux frais du client.
- Indemnisation de H par le responsable de traitement en cas d'instruction non conforme (clause « Périmètre de responsabilité »).
- Sécurité : renvoi au Trust Center (non lu).

### 2.7 Quel texte prime, et pour quel produit

- CGV, « Documents contractuels » **[cité]** : ordre décroissant (i) CGV, (ii) CGU, (iii) annexe Sécurité, (iv) Politique de confidentialité et, le cas échéant, annexe de protection des données personnelles (DPA), (v) Bon de Commande. La politique et le DPA sont **au même rang** dans cet ordre ; le texte ne dit pas lequel l'emporte entre eux.
- Terms of Service anglais §10.2 **[cité]** : le DPA fait partie intégrante de ces Terms. Pour l'extension, les conditions HoloTab priment sur les termes généraux en cas de conflit.
- La politique de confidentialité s'applique, selon son propre texte, à tous les utilisateurs et services ; l'extension y est explicitement visée.
- **Non établi par les textes** : si un titulaire de clé API self-service de la Models API est lié par les **CGV françaises** (avec la licence d'entraînement avec opposition) ou par les **Terms of Service anglais** (silencieux sur l'entraînement, « no retention » des invites). Les deux textes se présentent comme applicables à l'API (CGV : « API » ; ToS : « Inference API »).

## 3. Alibaba Cloud (Model Studio, `qwen3.8-27b`)

### 3.1 Textes lus

| Texte | URL | Date affichée |
|---|---|---|
| Régions et endpoints | https://www.alibabacloud.com/help/en/model-studio/regions | 11 oct. 2026 |
| Politique de rétention des données (ZDR) | https://www.alibabacloud.com/help/en/model-studio/data-retention-policy | 11 oct. 2026 |
| Sécurité et confidentialité | https://www.alibabacloud.com/help/en/model-studio/privacy-notice | 11 oct. 2026 |
| Résumé des données d'entraînement | https://www.alibabacloud.com/help/en/model-studio/qwen-and-wan-training-data-disclosure | 11 oct. 2026 |
| Conditions produit du site international, §4.48 Model Studio | https://www.alibabacloud.com/help/en/legal/latest/alibaba-cloud-international-website-product-terms-of-service-v-3-8-0 | 23 sept. 2026 |
| Data Processing Addendum | https://www.alibabacloud.com/help/en/legal/latest/fe2cxg | 29 nov. 2023 |
| EEA Data Processing Addendum | https://www.alibabacloud.com/help/en/legal/latest/ae8upq | 2 août 2023 |
| Fiche modèle `qwen3.8-27b` | https://www.alibabacloud.com/help/en/model-studio/qwen3-8-27b | non affichée |
| Tarifs par région | https://www.alibabacloud.com/help/en/model-studio/model-pricing | 12 oct. 2026 |
| Accords liés | https://www.alibabacloud.com/help/en/model-studio/related-agreements | 11 oct. 2026 |

**Non lus [inconnu]** : Membership Agreement, Privacy Policy du site international, liste des sous-traitants (accessible une fois connecté), Trust Center, SLA d'inférence, page « Service-specific terms » (erreur 500 à la récupération). L'entité contractante et le droit applicable ne sont donc pas établis.

### 3.2 Rôle, entraînement, rétention

- **Rôle [cité]** : les deux DPA désignent Alibaba Cloud comme sous-traitant (« processor ») du client ; conditions produit §4.48.1(b) : le traitement du « Member Content » (dont Input et Output) se fait pour les besoins du client et en son nom. Les DPA s'appliquent « if required by Data Protection Legislation ». **Lequel des deux DPA (EEA 2023-08 ou général 2023-11) s'applique à un client européen n'est pas précisé** dans les pages lues ; tous deux datent de 2023.
- **Entraînement [cité]** : la page Security and privacy dit qu'Alibaba « will never use your data for model training » ; conditions produit §4.48.1(e) : pas d'utilisation du Member Content pour développer ou améliorer les modèles sans consentement séparé ; politique de rétention §5.2 : pas d'entraînement, de fine-tuning ni d'amélioration de modèle de fondation sans consentement séparé. C'est un **engagement écrit**, présent dans les conditions contractuelles et pas seulement dans la doc.
- **Rétention par défaut [cité]** : invites et réponses conservées « no longer than 30 days », sauf durée légale plus longue ou nécessité raisonnable de sécurité ; fonctions à état (conversations, fichiers) jusqu'à suppression par l'utilisateur. Contenu signalé par les filtres de sécurité automatiques : jusqu'à 2 ans.
- **Zero Data Retention (ZDR) [cité]** : réservé aux clients entreprise du site international, sur demande à un account manager après évaluation, activable par espace de travail, régions hors Chine continentale (dont Francfort). Cache chiffré de 24 h maximum ; fichiers à état supprimés sous 48 h s'ils ne le sont pas avant. Hors ZDR : métadonnées, agents/workflows, RAG, batch, Playground, API Responses à état.
- **`qwen3.8-27b` et ZDR [cité + inférence]** : la liste des séries couvertes par ZDR nomme qwen-max (à partir de `qwen3.8-max`), qwen-plus (à partir de `qwen3.7-plus`), qwen-flash (à partir de `qwen3.8-flash`), deepseek, glm et certains modèles image, audio, embedding. `qwen3.8-27b` **n'y figure pas**. Selon la politique (§4.2), un appel à un modèle non supporté depuis un espace ZDR suit la rétention standard sans être bloqué. **[inférence]** `qwen3.8-27b` relèverait de la rétention standard (30 jours) ; à confirmer par Alibaba.

### 3.3 Régions

- **[cité]** Régions Model Studio : Pékin, Singapour, Allemagne (Francfort), Japon (Tokyo), Hong Kong, US (Virginie). La région fixe le point d'accès et le lieu de stockage des données ; la portée de déploiement fixe le lieu d'exécution de l'inférence (Francfort : Global ou EU). Les données statiques restent dans la région choisie. À Francfort, seul le domaine dédié à l'espace de travail est pris en charge (pas le domaine DashScope).
- **`qwen3.8-27b` [cité]** : la fiche modèle et la page des tarifs (12 oct. 2026) ne listent ce modèle que pour **Pékin (Chine continentale)** et **Singapour (portée International)**. Les modèles `qwen3.8` listés à Francfort sont `flash`, `max`, `max-0902`, `omni-flash`, avec portée « Global » dans les tableaux ; la portée « EU » n'apparaît à Francfort que pour des modèles plus anciens (par exemple `qwen-plus`, `qwen3.5-flash`, `qwen3-vl-plus`).
- **Constat documentaire (sans portée juridique)** : sur les pages consultées, Francfort n'offre pas `qwen3.8-27b`. Une disponibilité future ou par un canal commercial non public reste **[inconnu]**.
- Les conditions produit §4.48.1(viii) **[cité]** prévoient que le Member Content est transféré, stocké et traité dans le pays où Alibaba Cloud, ses affiliés ou ses sous-traitants exploitent des installations, éventuellement hors de la juridiction du client. **À relire** en regard de la phrase de la page Régions sur le stockage dans la région choisie.

### 3.4 Sous-traitants et transferts

- **Sous-traitants** : autorisation générale ; liste tenue à jour, accessible une fois connecté au compte ; avis d'au moins 10 jours ouvrés avant changement ; droit d'objection pour motif de protection des données ; sinon résiliation du service concerné (DPA §4(d)). **Liste elle-même : [inconnu]** (derrière connexion).
- **Transferts [cité]** : DPA EEA §6 : mesures nécessaires (décision d'adéquation, BCR ou SCC avec mesures supplémentaires). DPA général §6 : le client décide du lieu de stockage (« Selected Region ») ; en cas de transfert restreint, les SCC sont réputées conclues entre le client (exportateur) et Alibaba Cloud (importateur), avec l'option 2 de la clause 9 pour les sous-traitants ultérieurs. Le client reste seul responsable de la conformité de ses propres instructions de transfert (§6.5).
- Plafond de responsabilité : renvoi à la clause 11.5 du Membership Agreement (non lu).

## 4. Hébergeur UE tiers : MyIA

- Le dépôt utilise MyIA via `https://api.medium.text-generation-webui.myia.io/v1` (exemples de configuration, `.env.example`).
- Site public https://myia.io (consulté 2026-10-12) **[cité]** : présente des services (Whisper, Stable Diffusion, Oobabooga text-generation « Micro/Mini/Medium/Large », Open-webui, Dify). Aucune mention de Holo, de Qwen3.8, d'un hébergeur, d'un lieu d'hébergement, d'un DPA ni d'une politique de confidentialité. Les chemins courants (`/privacy`, `/legal`, `/terms`, `/dpa`, `/cgu`, `/rgpd`) renvoient 404.
- **[inconnu]** : modèles réellement servis (au-delà de la configuration du dépôt), région ou pays d'hébergement, rôle RGPD, rétention, entraînement, sous-traitants, existence d'un DPA. Aucun appel authentifié n'a été fait (pas de clé, hors périmètre).
- **Disponibilité de Holo4 ou Qwen3.8 via un hébergeur UE tiers** : aucun autre hébergeur n'a été examiné ; **non vérifié**.

## 5. Contradiction sur la rétention et l'entraînement chez H : levée ?

**Non levée.** Ce que les textes permettent d'établir et ce qui reste ouvert :

| Question | Établi par les textes | Reste ouvert |
|---|---|---|
| Produit visé par « zéro rétention » | Docs et ToS : Models API / « Inference API » | Le ToS date d'avril 2026, le DPA et les CGV de juin 2026 ; l'ordre de précédence entre ToS anglais et CGV n'est pas écrit |
| Produit visé par 30 / 90 jours | Politique de confidentialité, rédigée autour de l'extension, mais déclarée applicable aux « Services » | Application à une clé Models API non précisée |
| Rétention en vertu du DPA | « Durée du Contrat » (annexe A), définie par le client | Interaction avec « zéro rétention » non expliquée |
| Entraînement | CGV et DPA : permis sauf opposition ; politique : intérêt légitime sauf opt-out ; docs et ToS anglais : silence | Procédure d'opposition pour un client API ; effet d'une opposition sur la rétention de 30 jours |
| Qui prime | CGV > CGU > annexe Sécurité > Politique et DPA (même rang) > Bon de commande (ordre énoncé dans les CGV) | Applicabilité des CGV à un compte self-service ; rang relatif DPA / politique |

**Hypothèse de réconciliation [inférence, non confirmée]** : « zéro rétention » décrirait le stockage persistant côté inférence (Models API) ; 30/90 jours décriraient les traces conservées côté extension ou plateforme ; le DPA couvrirait les données hébergées du client sous contrat. Aucun texte ne formule cette répartition. Seule une réponse écrite de H (DPO ou support) peut la lever.

## 6. Écart entre la politique #350 et les faits relevés

Mise en regard des trois garanties exigées par la politique décidée dans #350 (non-entraînement écrit, région UE documentée, DPA signé) et de l'état documentaire. Ce n'est pas un jugement de conformité.

| Exigence #350 | H Company (Models API) | Alibaba Cloud (`qwen3.8-27b`) | MyIA |
|---|---|---|---|
| Engagement écrit de non-entraînement | Absent par défaut ; entraînement permis sauf opposition (CGV, DPA, politique) | Présent (conditions produit §4.48.1(e), politique de rétention §5.2), sauf consentement séparé | [inconnu] |
| Région UE documentée | Models API : non documentée. API Agents : UE disponible. DPA : UE principal, US résiduel | Francfort existe mais **ne liste pas** ce modèle ; Singapour et Pékin seulement | [inconnu] |
| DPA | Publié (français), H sous-traitant, 8 sous-traitants nommés ; modalités de signature non précisées | Deux DPA publiés (2023), SCC réputées conclues ; liste des sous-traitants derrière connexion | [introuvable] |
| Rétention connue | Contradictoire (§2.4) | 30 jours par défaut ; ZDR sur demande, modèle non listé | [inconnu] |

Mécanisme d'acceptation du DPA : le DPA se présente comme une annexe des CGV applicable au client professionnel ; si une signature distincte est requise, **le texte ne le dit pas**.

## 7. Passages à relire par une personne compétente

1. **CGV, « Droit de réutilisation des Données »** et **DPA, « Réutilisation des Données »** : portée de la licence « mondiale, sans limitation de durée », statut de H comme responsable de traitement sur les données pseudonymisées, obligation du client d'informer les personnes concernées, forme valable de l'opposition.
2. **Concordance des énoncés de rétention** (R1 à R6, §2.4), notamment DPA Sous-annexe A (« Durée du Contrat ») face à « zero data retention ».
3. **Ordre de précédence** : CGV, « Documents contractuels » ; applicabilité aux clés API self-service ; relation avec le ToS anglais (§10.2 et clause HoloTab).
4. **Hébergement** : exception AWS « résiduelle et transitoire » aux États-Unis pour les grappes GPU d'inférence ; absence de région pour la Models API.
5. **Transferts** : divergence entre la politique de confidentialité (pas de SCC spécifiques hors EEE) et le DPA (DPF / SCC).
6. **Allocation de responsabilité** : DPA H (indemnisation du sous-traitant, assistance facturée, audit aux frais du client) et plafond d'Alibaba (clause 11.5 du Membership Agreement, non lu).
7. **Alibaba** : tension entre données statiques « dans la région choisie » et §4.48.1(viii) des conditions produit ; effet de la portée « Global » sur le lieu d'exécution ; version du DPA applicable (2023-08 ou 2023-11) ; entité contractante.
8. **Statut du modèle `qwen3.8-27b`** : absence à Francfort et de la liste ZDR.

## 8. Questions écrites à poser avant tout engagement (propositions)

À H Company (DPO, voir le DPA) :
- Une clé API self-service est-elle soumise aux CGV ou au ToS anglais ?
- Que couvre « zero data retention » face à la durée du contrat de la Sous-annexe A ?
- Où est exécutée l'inférence de la Models API (région, pays) ?
- Comment exercer l'opposition à l'entraînement pour un compte API, et un engagement écrit de non-entraînement peut-il être consenti par avenant ?
- Quelle est la liste à jour des sous-traitants (Trust Center) ?

À Alibaba Cloud (compte entreprise) :
- `qwen3.8-27b` sera-t-il proposé à Francfort avec portée EU ? ZDR s'applique-t-il à ce modèle ?
- Liste à jour des sous-traitants ; DPA applicable ; entité contractante.

À MyIA : région d'hébergement, DPA, politique de rétention et d'entraînement, catalogue de modèles.

## 9. Limites de cette recherche

- Textes anglais et français lus tels que publiés le 2026-10-12 ; aucune version archivée ni historique de modifications consulté (les dates affichées par H sont parfois contradictoires).
- Les extraits sont courts ; l'URL fait foi. Aucune page authentifiée n'a été utilisée.
- Les pages Alibaba proviennent du site international en anglais ; le Membership Agreement et la Privacy Policy du site n'ont pas été lus.
- Aucune demande n'a été adressée aux fournisseurs ; aucun appel d'API n'a été fait.
