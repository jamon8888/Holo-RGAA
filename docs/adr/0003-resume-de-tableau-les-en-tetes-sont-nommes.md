# 0003 — Pour ce projet, un résumé de tableau nomme aussi les en-têtes

- Statut : accepté, précise l'ADR 0002 (la valeur des cellules reste hors résumé)
- Date : 2026-10-29
- Contexte d'origine : lot T (36 cartes, test 5.2.1)

## Contexte

L'ADR 0002 a retenu la lecture du glossaire RGAA 4.1.2 : un résumé donne la nature et la
structure du tableau. Le lot T devait mesurer l'application de cette règle écrite, avec 18
résumés attendus pertinents sur 36.

L'étiqueteur n'en a retenu que **7**. Le repère trivial (tout classer « non pertinent ») monte à
81 %, au-dessus du plafond de 60 % du protocole : le lot ne mesure rien. Les 7 cartes retenues sont
exactement celles qui **nomment les en-têtes** (Nord et Sud, Ardèche et Gers, Jardin et Cuisine,
Mercier, Vasseur et Blanc, Lyon et Rouen, du mardi au dimanche), et les 11 refus d'un résumé
pourtant correct en nature et en structure portent le même motif (« il manque le nom des
catégories jardin et cuisine »). Le test-retest est de 13/14 : le critère est stable.

## Décision

Pour la calibration de ce projet, le critère de l'étiqueteur est retenu, bien qu'il soit plus
strict que le glossaire. Un résumé est pertinent s'il donne la nature et la structure du tableau
**et nomme ses en-têtes** :

- il cite chaque libellé d'en-tête qui n'est pas un simple numéro d'ordre (catégories, sites,
  candidats, années, jours, prévu/réalisé, villes, gares) ;
- les libellés ordinaux (T1 à T4, Bureau 1 à 5) peuvent n'être que comptés ;
- le dernier niveau de lignes imbriqué dans un regroupement nommé (communes d'un département,
  produits d'une catégorie) peut n'être que compté ;
- il ne reprend toujours pas les valeurs des cellules (ADR 0002).

## Conséquences

- La règle a été ajustée sur les 36 étiquettes du lot T : l'accord sur ces cartes ne prouve rien.
  Le lot W (`tools/calibration/lots/resume_tableau_noms.py`) est le test hors échantillon : 24
  cartes neuves, quatre tableaux jamais montrés, 12 pertinentes pour 12 non pertinentes.
- Un test 5.2.1 « Assisté calibré » l'est donc contre ce critère strict. Un résumé conforme au
  glossaire mais sans les noms sera jugé non pertinent ; c'est un choix de projet, pas une
  exigence du RGAA, et les rapports ne doivent pas le présenter comme tel.
- L'ADR 0002 reste valable pour tout ce qui concerne les valeurs de cellules.
