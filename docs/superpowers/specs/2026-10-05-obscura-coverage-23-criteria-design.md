# Design — Obscura pour les 23 critères RGAA restants

**Date :** 2026-10-05  
**Statut :** spécification proposée, en attente de revue humaine  
**Base fonctionnelle :** archive `Holo-RGAA-master.zip` jointe à la demande. Le checkout courant est antérieur à certaines évolutions du ZIP ; l’implémentation devra reporter les changements nécessaires dans le checkout sans remplacer les changements locaux non liés.

## Objectif

Raccorder les contrôles manquants des critères RGAA 1.6, 3.3, 4.12, 4.13, 8.7, 10.9, 10.12, 10.13, 11.3, 11.8, 11.11, 12.1, 12.2, 12.4, 12.5, 12.8, 12.9, 12.10, 12.11, 13.3, 13.10, 13.11 et 13.12 au parcours d’audit.

Obscura est responsable de l’observation et des interactions navigateur. `rgaa-rules` transforme ces observations en résultats de mécanismes. `rgaa-orchestrator` décide quand les vérifications par page et les comparaisons inter-pages sont exécutées. Le statut final doit garder une trace de la méthode, de la couverture, des éléments observés et de la raison d’une revue.

Réduire les appels Holo inutiles est une exigence fonctionnelle : les contrôles déterministes passent d’abord ; Holo ne reçoit que les critères non tranchés qui bénéficient réellement d’un jugement IA. Les identifiants de critères sont dédupliqués avant l’appel et les résultats déterministes ne sont jamais envoyés pour obtenir un second verdict.

## Contrats de résultat

Chaque mécanisme retourne une issue explicite :

- `fail` avec observation et preuve quand un échec vérifiable est trouvé ;
- `pass` seulement si la couverture du mécanisme est complète pour les tests revendiqués et si tous sont vérifiés ;
- `review` lorsqu’un jugement humain, une inspection AT réelle, une mesure ambiguë ou un rendu non observable est nécessaire ;
- `NotTested`/erreur si la navigation, l’instrumentation ou l’extraction a échoué. Une erreur technique ne constitue pas une preuve de conformité ni de non-conformité.

Les résultats doivent être rattachés aux clés de tests RGAA disponibles dans le catalogue. Un mécanisme partiel peut faire échouer un critère, mais son absence de signal ne peut pas produire `Pass`. Les critères prévus comme manuels ne peuvent pas obtenir un `Pass` Holo par défaut.

## Architecture retenue

### Obscura — observations de page

Étendre le pont Obscura/CDP avec des opérations réutilisables et bornées : capture de contexte DOM/accessibilité, styles calculés, changement temporaire de viewport/zoom, injection et retrait de styles de test, navigation clavier, séquences de pointeur/touch, suivi des popups et événements, et collecte des métadonnées de documents liés. Les opérations qui modifient la page doivent restaurer l’état ou jeter la session après observation.

Les mesures par page partagent une même navigation et un contexte réutilisé. Les contrôles qui exigent une nouvelle page ou un nouvel état le déclarent explicitement. Toutes les actions sont limitées en durée et en nombre ; aucune soumission sensible de formulaire n’est déclenchée.

### rgaa-rules — mécanismes et couverture

Déclarer les mécanismes dans le registre commun, avec type, moteur, couverture, issues possibles, tests RGAA couverts, fixtures et provenance. Séparer les vérifications statiques des sondes comportementales et du mécanisme site-level. Les heuristiques indiquent `review` dès que la preuve n’est pas suffisante pour trancher.

### Orchestrateur — séquencement et contrôles site-level

Pour chaque page : collecter le contexte une fois, exécuter les contrôles déterministes indépendants en premier, puis identifier les critères encore ouverts. Pour le site : conserver les observations normalisées par URL, puis comparer les pages du même audit pour 12.1, 12.2, 12.4 et 12.5. Une URL qui échoue à l’audit doit rester visible dans le résultat et faire baisser/annuler la couverture au lieu de disparaître silencieusement.

### Holo — recours ciblé, dédupliqué et groupé

Construire l’ensemble des seuls critères non résolus pour lesquels le jugement du modèle ajoute une information exploitable. Exclure :

- les critères déjà tranchés par une preuve déterministe complète ;
- les critères déterministes dont le contrôle spécialisé n’a pas encore tourné ;
- les critères manuels dont la question exige l’écoute, un lecteur d’écran réel, une confirmation de parcours ou une décision humaine ;
- les doublons issus du recouvrement `IaAssiste` / `PartiallyAutomatable`.

Réutiliser le contexte préparé une fois par page. Regrouper les critères par tier de modèle et envoyer un batch par tier/page, avec découpage seulement si une limite de taille l’impose. Ne pas refaire un appel individuel pour les réponses manquantes ou mal formées : ces critères deviennent `NeedsReview` avec la cause visible. Les reprises réseau sont réservées aux erreurs transitoires documentées et plafonnées ; elles ne relancent jamais les critères déjà reçus. La télémétrie rapporte critères envoyés, nombre d’appels, tentatives et raisons d’exclusion, sans journaliser de secrets ni de contenu sensible de formulaire.

## Couverture proposée des 23 critères

| Critère | Contrôle Obscura prévu | Issue prudente / limite |
|---|---|---|
| 1.6 | Détecter images informatives et liaisons de description détaillée (`longdesc`, `aria-describedby`, liens/blocs associés) | `review` pour décider si une description détaillée est nécessaire ou si elle est pertinente |
| 3.3 | Mesurer le contraste des bordures, icônes, composants et indicateurs sur styles calculés et rendus mesurables | `review` pour dégradés, images, pseudo-éléments ou rôle visuel incertain |
| 4.12 | Identifier les médias non temporels puis tester les contrôles accessibles au clavier et au pointeur | `review` si le contrôle custom ne peut pas être actionné/interprété avec fiabilité |
| 4.13 | Examiner nom/rôle/état du player dans l’AXTree et ses transitions | `review` lorsque la compatibilité exige un lecteur d’écran ou une AT réelle |
| 8.7 | Inventorier les segments de texte et les déclarations `lang`/`xml:lang`; détecter les segments candidats non balisés | `review` si détection de langue ambiguë ; aucune langue devinée ne produit un `Pass` |
| 10.9 | Capturer les composants et indices de forme/position ; rapprocher styles/rendu et texte équivalent | `review` pour le jugement visuel de l’information portée uniquement par la forme ou position |
| 10.12 | Injecter temporairement les espacements RGAA, relever clipping, chevauchement et contenu masqué, puis restaurer | `review` pour les cas de rendu non mesurables ; pass seulement après contrôle complet des zones testées |
| 10.13 | Parcourir focus/hover, repérer les contenus additionnels et vérifier persistance, fermeture et atteignabilité | `review` pour la pertinence du contenu ou les interactions qui requièrent une décision humaine |
| 11.3 | Comparer les noms/libellés des champs d’une même fonction au sein du formulaire | `review` pour établir la même fonction lorsque le DOM ne suffit pas |
| 11.8 | Inspecter les listes de choix et la structure `optgroup`/labels ; signaler les longues listes candidates | `review` pour la logique du regroupement si elle ne se déduit pas de la structure |
| 11.11 | Déclencher uniquement une validation locale non destructive et observer message, association et suggestion | `review` pour la pertinence de la correction proposée ; ne jamais envoyer le formulaire |
| 12.1 | Sur le corpus de pages auditées, identifier navigation, plan de site et recherche ; compter les systèmes distincts | `review` si l’échantillon est incomplet ou si le périmètre du site n’est pas établi |
| 12.2 | Comparer présence, ordre et position structurelle des menus/barres entre pages | `review` si les pages auditées ne représentent pas les gabarits du site |
| 12.4 | Vérifier le lien vers le plan de site sur chaque page et vérifier sa cible | `review` si le crawl n’a pas couvert toutes les pages requises |
| 12.5 | Comparer la présence et l’accessibilité du moteur de recherche sur les pages du corpus | `review` si recherche externe, authentification ou échantillon incomplet |
| 12.8 | Enregistrer la séquence Tab/Shift+Tab, coordonnées et identifiants stables des éléments focusés | `review` pour comparer l’ordre visuel si les limites géométriques sont ambiguës |
| 12.9 | Tester les séquences clavier bornées et détecter focus bloqué, en tenant compte des dialogues/modalités | `review` pour pièges possibles dans des composants custom ou états inaccessibles |
| 12.10 | Instrumenter dès le chargement les listeners clavier et attributs de raccourcis pour identifier les touches simples | `review` pour prouver désactivation/remappage et effets globaux ; ne pas conclure sur les seuls noms de listeners |
| 12.11 | Tester les zones hover/focus, transfert pointeur, clavier et fermeture du contenu additionnel | `review` si le contenu ne peut être identifié comme additionnel ou si persistance incertaine |
| 13.3 | Inventorier liens/documents, MIME, extension, réponse HTTP ; inspecter métadonnées/structure accessibles selon format pris en charge | `review` pour contenu documentaire ou équivalence de version ; outil de validation externe si nécessaire |
| 13.10 | Exercer les gestes détectables en émulation tactile et chercher une commande simple équivalente | `review` pour équivalence fonctionnelle ; ne jamais exécuter une action sensible |
| 13.11 | Simuler pointer-down puis déplacement/annulation sans relâchement activateur et observer l’activation | `review` si l’action ne peut pas être annulée sans effet métier |
| 13.12 | Instrumenter événements capteurs/mouvement et rechercher une commande UI de remplacement | `review` pour l’équivalence et les fonctions nécessitant matériel réel |

## Erreurs, preuves et métriques

Chaque résultat conserve URL, critère/test, source, couverture, observation et références d’évidence (capture/AXTree si activée). Les données de capture sont hachées/référencées plutôt que copiées dans les prompts sans nécessité. Un outil absent, un timeout, un contexte incomplet ou une URL exclue ne doit jamais être converti en `Pass` ni disparaître du taux de couverture.

L’appel Holo doit recevoir uniquement les observations nécessaires, expurgées des valeurs de saisie sensibles. Les journaux comptent les appels et le nombre de critères inclus, mais n’exposent ni clé API ni données utilisateur.

## Découpage d’implémentation proposé

1. **Contrats et orchestration** : issues explicites, propagation test-level, éviter les doublons Holo, conserver les erreurs de page dans les agrégats.
2. **Sondes mesurables par page** : 1.6, 3.3, 8.7, 10.12, 11.3, 11.8, 11.11, 13.3.
3. **Interactions et rendu navigateur** : 4.12, 4.13, 10.9, 10.13, 12.8–12.11, 13.10–13.12.
4. **Comparaisons inter-pages** : 12.1, 12.2, 12.4, 12.5.
5. **Fixtures de corpus et validation E2E Obscura** pour chaque mécanisme, suivi de la mesure des appels Holo avant/après.

Les critères 4.13, 10.9, 11.11 et les tests de 1.6/13.3 qui nécessitent un jugement restent `NeedsReview` lorsque la preuve automatisée est insuffisante. Le projet ne revendique pas une conformité basée uniquement sur une inspection de DOM.

## Critères d’acceptation

- Les 23 critères ont des mécanismes actifs déclarés, ou un état explicite `NeedsReview`/`NotTested` avec raison et propriétaire du contrôle restant.
- Aucun mécanisme partiel n’émet un `Pass` au niveau du critère.
- Les cas d’échec de navigateur/crawl sont visibles et affectent la couverture.
- Les comparaisons 12.x utilisent l’ensemble des pages du même audit, avec sa couverture d’échantillon indiquée.
- Les 13 identifiants actuellement dupliqués entre les deux files Holo ne génèrent plus d’appel perdu ou de remplacement silencieux.
- Une métrique par page/audit démontre que Holo ne reçoit que les critères encore indécidables et que les appels sont batchés/dédupliqués.
- Les critères exigeant lecteur d’écran, écoute, matériel ou équivalence sémantique restent en revue humaine sauf preuve testée et justifiée.
- Fixtures pass/fail et vérification E2E sous Obscura sont présentes pour chaque mécanisme déterministe revendiquant une décision.
