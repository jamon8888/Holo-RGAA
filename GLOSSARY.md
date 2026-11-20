# Holo-RGAA

Plateforme d'audit d'accessibilité RGAA 4.1.2 : elle évalue des pages web critère par critère et produit un rapport de conformité.

## Language

### Automatisation des tests

**Test RGAA** :
Une vérification numérotée d'un critère RGAA (ex. 11.8.3). C'est l'unité à laquelle on attribue un niveau d'automatisation, car un même critère mélange souvent des tests de niveaux différents.
_Avoid_ : sous-critère, check

**Niveau d'automatisation** :
Ce qu'une machine peut apporter à un test RGAA. Il prend l'une de trois valeurs : Automatique, Assisté ou Irréductible.
_Avoid_ : automatisabilité, degré de couverture

**Automatique** :
Le niveau d'un test dont la machine rend seule un verdict `fail` ou `pass`, déterministe et accompagné d'une preuve.
_Avoid_ : automatisé, full auto

**Assisté** :
Le niveau d'un test dont la machine collecte la preuve et propose un indice ou un verdict (y compris d'un modèle), sans jamais conclure `pass` sans revue. Le statut rendu est `review`.
_Avoid_ : semi-automatique, aidé

**Irréductible** :
Le niveau d'un test pour lequel rien d'utile ne s'automatise au-delà de l'inventaire. Il est accompagné d'une justification écrite.
_Avoid_ : humain par conception, manuel, impossible

**Famille de preuve** :
Ensemble des tests RGAA qui posent la même question de pertinence sur le même type de preuve, et que la machine traite donc de la même façon : alternative d'image, description détaillée, transcription et sous-titres, audiodescription, résumé et libellé. C'est l'unité à laquelle on calibre.
_Avoid_ : groupe, thème, catégorie

**Calibration** :
Mesure de l'accord d'un modèle avec un étiquetage humain sur une famille de preuve, avec ancres, repère trivial et intervalle de confiance. Un test Assisté est **calibré** quand sa famille l'est, **non calibré** sinon. Le niveau Assisté ne dépend pas de la calibration ; ce qu'on peut affirmer de sa fiabilité en dépend.
_Avoid_ : validation, évaluation, benchmark

### Alternatives textuelles

**Alternative pertinente** :
Alternative textuelle d'une image informative qui est cohérente et complète à l'égard de deux sources à la fois : ce que l'image montre, et ce que la page dit de l'image. Il lui manque l'une ou l'autre et elle n'est pas pertinente.
_Avoid_ : alternative correcte, bonne alternative, alternative de qualité

**Image de données** :
Image dont l'information tient dans des grandeurs et leur comparaison : histogramme, courbe, camembert, tableau de chiffres, organigramme, frise chronologique, plan coté.
_Avoid_ : graphique, infographie, visualisation

**Complétude** :
Restitution de toutes les grandeurs qu'une image de données porte, et non de la seule tête de classement ni du seul titre. Elle est due y compris quand les mêmes valeurs figurent dans le texte voisin.
_Avoid_ : résumé du graphique, tendance, alternative synthétique

**Cohérence avec le contexte** :
Reprise dans l'alternative du sujet que la page donne à l'image : légende d'une galerie, raison d'être de l'illustration, activité de l'organisation derrière un logo. Elle est due même quand ces mots figurent déjà à côté de l'image.
_Avoid_ : reprise du contexte (le terme suggérait à tort un défaut)

**Identification par le contexte** :
Fait de nommer ce que représente l'image grâce à ce que dit la page (« forêt de Brocéliande », « érable du Japon », « Logo de l'association… »). Cas particulier de la cohérence avec le contexte.
_Avoid_ : —

**Description seule** :
Défaut d'une alternative qui décrit fidèlement ce qui est visible sans reprendre ce que la page dit de l'image. Elle n'est pas pertinente, quelle que soit la qualité de la description.
_Avoid_ : alternative trop descriptive, alternative hors contexte

### Tableaux de données

**Résumé (de tableau)** :
Passage de texte associé à un tableau de données complexe, qui donne des informations sur la **nature et la structure** du tableau. Il n'a pas à restituer le contenu des cellules : celui-ci est porté par l'association des cellules à leurs en-têtes (critères 5.6 et 5.7). Définition reprise mot pour mot du glossaire RGAA 4.1.2.
_Avoid_ : description du tableau, synthèse des données, résumé des chiffres

**Tableau de données complexe** :
Tableau dont les en-têtes ne sont pas répartis uniquement sur la première ligne et/ou la première colonne, ou dont la portée d'un en-tête ne vaut pas pour toute sa ligne ou toute sa colonne. Seul un tel tableau appelle un résumé (critères 5.1 et 5.2) : des cellules fusionnées dans le corps du tableau ne suffisent pas à le rendre complexe.
_Avoid_ : grand tableau, tableau à double entrée, tableau multi-niveaux
