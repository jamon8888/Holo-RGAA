# Ara (DINUM) face à Holo-RGAA : sommes-nous sur la bonne voie ?

> Analyse du code de [DISIC/Ara](https://github.com/DISIC/Ara) au commit `b3ece0a` (2026-11-12), licence MIT, 63 ★, dépôt actif (dernier push 2026-11-20). Vue 3 + NestJS + Prisma/PostgreSQL. Comparaison avec ce dépôt au 2026-11-21.

## 1. Ce qu'est Ara

Ara est l'outil gratuit de l'État pour **mener à la main** un audit RGAA, publier le rapport et générer la déclaration d'accessibilité. Il ne contient **aucune automatisation** : pas de navigateur piloté, pas d'axe, pas d'IA. L'auditeur coche C, NC, NA ou NT pour chaque critère sur chaque page, et Ara calcule et met en forme.

| Concept Ara (code) | Fichier | Équivalent Holo-RGAA |
|--------------------|---------|----------------------|
| `AuditType` : `FAST` (25 critères), `COMPLEMENTARY` (50), `FULL` (106) — listes officielles « audit rapide » et « audit complémentaire » de design.numerique.gouv.fr | `confiture-rest-api/src/audits/criteria.ts` | **absent** : nous n'auditons que les 106 critères |
| Page d'**éléments transverses** (en-tête, pied, menu évalués une fois) | `schema.prisma` (`transverseElementsPage`) | **absent** |
| `CriterionResult` par page × critère, statut `COMPLIANT`/`NOT_COMPLIANT`/`NOT_APPLICABLE`/`NOT_TESTED` | `schema.prisma` | `CriterionResult` + `CriterionStatus` (avec en plus `NeedsReview` et `Error`) |
| `NotCompliantItem` : plusieurs erreurs par critère, `userImpact` mineur/majeur/bloquant, `quickWin`, images d'exemple | `schema.prisma` | findings et *Preuves* (P-00) ; **pas d'impact utilisateur ni de quick win** |
| `TestEnvironment[]` : plateforme, OS, technologie d'assistance, navigateur | `schema.prisma` | une chaîne unique `environnement` dans la déclaration |
| Déclaration : dérogations, contenus non soumis, schéma pluriannuel, plan d'action, contact | `statements.controller.ts` | `rgaa-report/src/declaration.rs`, avec les mêmes rubriques |
| Taux = critères conformes / critères applicables ; critère NC s'il est NC sur une page au moins ; C s'il est C ou NA partout avec au moins un C | `DOCUMENTATION.md` | `rgaa-report/src/lib.rs` : même règle, plus le garde-fou « taux non opposable si l'audit est incomplet » |
| Méthodologies de test officielles affichées à l'auditeur | `confiture-web-app/scripts/build-rgaa-files.js` (génère `criteres.json` et `methodologies.json` depuis le dépôt officiel) | **nous n'embarquons pas `methodologies.json`** |
| Liens secrets d'édition (`editUniqueId`) et de consultation (`consultUniqueId`) | `schema.prisma` | page de revue avec comptes nominatifs (F-ACC-7) |
| Export CSV de la grille critères × pages | `audit-export.service.ts` | rapport HTML/PDF, déclaration, dépôt pilote ; **pas de grille CSV** |

Ara annonce dans sa feuille de route (`ROADMAP.md`) :

- en cours : plusieurs erreurs et recommandations par critère, et un « rapport centré sur les erreurs » ;
- à venir : **RGAA 5**, export de la grille et du rapport, et un **« audit par non-conformités »** où l'on relève les défauts au fil de l'analyse, sans suivre la liste des critères ;
- plus tard : collaboration entre plusieurs auditeurs, et export vers les outils de ticketing.

## 2. Ce qui valide notre direction

1. **Même source de vérité.** Notre `criteres.json` a **la même empreinte** que `RGAA/criteres.json` du dépôt officiel `DISIC/accessibilite.numerique.gouv.fr` (SHA-256 canonique `d64d84c0cb87`). L'application web d'Ara génère le sien depuis le même dépôt. Remarque : la copie embarquée dans l'API d'Ara (`confiture-rest-api/src/rgaa.json`, figée en 2022) diffère du texte officiel sur 38 tests et 3 intitulés. Elle ne sert qu'à valider les numéros, et ce n'est pas une référence.
2. **Même règle de calcul du taux et du statut** d'un critère sur l'échantillon. Notre garde-fou sur le taux, non opposable tant que l'audit est incomplet ou en revue, va plus loin qu'Ara.
3. **La feuille de route d'Ara va vers notre modèle.** « Audit par non-conformités », « rapport centré sur les erreurs », « plusieurs erreurs par critère » : c'est exactement un modèle fondé sur des *Preuves* rattachées à des éléments, que nous avons déjà.
4. **Le créneau est libre.** Ara ne fait aucune pré-analyse. Sur un **audit rapide** (25 critères), notre moteur couvre 15 critères en automatique (11 déterministes, 4 axe) et 10 en assisté avec preuve : 3.1, 5.3, 6.1, 7.1, 8.4, 9.1, 10.3, 11.2, 11.9, 11.10. **Ara n'est pas un concurrent, c'est la destination naturelle de nos résultats** pour les auditeurs du secteur public.
5. **Les rubriques de la déclaration correspondent une à une** (dérogations, non soumis, schéma pluriannuel, plan d'action, contact).

## 3. Ce qui nous manque (écarts à combler)

| # | Écart | Gravité | Pourquoi |
|---|-------|---------|----------|
| E1 | Types d'audit **rapide (25) / complémentaire (50) / complet (106)** | haute | C'est la pratique de la DINUM et ce que demandent les petits commanditaires. C'est aussi notre meilleur argument de coût : 25 critères, dont 15 automatiques |
| E2 | **Éléments transverses** évalués une fois | haute | Sans eux, un défaut d'en-tête ou de pied est compté sur chaque page et chaque revue se répète, alors qu'un auditeur le traite une fois |
| E3 | **Plusieurs environnements de test** (OS, technologie d'assistance, navigateur) | moyenne | La déclaration RGAA les liste. Nous écrivons une seule chaîne, et nous ne testons aujourd'hui avec **aucune** technologie d'assistance |
| E4 | **Impact utilisateur** (mineur / majeur / bloquant) et **quick win** sur chaque non-conformité | moyenne | C'est ce que l'auditeur livre au client pour prioriser. Notre remédiation n'a pas ce vocabulaire |
| E5 | **Méthodologies officielles** de chaque test (`methodologies.json`) | haute | Ara montre la méthode officielle. Nos skills ont **inventé** des étapes et des correspondances (catalogue faux, corrigé le 2026-11-21, commit `e51a5d5`). Le guidage pas à pas doit citer la méthode officielle |
| E6 | **Interopérabilité avec Ara** : exporter nos résultats vers un audit Ara | haute | L'API d'Ara permet de créer un audit (`POST /audits`) et d'écrire les résultats (`PATCH /audits/:editUniqueId/results`, éléments `{pageId, topic, criterium, status, compliantComment, notApplicableComment, notCompliantItems[]}`). Mais c'est une **API interne, sans contrat public** |
| E7 | **Grille CSV** critères × pages (et transverses) | basse | Même format que l'export d'Ara, pour que l'auditeur retrouve ses repères |
| E8 | **RGAA 5** | moyenne (à surveiller) | Ara l'anticipe. Notre plan de moteurs (`engine_plan.json`), nos règles axe et nos plugins sont indexés sur la 4.1.2. Garder les données versionnées par dossier (c'est déjà le cas) et ne jamais coder un numéro de critère en dur hors de ces données |

## 4. Ce qui diverge et doit être tranché

1. **`Error` compte comme une non-conformité.** `rgaa-report/src/lib.rs` (`reduire_statuts`) rend `NonConforme` si une page est en `Error`. Une panne de notre navigateur produit donc une non-conformité dans le rapport. Ara n'a pas ce statut : un critère non évalué reste `NOT_TESTED`. **Recommandation : `Error` vaut `NonTeste`**, ce qui est cohérent avec la règle maison « jamais de verdict fabriqué ».
2. **Correspondance axe → RGAA incohérente pour `button-name`.** `engine_plan.json` et l'étiquette axe-core `RGAA-11.9.1` rattachent la règle à **11.9**. Mais `rgaa-rules/src/axe_mapper.rs` (`button-name` → 11.1) et `mechanisms.toml` (mécanisme de 11.1) la rattachent aussi à **11.1**. Le linter est corrigé (11.9). Le moteur d'exécution reste à aligner.

## 5. Réponse à la question

**Oui, sur le fond.** Même source officielle, même règle de calcul, une feuille de route d'Ara qui converge vers notre modèle de preuves, et un créneau, la pré-analyse, qu'Ara ne couvre pas et ne compte pas couvrir.

**Mais il faut se repositionner.** Il ne faut pas refaire Ara : l'outil est gratuit, porté par l'État et connu des auditeurs publics. Il faut **l'alimenter**. Holo-RGAA pré-analyse et prépare les preuves. L'auditeur revoit dans Claude Desktop ou dans la page de revue, puis **publie dans Ara**, ou dans notre rapport pour le secteur privé (EAA). Les écarts E1, E2, E5 et E6 sont les conditions de crédibilité devant un auditeur.

Pour E6, deux voies :

- **(a) court terme** : un export vers l'API d'Ara, testé sur une instance **auto-hébergée** (MIT, `docker`/Scalingo). On ne l'utilise jamais contre l'instance publique sans accord.
- **(b) durable** : **proposer en amont une fonction d'import JSON** (contribution par pull request, prévue par `CONTRIBUTING.md`) et en discuter avec l'équipe d'Ara à la DINUM.
