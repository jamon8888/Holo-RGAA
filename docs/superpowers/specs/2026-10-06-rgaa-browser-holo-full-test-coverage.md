# Spécification — exécuter les 258 tests RGAA avec Obscura et Holo

**Date :** 2026-10-06  
**État :** design approuvé dans la conversation; spécification soumise à revue avant plan d’implémentation.  
**Dépôt :** `rgaa-rs` — audit RGAA 4.1.2.

## 1. Décision approuvée

Chaque critère RGAA doit être traité par un composant de Holo-RGAA. La détection statique ne conclut plus à elle seule qu’un critère est non applicable. Elle signale un candidat à une exploration active avec Obscura et Holo.

Pour chaque page auditée, le pipeline parcourt les **106 critères et les 258 tests** du catalogue local. Holo propose les étapes d’exploration et interprète les observations qui demandent un jugement. Obscura exécute les étapes dans le navigateur, collecte l’arbre d’accessibilité et les données de page, puis conserve des captures d’écran des états observés. Les autres mécanismes du dépôt — axe-core, sondes déterministes, contrôles média et comparaisons inter-pages — restent disponibles comme moteurs principaux ou de repli.

Un audit annoncé comme complet ne peut pas se terminer avec un critère `NotTested` ou `NeedsReview`. Si un mécanisme échoue, le pipeline passe au mécanisme de repli prévu. Si tous les mécanismes échouent, l’audit complet échoue explicitement et ne publie pas de rapport de conformité partiel comme s’il était complet.

## 2. Problème observé dans le code actuel

- `rgaa-core/src/na_detection.rs::detect_na` utilise l’absence d’images, formulaires, tableaux, iframes ou médias dans le contexte extrait pour marquer directement ces critères non applicables.
- `rgaa-orchestrator/src/pipeline.rs` applique ensuite cette carte après la fusion et remplace le statut déjà calculé par `NotApplicable`. Aucune exploration du navigateur ne peut donc contester ce statut.
- L’orchestrateur garantit actuellement une entrée par critère, mais les vecteurs de tests de plusieurs résultats restent vides; cela ne prouve pas que les **258 tests numérotés** du catalogue ont tous reçu un résultat.
- Obscura possède des opérations CDP pour charger les pages, parcourir le focus, cliquer et capturer l’écran. Holo possède une voie multimodale. Ces capacités ne sont pas encore assemblées en un parcours d’exploration de chaque page.
- Les réponses Holo de lots peuvent être incomplètes ou demander un outil non disponible. Le lot complet ne doit pas être abandonné pour cette raison.
- L’audit précédent du site avait été volontairement lancé avec `RGAA_MAX_PAGES=2`; ce plafond ne constitue pas la couverture cible.

## 3. Invariants d’acceptation

1. **Couverture complète par page :** un résultat réussi contient les 106 critères et exactement une décision pour chacun des tests attendus par le catalogue, soit 258 tests.
2. **Pas de N/A anticipé :** une observation statique d’absence crée une hypothèse et une liste de cibles à rechercher; elle ne modifie pas le statut final.
3. **N/A prouvé :** un test ou critère ne peut être non applicable qu’après un contrôle actif montrant l’absence de la situation correspondante sur la page explorée, avec observations et références de preuve enregistrées.
4. **Pas de sortie indéterminée dans un audit réussi :** chaque test d’un rapport complet est `Pass`, `Fail` ou `NotApplicable`. Une erreur non résolue arrête l’audit complet avec une erreur de couverture explicite.
5. **Axe-core partiel ne valide pas par silence :** une règle axe partielle peut fournir un échec observé; l’absence de violation doit être complétée par un mécanisme de couverture complète.
6. **Chaque verdict est traçable :** il référence le ou les tests, le composant qui les a évalués, l’observation navigateur ou le constat déterministe, et les captures associées lorsqu’elles éclairent le verdict.
7. **Comparaison site complète :** les critères inter-pages sont calculés sur l’ensemble effectivement crawlé. Si la limite de pages empêche d’atteindre une couverture complète, un audit en mode complet échoue au lieu de produire une conclusion de conformité du site.
8. **Actions Holo bornées :** Holo choisit seulement des actions typées sur des cibles inventoriées par Obscura. Le modèle ne fournit ni JavaScript libre ni sélecteur arbitraire.
9. **Pas d’action à effet externe :** aucun formulaire n’est soumis; aucune suppression, commande, achat, création de compte ou navigation hors origine n’est activée automatiquement.

## 4. Parcours cible

### 4.1 Crawl complet

Le mode d’audit complet découvre les pages internes atteignables selon la politique de crawl, les déduplique et audite chaque page. La limite `max_pages` reste configurable pour protéger l’exécution; si elle est atteinte avant la fin du crawl, le rapport ne peut pas être présenté comme complet. Les critères inter-pages reçoivent les observations du même ensemble complet de pages.

### 4.2 Contexte initial et inventaire Obscura

Pour chaque page, Obscura crée une session réutilisable et recueille :

- une capture initiale du rendu;
- le DOM et les inventaires utiles aux mécanismes (images, médias, formulaires, tableaux, liens, contrôles, landmarks);
- l’arbre d’accessibilité;
- l’état initial du focus et l’ordre de tabulation;
- les URL internes et les contrôles susceptibles de révéler du contenu additionnel.

Les observations d’absence deviennent des `ApplicabilityCandidate` associés aux critères/tests concernés. Elles sont transmises au planificateur d’exploration, elles ne sont pas fusionnées comme verdicts.

### 4.3 Plan d’exploration dirigé par Holo

Holo reçoit le contexte structuré et la capture multimodale, ainsi que les identifiants de cibles sûres produits par Obscura. Il renvoie un plan JSON validé par un schéma, comprenant des actions typées telles que :

- défiler vers une zone inventoriée;
- parcourir le focus par Tab et fermer un état avec Échap;
- activer un contrôle de divulgation explicitement classé comme sûr;
- suivre un lien interne de même origine;
- demander une capture et une nouvelle observation après le changement d’état.

Obscura vérifie le plan avant exécution : action autorisée, cible présente dans l’inventaire, même origine et pas de soumission/effet métier. Un plan invalide est refusé; le pipeline demande à Holo un plan corrigé ciblé ou choisit un parcours déterministe de repli.

Les captures sont conservées sous une structure stable par audit, page et état, par exemple `evidence/<audit_id>/<page_id>/state-000.png`. Les preuves du même état sont réutilisées par les critères concernés; le pipeline ne recapture pas la page pour chaque critère.

### 4.4 Exécution des tests et replis

Le catalogue `criteres.json` fournit les identifiants de tests attendus. Chaque test est affecté à un moteur primaire et à une chaîne de replis issue de `EnginePlan`, `MechanismRegistry` et des capacités navigateur :

1. exécuter axe-core quand une règle existe dans Obscura;
2. exécuter les sondes déterministes et comportementales sur le même état navigateur;
3. transmettre les observations et captures pertinentes à Holo pour les jugements sémantiques/visuels;
4. utiliser les analyseurs média et la comparaison inter-pages quand la nature du test le demande;
5. si la réponse d’un lot Holo est invalide ou incomplète, ne rejouer que les identifiants manquants, d’abord en lots réduits puis test par test. Une réponse valide à un autre test du lot n’est pas recalculée.

Les mécanismes réutilisent le même contexte, la même session et les mêmes preuves pour minimiser les appels navigateur et Holo. Le plan d’exploration est groupé par page et les résultats Holo sont groupés par critères/tests compatibles avec la limite de contexte.

### 4.5 Décision d’applicabilité

Une hypothèse N/A est réexaminée après l’exploration. Une absence n’est prouvée qu’avec :

- l’inventaire navigateur initial;
- la couverture des états atteignables pertinents (navigation, focus, divulgations, navigation interne);
- les contrôles ciblés de la catégorie concernée;
- une référence à la capture et aux observations qui justifient cette absence.

Si l’exploration révèle un élément pertinent, les tests correspondants sont exécutés normalement. Si l’absence est établie, chaque test concerné reçoit `NotApplicable` avec la preuve d’applicabilité. Une réponse Holo seule ne suffit pas à établir l’absence : le verdict N/A doit être adossé à une observation reproductible d’Obscura ou d’un autre mécanisme déterministe.

### 4.6 Agrégation et rapport

Un test reçoit `Pass`, `Fail`, `NotApplicable` ou une erreur d’exécution. Le statut du critère se réduit depuis les tests attendus :

- au moins un test échoue → critère `Fail`;
- tous les tests sont N/A → critère `NotApplicable`;
- tous les tests sont réglés et aucun n’échoue, avec au moins un test applicable → critère `Pass`;
- test manquant ou mécanisme en erreur après repli → échec de complétude de l’audit, aucun rapport final déclaré complet.

Le rapport JSON/PDF présente, pour chaque critère et test : identifiant, intitulé officiel, statut, moteur primaire, composant qui a réellement tranché, constat, confiance si Holo intervient, et liens vers les captures/observations. La synthèse indique `criteria_completed / 106`, `tests_completed / 258` par page, nombre de pages découvertes/auditées, appels Holo, replis et erreurs.

## 5. Modèle de données visé

Réutiliser `TestOutcome` et enrichir la preuve sans perdre la compatibilité des anciens rapports :

- `test_key` doit appartenir aux tests déclarés du critère;
- `status` ne peut pas être `NeedsReview`/`NotTested` dans un résultat d’audit déclaré complet;
- `source` porte le moteur qui a tranché;
- `evidence` contient une explication concise et reproductible;
- les références d’artefacts relient la décision aux captures Obscura et aux observations structurées.

La complétude est vérifiée contre les **clés de test**, pas seulement contre le nombre de résultats, afin d’empêcher qu’un doublon ou un test inconnu masque un identifiant absent. Les formats préexistants restent lisibles via des champs optionnels/defaultés; ils conservent leur état de couverture historique et ne deviennent pas rétroactivement des audits complets.

## 6. Sécurité et robustesse

- Les contenus de page restent des données non fiables dans les prompts Holo.
- Holo ne peut ni exécuter du code fourni par le site ni inventer des actions/identifiants hors inventaire.
- Toute action potentiellement irréversible ou extérieure à la lecture seule est interdite.
- Chaque étape a des délais, un budget d’actions par page et une limite de profondeur; dépasser le budget produit une erreur de complétude explicite.
- Les captures et preuves sont liées à l’audit et à l’URL concernée; les secrets et en-têtes sensibles ne sont jamais enregistrés.
- Les erreurs sont conservées avec leur cause; elles ne sont jamais converties en réussite ou en N/A.

## 7. Vérifications d’acceptation

Les tests d’implémentation doivent démontrer au minimum :

1. l’absence d’images dans le contexte initial ne produit qu’un candidat N/A et n’écrase aucun résultat navigateur;
2. une image découverte après ouverture d’un état interne est testée au lieu d’être classée N/A;
3. une page réellement sans contenu média aboutit à N/A uniquement avec inventaire, exploration et capture référencés;
4. les 106 critères et les 258 identifiants de test sont tous présents une fois dans une exécution réussie sur fixture;
5. un axe partiel silencieux déclenche son repli et ne donne pas `Pass`;
6. une réponse Holo manquant des résultats pour quelques tests ne relance que ces tests;
7. un appel d’outil non autorisé/invalide par Holo est rejeté et passe au repli au lieu d’abandonner le lot;
8. les actions de soumission de formulaire, d’origine externe et les actions métier sont refusées;
9. les captures sont bien écrites et leurs références se retrouvent dans le rapport;
10. un audit complet ne peut pas se finaliser avec `NeedsReview`, `NotTested`, un test attendu manquant ou un crawl tronqué;
11. les critères inter-pages utilisent toutes les pages découvertes du même crawl;
12. les anciens rapports JSON restent désérialisables.

## 8. Hors périmètre

- Modifier le site audité ou envoyer des données de formulaire.
- Présenter un audit automatisé comme certification réglementaire officielle.
- Rejouer tous les contrôles pour corriger une réponse manquante d’un seul test.
- Déduire la conformité à partir d’une simple capture lorsque le test exige des preuves DOM, audio, clavier ou inter-pages.

## 9. Revue de la spécification

- Les exigences explicitement validées par l’utilisateur sont reprises : aucune conclusion N/A par détection statique seule; exploration Obscura pilotée par Holo; captures d’écran; crawl complet; replis entre composants; pas de critère laissé en `NeedsReview`/`NotTested` dans un audit déclaré complet.
- Le plan préserve la distinction entre un verdict de conformité et un échec d’exécution : un outil incapable de conclure ne fabrique pas un `Pass` ou un `Fail`.
- Les actions Holo sont strictement structurées et validées pour éviter le problème antérieur d’appel à `evaluate_criteria` non autorisé.
- Le nombre de tests est comparé aux clés du catalogue (258) et non à un simple total de lignes.
- Aucun comportement de navigation destructive n’est autorisé.

**À confirmer par l’utilisateur :** cette spécification constitue le contrat avant rédaction du plan d’implémentation.
