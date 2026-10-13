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

### Alternatives textuelles

**Alternative pertinente** :
Alternative textuelle d'une image informative qui annonce la nature du média, décrit ce que l'image montre, nomme le sujet précis quand le contexte l'identifie (lieu, organisation, espèce, produit) et, s'il s'agit d'une image de données, en restitue la série complète. Elle ne reprend pas le contenu rédactionnel du texte voisin, et reste aussi brève que cette restitution le permet (test 1.3.9, voir `docs/adr/0001-alternatives-serie-complete.md`).
_Avoid_ : alternative correcte, bonne alternative, alternative de qualité

**Image de données** :
Image dont l'information tient dans des grandeurs et leur comparaison : histogramme, courbe, camembert, tableau de chiffres, organigramme, frise chronologique.
_Avoid_ : graphique, infographie, visualisation

**Série complète** :
Restitution dans l'alternative de toutes les catégories d'une image de données et de leur valeur, dans l'ordre que l'image met en évidence. Une alternative qui ne cite que la tête du classement n'est pas pertinente, y compris quand les valeurs figurent dans le texte voisin.
_Avoid_ : résumé du graphique, tendance, alternative synthétique

**Nature du média annoncée** :
Mention, en tête de l'alternative, de ce qu'est l'image pour la personne qui l'écoute : « Photo », « Logo de… », « Organigramme », « Frise chronologique ». Elle précède la description.
_Avoid_ : préfixe, « image de »

**Identification par le contexte** :
Fait de nommer ce que représente l'image grâce à ce que dit la page (« forêt de Brocéliande », « érable du Japon », « Logo de l'association… »).
_Avoid_ : reprise du contexte

**Reprise du contexte** :
Fait de recopier dans l'alternative le contenu rédactionnel du texte voisin : thème de la page, argumentaire, prix, dates, durée. Elle rend une alternative non pertinente. Les grandeurs d'une image de données n'en relèvent pas, même puisées dans le texte voisin : voir **Série complète**.
_Avoid_ : contexte pris en compte
