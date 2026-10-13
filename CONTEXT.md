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
Alternative textuelle d'une image informative qui décrit ce que l'image montre, nomme le sujet précis quand le contexte l'identifie (lieu, organisation, espèce, produit, valeurs d'un graphique) et ne reprend pas le contenu du texte voisin. Elle reste concise (test 1.3.9).
_Avoid_ : alternative correcte, bonne alternative, alternative de qualité

**Identification par le contexte** :
Fait de nommer ce que représente l'image grâce à ce que dit la page (« forêt de Brocéliande », « érable du Japon », « Logo de l'association… »). C'est la seule contribution du contexte à une alternative pertinente.
_Avoid_ : reprise du contexte

**Reprise du contexte** :
Fait de recopier dans l'alternative le contenu du texte voisin (prix, dates, durée, thème de la page). Elle rend une alternative non pertinente.
_Avoid_ : contexte pris en compte
