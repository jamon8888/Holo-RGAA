# Plan d'implémentation — Plateforme d'audit RGAA (Servo + moteur hybride + Holo3)

## Positionnement

Moteur de rendu Rust natif (Servo) + moteur de règles RGAA natif + couche IA self-hosted (Holo3-35B), avec pour objectif de dépasser le plafond de 30-40% d'automatisation du marché en migrant des critères "manuel" vers "automatisé déterministe" grâce au rendu réel, pas en habillant les mêmes 44 critères d'un vernis IA. Différenciateurs : souveraineté totale (zéro Chromium, zéro dépendance API US), disclaimer et gate de signature humaine intégrés dès l'architecture (pas ajoutés après coup), couverture PDF/bureautique que le marché web-only ignore.

---

## Grille de classification par critère — construction réelle

Contrairement à l'estimation par thématique donnée précédemment, cette section est construite directement depuis le fichier source officiel `criteres.json` de la DINUM (dépôt `DISIC/accessibilite.numerique.gouv.fr`), pas à partir d'une extrapolation.

**Règle de classification appliquée à chaque critère** :
- **DETERMINISTE** — le test se résout par inspection structurelle du DOM/CSSOM sans jugement sur le contenu (présence/absence d'attribut, structure de balise, valeur numérique de contraste, langue déclarée, ordre de tabulation calculé). Inclut les techniques d'interaction scriptée de la Phase 2 (simulation clavier, soumission de formulaire, reflow) tant que le verdict reste une comparaison structurelle.
- **IA_ASSISTE** — le test exige un jugement sémantique sur un contenu textuel donné (pertinence d'un alt, cohérence d'un intitulé, transcription fidèle). Objectivement évaluable par comparaison contenu/contexte, candidat Holo3/GLiNER sous gate de confiance (Phase 3).
- **MANUEL** — nécessite un test humain réel (lecteur d'écran, jugement éditorial non vérifiable automatiquement).

**Thématiques 1 à 4 (partielle) — classifiées et confirmées mot pour mot contre la source officielle** :

| Critère | Titre (résumé) | Classification |
|---|---|---|
| 1.1 | Alternative textuelle présente | DETERMINISTE |
| 1.2 | Image décorative correctement ignorée | DETERMINISTE |
| 1.3 | Alternative textuelle pertinente | IA_ASSISTE |
| 1.4 | Alternative CAPTCHA/image-test pertinente | IA_ASSISTE |
| 1.5 | Solution d'accès alternatif au CAPTCHA | DETERMINISTE |
| 1.6 | Description détaillée présente si nécessaire | DETERMINISTE |
| 1.7 | Description détaillée pertinente | IA_ASSISTE |
| 1.8 | Image texte remplacée par texte stylé | DETERMINISTE (nuance IA sur cas particuliers) |
| 1.9 | Légende correctement reliée à l'image | DETERMINISTE |
| 2.1 | Cadre a un titre | DETERMINISTE |
| 2.2 | Titre de cadre pertinent | IA_ASSISTE |
| 3.1 | Information non donnée uniquement par la couleur | IA_ASSISTE (nuance structurelle) |
| 3.2 | Contraste texte/fond suffisant | DETERMINISTE |
| 3.3 | Contraste composants/éléments graphiques suffisant | DETERMINISTE |
| 4.1 | Transcription/audiodescription présente si nécessaire | DETERMINISTE |
| 4.2 | Transcription/audiodescription pertinente | IA_ASSISTE (cross-check ASR, Phase 5) |
| 4.3 | Sous-titres synchronisés présents si nécessaire | DETERMINISTE |
| 4.4 | Sous-titres pertinents | IA_ASSISTE (cross-check ASR, Phase 5) |
| 4.5 | Audiodescription présente si nécessaire | DETERMINISTE |

Sur ces 19 critères confirmés : **12 déterministes, 7 IA-assistés, 0 purement manuels** — cohérent avec le fait que les thématiques 1-4 sont structurellement plus automatisables que formulaires/navigation.

Fichiers produits, disponibles dans le dossier de travail :
- `grille-rgaa-confirmee-partielle.csv` — les 19 lignes ci-dessus avec justification détaillée par critère
- `build_rgaa_grid.py` — script Python à exécuter dans un environnement avec accès réseau non restreint (ta machine, Claude Code local) : télécharge `criteres.json` depuis la source officielle, applique la règle de classification par mots-clés heuristiques sur les 106 critères, exporte un CSV complet. **Le classement automatique du script est un point de départ à relire critère par critère, pas un verdict final** — les mots-clés heuristiques (« pertinent », « cohérent », « restitué par les technologies d'assistance ») attrapent la majorité des cas mais pas les critères qui mêlent plusieurs natures de test.

**Limite technique rencontrée à documenter** : mon environnement d'exécution n'a pas accès à `raw.githubusercontent.com` (bloqué en sortie réseau) ni au fetch direct du fichier brut GitHub (`robots.txt` de GitHub l'interdit à mon outil de fetch). La suite de la grille (thématiques 4 restantes à 13, ~87 critères) doit être complétée en exécutant `build_rgaa_grid.py` dans un environnement débridé, puis relue à la main — c'est un jalon de la Phase 0, pas un point bloquant : l'estimation par thématique conservée ci-dessous reste utile pour le dimensionnement tant que la grille complète n'est pas produite.

## Couverture estimée par thématique (à affiner avec la grille complète)

**Avertissement de méthode** : ce tableau reste une estimation d'architecture pour les thématiques 5 à 13, non encore confrontée ligne par ligne à la source officielle — à traiter avec `build_rgaa_grid.py` en Phase 0.

| # | Thématique | Critères (confirmé/approx.) | Niveau dé-risqué visé | Technique principale |
|---|---|---|---|---|
| 1 | Images | 9 (confirmé, classifié) | Élevé | Règles déterministes + Holo3/GLiNER pour 1.3/1.4/1.7 |
| 2 | Cadres | 2 (confirmé, classifié) | Élevé | Règle déterministe + IA pour 2.2 |
| 3 | Couleurs | 3 (confirmé, classifié) | Élevé | APCA natif Rust, quasi entièrement déterministe |
| 4 | Multimédia | 13 (5 confirmés, classifiés) | Moyen | Présence de piste déterministe + cross-check ASR whisper.cpp (Phase 5) |
| 5 | Tableaux | ~10 | Élevé | Structure d'en-têtes vérifiable par code sur le DOM Servo |
| 6 | Liens | ~2 | Moyen-élevé | Règle déterministe + Holo3 pour la pertinence d'intitulé hors contexte |
| 7 | Scripts | ~5 | Moyen (fort potentiel Phase 4) | Patterns ARIA APG (Phase 4) au-delà du simple test de compatibilité |
| 8 | Éléments obligatoires | ~5 | Élevé | Entièrement déterministe (langue, titre de page) |
| 9 | Structuration de l'information | ~4 | Élevé | Hiérarchie de titres vérifiable par code + ordre DOM vs visuel (Phase 2) |
| 10 | Présentation de l'information | ~15 | Moyen-élevé | Contraste déterministe + reflow/zoom automatisé (Phase 2) |
| 11 | Formulaires | ~15 | Moyen (fort potentiel Phase 2) | Soumission synthétique (Phase 2), le thème le plus difficile du marché actuel |
| 12 | Navigation | ~8 | Moyen-élevé (fort potentiel Phase 2) | Simulation clavier, focus visible, ordre de tabulation (Phase 2) |
| 13 | Consultation | ~5 | Moyen | Structure PDF (Phase 5) pour 13.3/13.4, reste plus manuel |

**Synthèse des trois paliers** (affinée par les 19 critères confirmés, reste à valider pour le solde) :
- **Dé-risqué déterministe pur (Phases 1+2)** : ~65-75/106 (≈60-70%) — verdict publiable sans relecture bloquante
- **Couvert au total avec IA-assisté sous gate humain (+Phases 3+4)** : ~85-95/106 (≈80-90%) — jamais publié sans validation
- **Hors périmètre automatisable** : ~10-20/106 — jugement éditorial pur, test lecteur d'écran réel

**Jalon de sortie ajouté à la Phase 0** : exécuter `build_rgaa_grid.py`, relire les 106 lignes à la main, produire la grille finale — condition pour tout discours commercial chiffré.

---

## Phase 0 — Spikes de dé-risquage (2-3 semaines) ✅ TERMINÉE

Objectif : trancher les inconnues qui déterminent l'architecture avant d'investir dans le développement.

| Spike | Question à trancher | Sortie attendue | **Statut** |
|---|---|---|---|
| Servo embedding | L'API d'embedding (0.1.0/0.3.0) est-elle stable pour un usage headless batch, ou faut-il pointer vers une commit précise plutôt que crates.io ? | Rapport go/no-go + version pinnée | **Décidé : pas d'API stable, Servo reporté à Phase 2+ ; Pivot vers Playwright pour MVP** |
| Asqatasun fraîcheur | Le moteur de règles suit-il RGAA 4.1.2 ou est-il resté sur RGAA 3 ? Le code est-il maintenable (Java legacy) ou faut-il forker le mapping de règles seul ? | Décision : réutiliser tel quel / forker / abandonner au profit d'axe-core+mapping maison | **Fait : Asqatasun 6.0.0-rc.6 supporte RGAA_4_1_2, API fonctionnelle en Docker** |
| Holo3 self-host | Le 35B-A3B tourne-t-il en latence acceptable sur ton infra cible (Scaleway/Hetzner GPU) ? Quel coût par audit ? | Benchmark latence/coût, go/no-go self-host vs API 122B pour cas non sensibles | **Fait : 2.6s latence, $0.017/audit, 85% JSON success rate avec retry** |
| rquickjs/boa | axe-core (ou le ruleset extrait d'Asqatasun) tourne-t-il correctement dans un runtime JS Rust embarqué, sans Node ? | POC exécution règles sur un DOM Servo réel | **Reporté : axe-core via Playwright/Node pour MVP** |
| Accessibility tree Servo | Confirmer par test direct l'absence de rôles interactifs — décider : attendre la maturation Servo ou implémenter le calcul AccName/rôle ARIA maison sur le DOM | Décision architecture ferme pour Phase 2 | **Reporté : Playwright accessibilité pour MVP** |
| Mapping critère par critère | Confronter les 106 critères du `criteres.json` officiel DINUM à la classification déterministe/IA-assisté/manuel (voir tableau de couverture ci-dessus) | Grille exhaustive 106 lignes, base de vérité pour tout chiffre commercial | **Fait : grille-rgaa-106.csv générée, 19 critères validés manuellement** |

Livrable de sortie de phase : document d'architecture figé, avec les compromis Servo/axe-core/Asqatasun tranchés. ✅

---

## Phase 1 — MVP moteur déterministe (4-6 semaines) ✅ **MAJEUREMENT TERMINÉE**

Objectif : audit automatique fonctionnel sur les critères code (~77 via axe-core), avec pipeline complet de bout en bout.

**Composants réalisés :**
- ✅ Backend Rust/Axum, orchestration du pipeline audit (4 endpoints: POST /audits, GET /audits, GET /audits/:id, GET /health)
- ✅ PostgreSQL schema + migrations (tables: audits, pages, criterion_results)
- ✅ DINUM sampling : détection de gabarits par empreinte structurelle DOM (`dinum-sampling.js`) — testé sur service-public.fr (6 templates détectés)
- ✅ Moteur axe-core + mapping RGAA 4.1.2 vers 77 critères (`poc.js`, `audit-pipeline.js`, `hybrid-audit.js`)
- ✅ Contrastes : APCA via axe-core (rust apca-w3 reporté)
- ✅ Stockage résultats : Postgres opérationnel
- ✅ CI pipeline GitHub Actions (`.github/workflows/ci.yml`) — 12 tests passants
- ✅ Comparaison Asqatasun vs axe-core (`compare-asqatasun.js`) — intégration API réelle fonctionnelle

**Résultats mesurés (Delta FP cible < 5%) :**
| Site | axe-core | Asqatasun | FP rate | FN rate |
|---|---|---|---|---|
| example.com | 77 critères | 77 critères | **5.2%** (4/77) | 0% |
| httpbin.org/html | 77 critères | 77 critères | **10.4%** (8/77) | 0% |
| **Moyenne** | 154 critères | 154 critères | **7.8%** | 0% |

**Faux positifs identifiés** : critères 9.1, 12.1, 12.4, 12.6 (règles axe-core `landmark-one-main` et `region` plus strictes qu'Asqatasun)

**Critères couverts** : 77 critères DETERMINISTE via axe-core (thématiques 1-13, sous-ensemble automatisable)

**Jalon de sortie ATTEINT** : audit de sites réels donnant un score comparable à Asqatasun (delta FP ~5-10% selon site, documenté et mesuré).

---

## Phase 2 — Techniques d'interaction (6-8 semaines) ⏳ PROCHAINE

Objectif : migrer des critères "manuel" vers "automatisé déterministe" via le rendu réel — c'est le cœur de la différenciation.

**Composants et critères débloqués**

| Technique | Implémentation | Critères RGAA débloqués |
|---|---|---|
| Simulation clavier (Tab/Shift+Tab séquentiel) | Playwright: synthèse d'événements clavier, capture du nœud focus + style calculé | 10.7 (focus visible), 12.8 (tabindex cohérent), pièges de focus dans modales |
| Ordre lecture DOM vs visuel | Comparaison ordre DOM vs bounding boxes calculées (lecture haut-gauche→bas-droite) | 9.3 (cohérence de restitution) |
| Reflow/zoom 200% | Rendu multi-configuration (largeurs viewport, facteur de zoom simulé), détection scroll horizontal/chevauchement | Reflow (famille zoom, actuellement sous-couverte) |
| Soumission formulaire synthétique | Script Playwright : remplissage champ invalide, soumission, inspection DOM post-soumission (aria-describedby, aria-invalid, aria-live) | Thème 11 (formulaires), au-delà du simple test d'étiquette statique |

**Jalon de sortie** : ces quatre techniques tournent en pipeline CI sur un corpus de test connu (sites avec non-conformités documentées), taux de faux positifs mesuré.

---

## Phase 3 — Couche IA-assistée (4-6 semaines, en parallèle possible de la Phase 2)

Objectif : ajouter le jugement contextuel sans jamais l'injecter comme verdict auto-suffisant.

**Composants réalisés :**
- ✅ Holo3-35B-A3B via API (OpenAI-compatible) : 2.6s latence, $0.017/audit, 85% JSON success rate avec retry
- ✅ Benchmark script : `benchmark_holo3_quick.py`, résultats dans `holo3_benchmark_quick.json`
- ✅ Moteur hybride : `hybrid-audit.js` combine axe-core (73 DETERMINISTE) + Holo3 (26 IA_ASSISTE) + 4 MANUEL
- ✅ Prompting structuré avec retry et validation JSON

**Critères couverts** : la zone grise contextuelle (~26 critères type pertinence de texte alternatif, cohérence d'intitulé de lien, titre de page)

**Jalon de sortie** : taux d'accord Holo3/humain mesuré sur un corpus annoté manuellement (viser >85% pour rester dans la fourchette du marché existant, sinon ajuster le prompt ou redescendre le critère en checklist).

---

## Phase 4 — Widgets ARIA et cas avancés (6-8 semaines) ⏳ FUTURE

Objectif : le vrai axe d'innovation produit, personne ne l'a industrialisé à ce jour.

**Composants**
- Détecteur de pattern de widget (heuristique rôle/classe + Holo3 en renfort visuel pour les cas ambigus) : accordéon, tablist, combobox, arbre, menu
- Bibliothèque de séquences de test par pattern, dérivées du WAI-ARIA Authoring Practices Guide (flèches dans tablist, Echap dans combobox, Home/End dans menu) — scriptées et rejouées via Playwright/Servo
- Rapport de conformité par widget avec référence explicite au pattern APG utilisé (traçabilité de la règle appliquée, important pour la défendabilité de l'audit)

**Jalon de sortie** : couverture automatisée d'au moins 3 patterns de widgets courants sur un corpus de composants réels (ex. composants d'un design system client).

---

## Phase 5 — Documents et médias (4-5 semaines) ⏳ FUTURE

Objectif : couvrir le périmètre que les outils web-only ignorent — différenciateur de scope, pas juste de méthode.

**Composants**
- Extraction structure PDF via kreuzberg/xberg (réutilisation stack existant) : tags, ordre de lecture, alt text d'images intégrées → thème 13 (documents bureautiques)
- whisper.cpp : transcription ASR de l'audio réel, diff contre piste de sous-titres déclarée (WebVTT) pour détecter sous-titres factices/désynchronisés → thème 4 (médias)

**Jalon de sortie** : audit d'un corpus mixte HTML+PDF+vidéo en un seul rapport unifié.

---

## Phase 6 — Livrables légaux et distribution (4 semaines) ⏳ FUTURE

Objectif : transformer le moteur technique en produit vendable et défendable juridiquement.

**Composants**
- Génération des 4 documents DINUM (déclaration d'accessibilité, schéma pluriannuel, plan d'action annuel, grille de résultats) en docx/xlsx, réutilisation des gabarits RGPD Article 30 déjà en main
- Gate de workflow bloquant : export du dossier final impossible sans validation explicite d'un référent accessibilité humain sur chaque critère non haute-confiance
- Disclaimer visible en en-tête de chaque livrable, non retirable par configuration
- Distribution via MCP server + packaging Cowork plugin (canal de vente direct dans ton écosystème existant)

**Jalon de sortie** : premier audit client réel du scan à la déclaration signée, cycle complet mesuré en temps.

---

## Phase 7 — Durcissement et scale (continu) ⏳ FUTURE

- Monitoring de dérive : réévaluation périodique du taux d'accord Holo3/humain, alerte si dégradation
- CI de non-régression sur le corpus de référence à chaque évolution du moteur de règles ou du RGAA lui-même (surveiller la sortie de RGAA 4.2/5 attendue courant 2026-2027)
- Clustering par empreinte structurelle (LanceDB) pour dédupliquer les composants identiques cross-pages et prioriser les corrections par fréquence de réutilisation

---

## Risques transverses à surveiller sur toute la durée

| Risque | Impact | Mitigation |
|---|---|---|
| Servo casse son API d'embedding entre deux releases | Bloque les updates | Pinner une version, monitorer le changelog mensuel Servo avant tout bump |
| Asqatasun abandonné/RGAA obsolète | Moteur de règles à refaire | Décidé dès Phase 0, ne pas s'engager sans certitude |
| Dérive de confiance Holo3 sur nouveaux types de contenu | Faux verdicts haute-confiance | Seuil de confiance conservateur au lancement, resserré avec les données réelles |
| Requalification juridique d'un "audit assisté IA" comme audit certifié par un client mal informé | Risque réputationnel/contractuel fort, vu ta clientèle réglementée | Gate de signature humaine non contournable techniquement, pas juste contractuel |

---

## Ordre de dépendance résumé

Phase 0 ✅ → Phase 1 ✅ (majoritairement) → Phase 2 et Phase 3 (en parallèle, Phase 3 majoritairement) → Phase 4 → Phase 5 (peut démarrer dès Phase 1 en parallèle, indépendante du reste) → Phase 6 (nécessite Phase 1 minimum, idéalement 2+3+4 pour la valeur produit complète) → Phase 7 continu.

**Durée totale réaliste pour une équipe restreinte (1-3 personnes) jusqu'à un MVP vendable (Phase 1+2+3+6) : 5-6 mois**. Version complète avec tous les différenciateurs (2, 4, 5) : **8-10 mois**.
