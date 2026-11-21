# Contrôler l'accessibilité des PDF : veraPDF en sidecar ou contrôles natifs lopdf

> **Date de consultation des sources : 2026-11-21.** Ticket [#443](https://github.com/jamon8888/Holo-RGAA/issues/443), carte [#441](https://github.com/jamon8888/Holo-RGAA/issues/441), manque **M2** ([`manques-socle-auditeur.md`](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/strategy/manques-socle-auditeur.md)).
> Sources primaires uniquement (textes officiels, documentation et code des projets). Chaque fait est suivi de son URL ; ce qui n'a pas été trouvé est signalé explicitement.

## 1. Question posée

Le socle détecte aujourd'hui les liens vers des PDF au titre du test RGAA 13.3, mais n'ouvre pas le fichier : balisage, ordre de lecture, alternatives, langue et PDF/UA ne sont pas contrôlés ([M2](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/strategy/manques-socle-auditeur.md)). La règle d'architecture est **« Rust d'abord, sidecar sinon »** : une bibliothèque Rust mûre est préférée ; à défaut, ou si elle est nettement moins bonne (langues, précision, licence de liaison), un exécutable externe est piloté en sidecar ([P-00 §2](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md)). La même capacité peut exister sous les deux formes, l'utilisateur choisissant dans `rgaa.toml` et la *Preuve* indiquant la variante via `version_modele` ([P-00 §2](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md)).

## 2. Ce que demandent les référentiels

### RGAA 4.1.2 — critère 13.3, test 13.3.1

Le critère 13.3 est : « Dans chaque page web, chaque document bureautique en téléchargement possède-t-il, si nécessaire, une version accessible (hors cas particuliers) ? ». Le test 13.3.1 accepte un document compatible avec l'accessibilité, une version alternative en téléchargement, ou une version alternative HTML ([critères et tests, #13.3](https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/#13.3)).

La méthodologie est explicite : **« Pour les documents au format .pdf, analyser le fichier avec l'outil PAC (PDF Accessibility Checker) et vérifier l'absence d'erreur d'accessibilité »**. La note précise que PAC analyse aussi des critères de qualité « par exemple la norme PDF/UA », mais que **seules les erreurs relatives aux critères de la « Liste document bureautique en téléchargement » rendent le critère non conforme** ; PAC ne fonctionne que sous Windows, contrôle manuel sur Mac ([critères et tests, #13.3](https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/#13.3)). Le référentiel complet est publié en PDF par la DINUM ([RGAA v4.1.2](https://accessibilite.numerique.gouv.fr/doc/RGAA-v4.1.2.pdf)).

**Conséquence :** un résultat PDF/UA conforme n'est pas, à lui seul, un verdict RGAA 13.3. Il fournit une *preuve* à faire entrer dans la revue humaine, exactement comme le impose le contrat de plugin : `Portee::Mesuree` ne peut donner `fail` que sur un test classé automatique, tout le reste donne `review` ([P-00 §5](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md)).

### EN 301 549 V3.2.1 — chapitre 10 « Non-web documents »

Le chapitre 10 s'applique aux documents qui ne sont pas des pages web, ne sont pas intégrés dans une page web, et sont fournis avec une page web sans être rendus avec elle — donc aux **documents téléchargeables** ([EN 301 549 V3.2.1, §10.0](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf)). Il mappe des critères WCAG 2.1 sur ces documents, notamment :

| Clause | Exigence (traduction de la norme) |
|---|---|
| [10.1.1.1](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf) | Non-text content (alternatives) |
| [10.1.3.1 / 10.1.3.2](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf) | Info and relationships / Meaningful sequence (structure, ordre de lecture) |
| [10.2.4.2](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf) | Document titled (titre du document) |
| [10.3.1.1 / 10.3.1.2](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf) | Language of document / Language of parts (langue par défaut et changements) |
| [10.4.1.1 / 10.4.1.2](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf) | Parsing / Name, role, value |

**Non trouvé :** aucune occurrence de « PDF/UA », « ISO 14289 » ni « tagged PDF » dans le texte officiel [EN 301 549 V3.2.1](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf) (recherche textuelle sur le PDF téléchargé le 2026-11-21). La norme harmonisée n'impose donc pas directement PDF/UA : elle impose des *résultats* d'accessibilité. PDF/UA est un moyen de les satisfaire, pas une fin en soi.

### PDF/UA : deux normes distinctes

- **PDF/UA-1** = [ISO 14289-1:2014](https://www.iso.org/standard/64599.html), « Use of ISO 32000-1 », 2ᵉ édition, confirmée en 2025 (fiche ISO).
- **PDF/UA-2** = [ISO 14289-2:2024](https://www.iso.org/standard/82278.html), « Use of ISO 32000-2 », 1ʳᵉ édition, mars 2024 (fiche ISO ; page directe retournant 403 pendant la consultation, information relevée dans l'extrait de recherche de la fiche officielle).

## 3. veraPDF : ce que ça apporte vraiment

### Profils, règles, sortie machine

veraPDF se décrit comme « un validateur de format de fichiers open source conçu à cette fin, couvrant toutes les parties PDF/A et PDF/UA et tous les niveaux de conformité » ([verapdf.org](https://verapdf.org/)). La CLI embarque les profils `ua1` (PDF/UA-1) et `ua2` (« PDF/UA-2 + Tagged PDF »), en plus des profils PDF/A et WTPDF (sortie de `verapdf -l`, 2026-11-21).

Le moteur implémente chaque énoncé « shall » des spécifications sous forme de profils XML ; **pour PDF/UA, seuls les contrôles vérifiables par machine sont exécutés**, les points nécessitant un jugement humain étant décrits par le [protocole Matterhorn](https://pdfa.org/resource/the-matterhorn-protocol/) ([documentation veraPDF, « Validation »](https://docs.verapdf.org/validation/)). Les règles sont publiées et documentées : par exemple [PDF/UA-1 rule 6.2-1](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (`MarkInfo/Marked == true`), [7.1-11](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (`StructTreeRoot` présent), [7.1-9](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (`dc:title` dans XMP), [7.1-10](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (`DisplayDocTitle == true`), [7.2-29](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (validité du code de langue), [7.3-1](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (`Alt` ou `ActualText` sur les figures), [7.4.2-1](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (saut de niveau de titre) et [7.5-1/2](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) (en-têtes de tableaux). Le jeu de règles PDF/UA-2 est publié séparément ([PDFUA Part 2 rules](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-2-rules)).

Sortie machine confirmée sur la version installée : `--format` accepte `raw`, `xml`, `text`, `html`, `json` (défaut `xml`) ; le JSON contient `report.jobs[].validationResult[0]` avec `compliant`, `statement`, `profileName`, `passedRules`, `failedRules`, `failedChecks` et `ruleSummaries` (clause, numéro de test, description, tags). Options utiles pour un sidecar : `--maxfailures`, `--maxfailuresdisplayed`, `--processes`, `--success`, `--addlogs`, `--password` (sortie de `verapdf --help`, 1.30.4). Le format XML est le format de rapport « machine readable » historique ([CLI](https://docs.verapdf.org/cli/)).

### Licence et empaquetage

Le dépôt officialise une **double licence GPLv3+ et MPLv2+** ([README, « Licensing »](https://github.com/veraPDF/veraPDF-library) ; fichiers [`LICENSE.GPL`](https://github.com/veraPDF/veraPDF-library/blob/integration/LICENSE.GPL) et [`LICENSE.MPL`](https://github.com/veraPDF/veraPDF-library/blob/integration/LICENSE.MPL)). Le ticket parle de GPL-3.0 : c'est exact pour l'une des deux branches, mais la double licence est un fait à retenir. La bannière de l'exécutable indique « Released under the GNU General Public License v3 and the Mozilla Public License v2 or later » (sortie locale).

Prérequis de compilation : **Java 8, 11, 17, 21 ou 25 + Maven 3+** ([README](https://github.com/veraPDF/veraPDF-library)). Distribution : une archive zip d'environ 32 Mo contenant un installateur IzPack (`verapdf-izpack-installer-1.30.4.jar`), un script batch Windows et un script bash Linux/macOS ; l'installation non interactive se fait avec un XML et ne dépose ici que la CLI ([guide d'installation](https://docs.verapdf.org/install/)). Le CLI se lance par un script shell qui appelle `java -jar` — ce n'est **pas** un protocole JSON-lignes sur stdin/stdout : un adaptateur est nécessaire pour le [§7 bis de P-00](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md).

### Limites

- Contrôles **machine uniquement** pour PDF/UA ([documentation](https://docs.verapdf.org/validation/)) : ni pertinence d'une alternative, ni exactitude de l'ordre de lecture par rapport à la maquette, ni usage réel par un lecteur d'écran.
- Le résultat est une conformité **au profil choisi**, pas au RGAA : la méthodologie RGAA 13.3 demande PAC et ne retient que les erreurs de sa liste ([critères et tests](https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/#13.3)).
- Coût de démarrage JVM et coût sur gros documents mesurés ci-dessous.

## 4. lopdf : ce qui est vérifiable nativement

[lopdf](https://github.com/J-F-Liu/lopdf) est une « bibliothèque Rust pour la manipulation de documents PDF », sous **licence MIT** (exception : la police Montserrat), 2,3 k étoiles, version `0.39.0` utilisée ici, prérequis **Rust ≥ 1.85** ([README](https://github.com/J-F-Liu/lopdf)). La documentation de l'API expose `Document::load`, `get_pages`, `extract_text`, `get_object`, `dereference` ([docs.rs/lopdf 0.39.0](https://docs.rs/lopdf/0.39.0/lopdf/)). Le README renvoie à la [PDF 1.7 Reference d'Adobe](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf) pour la structure du format.

Un sondage de 150 lignes, écrit pour cette recherche hors du dépôt, a parcouru les objets via `doc.catalog()`, les clés `StructTreeRoot`, `MarkInfo/Marked`, `Lang`, `Metadata`, le dictionnaire `Info/Title`, puis l'arbre de balisage par les clés `K`/`Kids` en suivant les références, en comptant les `/S` (dont `H1`–`H6`) et les attributs `/Alt` et `/ActualText`. Résultats sur les 50 PDF :

- `StructTreeRoot` présent : **50/50** ; `MarkInfo/Marked = true` : **50/50** ;
- langue (`Lang`) lisible : **44/50** ; titre (`Info/Title`) : **35/50** ; flux XMP (`Metadata`) : **50/50** ;
- l'arbre est parcourable : jusqu'à **27 128 éléments de structure** et **921 titres H1–H6** sur le PDF de 186 pages ;
- alternatives : sur quatre PDF du corpus dédiés à la propriété « Figure », le sondeur lit `Alt` sur celui qui contient `Alt`, `ActualText` sur les deux qui contiennent `ActualText`, et rien sur celui qui échoue — la **présence** d'une alternative est donc détectable nativement (règle [7.3-1](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules)).

**Non trouvé dans lopdf :** aucun validateur PDF/UA, aucun modèle de conformité, aucune API dédiée à l'arbre de balisage. Une recherche de code `StructTreeRoot` dans le dépôt [`J-F-Liu/lopdf`](https://github.com/J-F-Liu/lopdf) renvoie **0 résultat** (2026-11-21) et la page `Document` de [docs.rs](https://docs.rs/lopdf/0.39.0/lopdf/struct.Document.html) ne contient ni `StructTreeRoot`, ni `Lang`, ni `MarkInfo`. lopdf donne des **objets**, pas des **règles** : la validité du code de langue, la hiérarchie des titres, la conformité des types de structure, l'ordre de lecture correct ou l'adéquation d'une `Alt` restent à écrire soi-même.

## 5. Essai réel sur 50 PDF publics

### Protocole

- Machines et versions : Rust `1.100.0`, lopdf `0.39.0` (build `--release`) ; veraPDF `1.30.4` (build 2026-11-09) sur OpenJDK `21.0.13.1`, Linux x86_64, 4 cœurs / 7,7 Go.
- Corpus : 2 PDF institutionnels ([RGAA v4.1.2, 131 p.](https://accessibilite.numerique.gouv.fr/doc/RGAA-v4.1.2.pdf) ; [ETSI EN 301 549 V3.2.1, 186 p.](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf)) + **48 PDF tirés au sort** dans les dossiers `PDF_UA-1/` et `PDF_UA-2/` du [corpus public veraPDF](https://github.com/veraPDF/veraPDF-corpus) (liste en annexe).
- Commandes : `verapdf -f ua1 --format json --processes {1,4} pdfs/*.pdf` ; sondage lopdf sur les mêmes fichiers. Chronométrage wall-clock (`date +%s%N`) + champ `processingTime.difference` du rapport JSON.

### Mesures

| Scénario | Temps wall-clock | Détail |
|---|---|---|
| veraPDF ua1, 50 PDF, 1 processus | **51 147 ms** | 45 819 ms de temps de traitement cumulé ; médiane 132 ms/Fichier ; 20 conformes / 30 non conformes |
| veraPDF ua1, 50 PDF, 4 processus | **44 502 ms** | 65 940 ms cumulés (chevauchement) ; médiane 273 ms/fichier ; mêmes 20/30 conformes |
| veraPDF ua1, 1 petit fichier isolé | **1 680 ms** | démarrage JVM + validation : le coût fixe domine sur les petits PDF |
| veraPDF ua1, ETSI 186 p. / RGAA 131 p. | **22 406 ms / 13 476 ms** | les deux gros documents représentent à eux seuls 35,9 s |
| lopdf, 50 PDF (passe à chaud) | **1 711 ms** (34,21 ms/fichier en moyenne) | 0 échec de lecture ; 2 833 ms au premier passage à froid |
| lopdf, ETSI / RGAA | **1 240 ms / 1 172 ms** | le parcours complet de l'arbre de balisage domine |

Ordre de grandeur : **environ 30× plus rapide** en natif sur le lot, et le démarrage d'un processus veraPDF coûte plus d'une seconde et demie avant toute validation.

### Différence entre profils (contrôle croisé)

Sur les quatre PDF du corpus dédiés à la Figure (`8.2.5.28.2-t01-*`), `verapdf -f ua1` donne **0 conforme / 4** (7 704 ms) tandis que `-f ua2` donne **3 conformes / 4** (6 254 ms). Le profil n'est donc pas interchangeable : un plugin doit enregistrer explicitement le profil utilisé dans la preuve.

### Limites de l'essai

- Le sondage lopdf est un prototype jetable, pas un plugin P-00 : il ne produit ni `Preuve` ni JSON Schema, et ne teste qu'une douzaine de signaux.
- Les 48 PDF du corpus sont des fichiers de test volontairement miniaturisés et souvent non conformes ; les temps ne sont pas représentatifs d'un audit de 50 documents réels de grande taille. Les deux PDF institutionnels, eux, le sont.
- Un seul passage par configuration veraPDF, sans répétition statistique ; les écarts entre 1 et 4 processus ne doivent pas être surinterprétés (ordonnanceur, caches disque).
- **Non trouvé :** aucun benchmark officiel publié par veraPDF ou lopdf sur 50 PDF ([dépôt veraPDF](https://github.com/veraPDF/veraPDF-library), [dépôt lopdf](https://github.com/J-F-Liu/lopdf)) ; les chiffres ci-dessus sont locaux et datés.

## 6. Recommandation — « Rust d'abord, sidecar sinon »

**Retenir les deux formes, avec le natif par défaut et veraPDF en sidecar activable.** C'est la réponse conforme à [P-00 §2](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md), qui prévoit expressément qu'une même capacité existe sous les deux formes et que l'utilisateur choisisse.

1. **Natif (plugin Rust, premier choix) — pré-tri et collecte de faits.** lopdf lit les 50 fichiers sans échec, ~30× plus vite, sans JVM ni dépendance GPL. Le plugin `rgaa-analysis-pdf` émet des preuves `Portee::Mesuree` sur ce que le sondage a effectivement lu : présence de `StructTreeRoot`, de `MarkInfo/Marked` et de `Lang`, présence d'un flux XMP, titre `Info`, décompte des éléments de structure et des `H1`–`H6`, présence de `Alt`/`ActualText`, taille et nombre de pages. `dc:title` et `DisplayDocTitle` sont à ajouter. Ces faits servent d'`Indice` pour ordonner la file de revue, **jamais de statut** ([P-00 §5](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md)). Il ne peut pas affirmer « PDF/UA conforme ».
2. **Sidecar veraPDF (repli nécessaire) — validation PDF/UA et preuves détaillées.** lopdf n'a aucun modèle de conformité PDF/UA (recherche de code à 0 résultat) : c'est le cas « bibliothèque Rust nettement moins bonne » de [P-00 §2](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md). veraPDF apporte des règles formalisées (106 règles évaluées sur un échantillon du corpus), le rapport clause par clause et les deux profils `ua1`/`ua2`. Sa double licence GPLv3+/MPLv2+ ([README](https://github.com/veraPDF/veraPDF-library)) et son exécutable Java imposent le sidecar : aucun lien dans nos binaires, isolement réel, timeout ([P-00 §2](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md)).
3. **Intégration.** Manifeste `plugins/verapdf/sidecar.toml` (mode `persistant`, un processus par audit, `timeout_ms` ≥ 60 000 — un PDF de 186 pages a pris 22,4 s, hors démarrage JVM —, `binaires_requis` `java --version`, `licence = "GPL-3.0-or-MPL-2.0"`) ; adaptateur traduisant `initialiser`/`analyser` en invocation CLI avec fichiers passés par **chemin** et rapport JSON renvoyé en `preuves`, sans champ `status`/`verdict` ; empreinte SHA-256 de l'exécutable affichée par `rgaa plugins list` ([P-00 §7 bis](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md)). Le profil (`ua1` ou `ua2`) est une option enregistrée dans `version_modele` aux côtés de la version veraPDF.
4. **Couverture des tests.** Déclarer `tests = ["13.3.1"]` et mapper les clauses EN 301 549 utiles : 10.1.1.1 (alternatives), 10.1.3.1/10.1.3.2 (structure et ordre), 10.2.4.2 (titre), 10.3.1.1/10.3.1.2 (langue), 10.4.1.2 ([EN 301 549](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf)). Le jugement final reste humain : la méthodologie RGAA 13.3 demande PAC et ne retient que les erreurs de sa liste ([critères et tests](https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/#13.3)).
5. **Dégradation.** Sidecar absent, timeout ou JSON invalide → tests concernés en `review` « preuve manquante (sidecar verapdf : raison) », audit poursuivi avec les preuves natives ([P-00 §7 bis](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md)).

**Ce qu'on ne promet pas :** « audit PDF/UA automatique », « équivalent PAC », ni verdict RGAA automatique sur un document téléchargé. On promet des preuves mesurées, datées, reproductibles et clairement attribuées à leur variante (native ou sidecar).

## 7. Sources

Consultées le 2026-11-21 ; primary sources uniquement.

1. Ticket [#443](https://github.com/jamon8888/Holo-RGAA/issues/443) et carte [#441](https://github.com/jamon8888/Holo-RGAA/issues/441).
2. [P-00 — Contrat de plugin d'analyse, branche `116zoo/business-rgaa`](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/specs/plugins-analyse/00-contrat-plugin.md) ; [manques du socle auditeur, M2](https://github.com/jamon8888/Holo-RGAA/blob/116zoo/business-rgaa/docs/strategy/manques-socle-auditeur.md).
3. RGAA : [critères et tests, #13.3](https://accessibilite.numerique.gouv.fr/methode/criteres-et-tests/#13.3) ; [RGAA v4.1.2 (PDF)](https://accessibilite.numerique.gouv.fr/doc/RGAA-v4.1.2.pdf).
4. ETSI : [EN 301 549 V3.2.1 (2021-03), chapitre 10](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf).
5. ISO : [14289-1:2014 (PDF/UA-1)](https://www.iso.org/standard/64599.html) ; [14289-2:2024 (PDF/UA-2)](https://www.iso.org/standard/82278.html) — page directe 403 le 2026-11-21, référence relevée dans l'extrait officiel du moteur de recherche.
6. veraPDF : [site](https://verapdf.org/) ; [dépôt `veraPDF-library`](https://github.com/veraPDF/veraPDF-library) ; [installation](https://docs.verapdf.org/install/) ; [CLI](https://docs.verapdf.org/cli/) ; [validation](https://docs.verapdf.org/validation/) ; [règles PDF/UA-1](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-1-rules) et [PDF/UA-2](https://github.com/veraPDF/veraPDF-validation-profiles/wiki/PDFUA-Part-2-rules) ; [corpus de test](https://github.com/veraPDF/veraPDF-corpus) ; [protocole Matterhorn](https://pdfa.org/resource/the-matterhorn-protocol/).
7. lopdf : [dépôt `J-F-Liu/lopdf`](https://github.com/J-F-Liu/lopdf) ; [docs.rs 0.39.0](https://docs.rs/lopdf/0.39.0/lopdf/) ; [PDF 1.7 Reference (Adobe)](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf).

## Annexe — composition du lot de 50 PDF

Deux PDF institutionnels : [RGAA v4.1.2](https://accessibilite.numerique.gouv.fr/doc/RGAA-v4.1.2.pdf), [ETSI EN 301 549 V3.2.1](https://www.etsi.org/deliver/etsi_en/301500_301599/301549/03.02.01_60/en_301549v030201p.pdf). Les 48 autres proviennent du [corpus veraPDF](https://github.com/veraPDF/veraPDF-corpus) (`master`), chemins relatifs :

<details>
<summary>48 chemins du corpus</summary>

- `PDF_UA-1/7.18 Annotations/7.18.5 Links/7.18.5-t02-fail-a.pdf`
- `PDF_UA-1/7.18 Annotations/7.18.3 Tab order/7-18.3-t01-pass-a.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.5 Fonts/8.4.5.5 Embedding/8.4.5.5.1 General/8.4.5.5.1-t01-fail-a.pdf`
- `PDF_UA-1/7.21 Fonts/7.21.7 Unicode character maps/7.21.7-t02-fail-b.pdf`
- `PDF_UA-1/7.2 Text/7.2-t32-fail-a.pdf`
- `PDF_UA-2/8.2 Logical structure/8.2.2 Real content/8.2.2-t01-pass-b.pdf`
- `PDF_UA-1/5 Version identification/5-t05-pass-a.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.5 Fonts/8.4.5.3 Composite fonts/8.4.5.3.1 General/8.4.5.3.1-t01-pass-b.pdf`
- `PDF_UA-1/7.5 Tables/7.5-t01-pass-a.pdf`
- `PDF_UA-1/7.5 Tables/7.5-t01-pass-d.pdf`
- `PDF_UA-1/7.5 Tables/7.5-t01-pass-b.pdf`
- `PDF_UA-1/7.2 Text/7.2-t23-pass-c.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.4 Declaring natural language/8.4.4-t02-pass-f.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.5 Fonts/8.4.5.3 Composite fonts/8.4.5.3.1 General/8.4.5.3.1-t01-fail-b.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.4 Declaring natural language/8.4.4-t02-fail-b.pdf`
- `PDF_UA-1/7.21 Fonts/7.21.3 Composite fonts/7.21.3.1 General/7.21.3.1-t01-fail-b.pdf`
- `PDF_UA-1/7.2 Text/7.2-t29-pass-c.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.4 Declaring natural language/8.4.4-t02-fail-o.pdf`
- `PDF_UA-1/7.1 General/7.1-t07-fail-a.pdf`
- `PDF_UA-1/7.2 Text/7.2-t34-pass-d.pdf`
- `PDF_UA-1/7.20 XObjects/7.20-t01-pass-a.pdf`
- `PDF_UA-1/7.2 Text/7.2-t29-pass-i.pdf`
- `PDF_UA-1/7.2 Text/7.2-t17-pass-c.pdf`
- `PDF_UA-2/8.9 Annotations/8.9.2 Semantics and content/8.9.2.4 Annotation types/8.9.2.4.10 File attachment/8.9.2.4.10-t01-fail-a.pdf`
- `PDF_UA-1/7.1 General/7.1-t03-pass-a.pdf`
- `PDF_UA-1/5 Version identification/5-t02-fail-a.pdf`
- `PDF_UA-1/7.2 Text/7.2-t03-fail-c.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.5 Fonts/8.4.5.7 Character encodings/8.4.5.7-t02-pass-d.pdf`
- `PDF_UA-1/7.1 General/7.1-t06-pass-a.pdf`
- `PDF_UA-2/8.10 Forms/8.10.2 Context/8.10.2.3 Contents entry/8.10.2.3-t01-fail-a.pdf`
- `PDF_UA-1/7.2 Text/7.2-t29-pass-h.pdf`
- `PDF_UA-1/7.2 Text/7.2-t17-fail-a.pdf`
- `PDF_UA-1/7.2 Text/7.2-t33-fail-a.pdf`
- `PDF_UA-2/8.2 Logical structure/8.2.5 Additional requirements for specific structure types/8.2.5.26 Table (Table, TR, TH, TD, THead, TBody, TFoot)/8.2.5.26-t04-fail-a.pdf`
- `PDF_UA-1/7.4 Headings/7.4.2 Numbered headings/7.4.2-t01-pass-d.pdf`
- `PDF_UA-1/7.2 Text/7.2-t37-fail-a.pdf`
- `PDF_UA-1/7.21 Fonts/7.21.3 Composite fonts/7.21.3.3 CMaps/7.21.3.3-t01-fail-a.pdf`
- `PDF_UA-1/7.21 Fonts/7.21.6 Character encodings/7.21.6-t02-fail-d.pdf`
- `PDF_UA-1/7.21 Fonts/7.21.3 Composite fonts/7.21.3.1 General/7.21.3.1-t01-fail-a.pdf`
- `PDF_UA-1/7.2 Text/7.2-t21-pass-c.pdf`
- `PDF_UA-1/7.18 Annotations/7.18.1 General/7.18.1-t03-pass-f.pdf`
- `PDF_UA-2/8.2 Logical structure/8.2.4 Structure types/8.2.4-t01-pass-a.pdf`
- `PDF_UA-1/7.18 Annotations/7.18.2 Annotation types/7.18.2-t01-fail-a.pdf`
- `PDF_UA-2/8.2 Logical structure/8.2.5 Additional requirements for specific structure types/8.2.5.28 Figure/8.2.5.28.2 Figure properties/8.2.5.28.2-t01-pass-b.pdf`
- `PDF_UA-1/7.2 Text/7.2-t29-pass-b.pdf`
- `PDF_UA-1/7.18 Annotations/7.18.1 General/7.18.1-t02-pass-d.pdf`
- `PDF_UA-2/8.4 Text representation for content/8.4.5 Fonts/8.4.5.7 Character encodings/8.4.5.7-t02-pass-c.pdf`
- `PDF_UA-1/7.21 Fonts/7.21.7 Unicode character maps/7.21.7-t02-pass-a.pdf`

</details>
