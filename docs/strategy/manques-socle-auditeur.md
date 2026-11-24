# Ce qui manque au socle auditeur RGAA/EAA

> 2026-11-21 · Suite de l'analyse contradictoire du socle (Claude Desktop + computer use + Holo-RGAA) et de l'[analyse d'Ara](analyse-ara.md). Décisions de référence : PRD §0, D5 à D9.
> Carte wayfinder : [Socle auditeur RGAA/EAA démontrable en cabinet](https://github.com/jamon8888/Holo-RGAA/issues/441) — tickets de décision #442 à #449, chantiers d'exécution #450 à #452.

## 1. Méthode

On suit le travail réel d'un cabinet, étape par étape :

1. la commande ;
2. l'échantillon ;
3. la collecte ;
4. la revue ;
5. le livrable ;
6. le suivi.

Pour chaque étape, on cherche dans le code ce qui existe. On ne retient comme manque que ce qui **n'est pas déjà ticketé**. Ce qui est déjà ticketé est rappelé en fin de document.

## 2. Ce qui existe déjà (constaté)

| Étape | Existant | Où |
|-------|----------|----|
| Échantillon | découverte des pages obligatoires : accueil, contact, mentions légales, accessibilité, aide, plan du site, authentification | `rgaa-orchestrator/src/pipeline.rs` (`discover_rgaa_sample_pages`) |
| Échantillon | garde-fou « échantillon insuffisant » | `rgaa-report/src/guard.rs` |
| Collecte | zoom 200 %, reliure à 320 px, espacement du texte, orientation, navigation multi-pages (12.1, 12.2, 12.5 à 12.7) en déterministe | `engine_plan.json` |
| Collecte | parcours simples avant scan (clic, saisie, attente), cookies | `PreScanAction`, `Cookie` (MCP `analyze`) |
| Livrable | déclaration FR, 12 packs pays, PDF natif | `rgaa-report` (`declaration.rs`, `packs.rs`, `pdf_native.rs`) |
| Suivi | revérification de fichiers corrigés face à un audit de référence | outil MCP `verify_fix` |
| Distribution | binaires Linux, macOS (x86, ARM) et Windows | `.github/workflows` (cibles de release) |

## 3. Manques

| # | Manque | Étape | Gravité | Pourquoi c'est bloquant pour un cabinet | Traitement |
|---|--------|-------|---------|------------------------------------------|------------|
| M1 | **Correspondance EN 301 549 et livrable EAA.** Seules les étiquettes `EN-301-549` d'axe-core figurent dans `axe_rules.json` (69 occurrences, partielles) ; aucune table clause → critère RGAA, aucun livrable EAA. *Corrigé le 2026-11-21 : la première version disait « aucune occurrence », la recherche #442 a montré le contraire* | livrable | haute | La loi EAA renvoie à la norme harmonisée EN 301 549, pas au RGAA. Un cabinet qui audite un acteur privé doit citer les clauses (chapitre 9 pour le web, 10 pour les documents, 11 pour les logiciels, 12 pour la documentation et le support). Sans cette table, « auditeur EAA » est un argument vide | recherche |
| M2 | **Accessibilité des documents PDF.** On détecte les liens vers des PDF (13.3), on ne contrôle pas le PDF lui-même (balisage, ordre de lecture, alternatives, langue, PDF/UA) | collecte | haute | Les sites publics et les services EAA publient leurs contrats, factures et formulaires en PDF. C'est une part importante des non-conformités constatées en audit | recherche, puis plugin d'analyse |
| M3 | **Échantillon par gabarits et parcours.** Les pages obligatoires sont trouvées, mais pas les **gabarits** (pages types) ni les **processus** complets (commande, inscription) que la méthode RGAA impose de tester de bout en bout | échantillon | haute | Un auditeur passe beaucoup de temps à choisir l'échantillon et à le justifier. C'est aussi là que l'outil peut faire gagner du temps sans toucher au jugement | implémentation (gabarits) + prototype (parcours) |
| M4 | **Parcours enregistrés et rejoués.** Les `PreScanAction` sont écrites à la main | collecte | moyenne | **Remarque :** c'est le meilleur usage de computer use / Claude for Chrome. L'auditeur fait le parcours une fois, guidé par Claude. On l'**enregistre** en étapes déterministes (sélecteurs, valeurs), puis le backend le **rejoue** à l'identique, pour l'audit comme pour le réaudit. On obtient la reproductibilité sans dépendre du modèle au rejeu | prototype |
| M5 | **Installation en un clic dans Claude Desktop.** Il faut aujourd'hui un script d'installation et une configuration MCP manuelle | distribution | moyenne | Un auditeur n'est pas développeur. Le format **MCP Bundle** (`.mcpb`, dépôt `modelcontextprotocol/mcpb`, 2 185 ★) sert précisément à cela. Il reste à vérifier : signature, mises à jour, embarquement du binaire du navigateur | recherche |
| M6 | **Rapport en marque blanche.** Le rapport porte notre présentation | livrable | moyenne | Le cabinet vend **son** rapport, avec son logo, ses mentions et son modèle. Sans marque blanche, on le pousse à recopier dans son propre document, et toute la traçabilité se perd | implémentation |
| M7 | **Réaudit et suivi dans le temps.** `verify_fix` compare des fichiers, mais rien ne compare deux audits d'un même site (corrigé, régressé, nouveau) | suivi | moyenne | Le réaudit après correction est la mission récurrente des cabinets, et le meilleur argument d'abonnement. Il s'appuie sur M4 (mêmes parcours) et sur les éléments transverses (#429) | implémentation |
| M8 | **Relecture croisée (principe des quatre yeux).** Un seul relecteur publie | revue | moyenne | Les cabinets sérieux font relire un audit avant livraison. La page de revue doit permettre une seconde relecture tracée, ou au moins un statut « relu par » | grilling |
| M9 | **Mention de la méthode et de l'usage de l'IA dans le rapport** | livrable | moyenne | L'auditeur engage sa responsabilité. Le commanditaire doit savoir quels tests ont été préparés par la machine, avec quels outils et quel modèle, et que chaque décision est humaine. Ara a déjà un champ « outils utilisés ». **Remarque :** le dire nous protège, le taire nous expose | grilling |
| M10 | **Modèle de prix pour un cabinet** (par audit, par siège, par crédit) | commande | haute | La grille du pilote (#407) a été pensée pour un client final. Un cabinet revend du temps : il faut que le coût de l'outil soit lisible dans son devis | grilling |
| M11 | **Recrutement de deux cabinets pour une démonstration** | commande | haute | D5 vise les cabinets, mais aucun n'est identifié. Sans démonstration en conditions réelles, les prérequis de D9 n'ont pas de date | tâche (humain) |

### Brouillard (pas encore assez net pour un ticket)

- **RGAA 5.** Ara l'anticipe. Nos données sont versionnées par dossier, ce qui est bien. Mais le plan des moteurs, les règles et les plugins sont indexés sur la 4.1.2. Le ticket viendra à la publication du texte.
- **Mode opératoire de l'auditeur** : la documentation pas à pas, de la commande au livrable, dans Claude Desktop. À écrire après la démonstration (M11).
- **Responsabilité contractuelle** : CGU du cabinet, assurance responsabilité civile professionnelle et clause « outil d'aide ». Elle se précise avec M9 et le contrat pilote (#413).

### Hors périmètre de ce socle

- **Applications mobiles (RAAM)** et **logiciels** (EN 301 549, chapitre 11). C'est un autre produit, qui demande une autre collecte (Appium, XCUITest).

## 4. Remarques opportunes

1. **Ne pas promettre « auditeur EAA » avant M1.** La recherche #442 ajoute qu'aucune version d'EN 301 549 n'est encore citée au JOUE au titre de l'EAA : pas de présomption de conformité. Il faut donc écrire « audit selon EN 301 549 v3.2.1, norme en attente de citation sous l'EAA ». Il faut dire « web, au sens EAA (EN 301 549, chapitre 9) », c'est-à-dire WCAG 2.1 AA, ce que nous savons déjà relier à partir des références WCAG de chaque critère RGAA.
2. **Faire du réaudit (M7) l'offre récurrente.** L'audit initial se vend une fois. Le suivi trimestriel avec des parcours rejoués (M4) se vend chaque trimestre, et c'est là que l'automatisation rapporte le plus au cabinet.
3. **Les PDF (M2) relèvent du sidecar.** veraPDF (352 ★, actif, GPL-3.0) est la référence PDF/UA : jamais lié, en sidecar comme le prévoit P-00. On peut y ajouter des contrôles natifs légers avec `lopdf` (2 297 ★, MIT), comme la présence de l'arbre de balisage et la langue du document.
4. **Computer use enregistre, il ne juge pas.** C'est la conséquence de D6 et de M4 : le modèle sert à guider et à enregistrer, et les preuves comme le rejeu sont déterministes.
5. **L'outil doit être irréprochable sur le référentiel.** Le catalogue inventé corrigé le 2026-11-21 montre que la moindre erreur de numéro de critère disqualifie l'outil aux yeux d'un auditeur. Le garde-fou CI ajouté doit couvrir aussi le plugin Codex et toute nouvelle skill.

## 5. Déjà ticketé (rappel, pas de doublon)

- **Fiabilité :** contraste et liens dans le navigateur d'audit (#439, #371).
- **Référentiel et méthode :** méthodologies officielles (#432), types d'audit rapide, complémentaire et complet (#428), éléments transverses (#429).
- **Livrable et environnement :** environnements de test (#430), impact utilisateur et quick win (#431), grille CSV (#434), export vers Ara (#433).
- **Mesures :** plugins d'analyse P-00 à P-10 (#416, #437 pour le lecteur d'écran).
- **Indicateurs et modes :** taux de réouverture (#438), deux modes affichés (#440).
- **Divergences du moteur :** erreur ≠ non-conformité (#435), correspondance axe `button-name` (#436).
