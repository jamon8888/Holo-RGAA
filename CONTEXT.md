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
