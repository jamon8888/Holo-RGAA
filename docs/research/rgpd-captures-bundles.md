# Obligations RGPD des captures et bundles d'audit

Ticket : #404 (carte parente #401). Contexte : PRD `docs/strategy/PRD-holo-rgaa-europe.md`
(branche `116zoo/business-rgaa`) : NF-2 (« Le SaaS est sous-traitant : DPA signé et durée de
conservation par défaut fixée avant tout pilote payant (proposition : 90 jours pour les captures
[hypothèse]) ; registre »), D1 (offre Souveraine / offre Confort), D4 et F-ACC-1 (instance dédiée
par client en M1).

**Nature de ce document.** Il rapporte ce que disent les textes et les autorités (RGPD, CNIL, CEPD/EDPB,
Commission, Tribunal de l'UE). Ce n'est pas un avis juridique. Les rapprochements avec le produit sont
signalés comme tels (« Lecture produit ») et restent à valider (dernière section).

Toutes les sources ont été consultées le **2026-11-20**. Le texte du RGPD est cité d'après la version
française publiée par la CNIL (`https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitreN`),
EUR-Lex refusant les requêtes automatisées ce jour-là.

---

## (a) Le SaaS est-il sous-traitant (art. 28) ? Contenu obligatoire du DPA

### Textes

- **Art. 4.7 RGPD** — responsable du traitement : l'organisme « qui, seul ou conjointement avec d'autres,
  détermine les finalités et les moyens du traitement ».
- **Art. 4.8 RGPD** — sous-traitant : l'organisme « qui traite des données à caractère personnel pour le
  compte du responsable du traitement ».
- **Art. 4.2 RGPD** — le « traitement » couvre « toute opération […] effectuées ou non à l'aide de
  procédés automatisés », dont la conservation.
- **Art. 28.10 RGPD** — « si, en violation du présent règlement, un sous-traitant détermine les finalités
  et les moyens du traitement, il est considéré comme un responsable du traitement pour ce qui concerne ce
  traitement. »

Source : https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre1 et `.../chapitre4`.

- **CEPD, Lignes directrices 07/2020 sur les notions de responsable du traitement et de sous-traitant**,
  version 2.1 (adoptée le 7 juillet 2021, corrections mineures le 20 septembre 2022) :
  https://www.edpb.europa.eu/system/files/documents/2023-10/EDPB_guidelines_202007_controllerprocessor_final_en.pdf
  - § 76 : « Two basic conditions for qualifying as processor are: a) being a separate entity in relation
    to the controller and b) processing personal data on the controller's behalf. »
  - Résumé exécutif : « some more practical aspects of implementation (“non-essential means”) can be left
    to the processor ».
  - Exemple « Hosting services » : un hébergeur qui stocke des données sans les traiter autrement « is
    processing personal data on employer A's behalf and is therefore a processor », et « a data processing
    agreement according to Article 28 must be concluded ».
  - Exemple « standardised cloud storage service » : même avec un contrat standard « take it or leave it »,
    « Insofar as the cloud service provider does not process the personal data for its own purposes and
    stores the data solely on behalf of its customers and in accordance with instructions, the service
    provider will be considered as a processor. »
  - § 117 : au-delà des instructions, le sous-traitant « will even be considered a controller in respect of
    that processing in accordance with Article 28(10) ».

**Lecture produit (à valider).** Le schéma décrit par le CEPD (stockage pour le compte du client, selon
ses instructions, sans finalité propre) correspond à ce que NF-2 suppose. Toute réutilisation des captures
par l'éditeur pour ses propres fins (amélioration d'un modèle, calibration, jeux de test, statistiques
produit) relèverait de l'art. 28.10 / § 117 pour ce traitement-là. À noter que le client lui-même peut
être soit l'exploitant du site audité, soit un prestataire (agence) auditant le site d'un tiers : la
chaîne responsable / sous-traitant / sous-traitant ultérieur diffère alors, et ce cas n'est pas traité
ici (voir dernière section).

### Contenu obligatoire du contrat (art. 28.3)

Le contrat « définit l'objet et la durée du traitement, la nature et la finalité du traitement, le type de
données à caractère personnel et les catégories de personnes concernées, et les obligations et les droits
du responsable du traitement ». Il prévoit que le sous-traitant :

- a) ne traite les données « que sur instruction documentée du responsable du traitement, y compris en ce
  qui concerne les transferts […] vers un pays tiers » ;
- b) veille à la confidentialité des personnes autorisées ;
- c) « prend toutes les mesures requises en vertu de l'article 32 » (sécurité) ;
- d) respecte les conditions des § 2 et 4 pour recruter un autre sous-traitant ;
- e) aide le responsable à répondre aux demandes d'exercice des droits (chapitre III) ;
- f) aide le responsable à respecter les art. 32 à 36 (sécurité, violations, AIPD, consultation préalable) ;
- g) « selon le choix du responsable du traitement, supprime toutes les données à caractère personnel ou
  les renvoie au responsable du traitement au terme de la prestation de services […] et détruit les copies
  existantes » ;
- h) met à disposition les informations nécessaires et permet les audits, y compris des inspections.

Autres points de l'art. 28 : § 1 (garanties suffisantes) ; § 2 (pas de sous-traitant ultérieur sans
autorisation écrite préalable, spécifique ou générale ; en cas d'autorisation générale, information des
changements et faculté d'objection) ; § 4 (mêmes obligations imposées au sous-traitant ultérieur, le
sous-traitant initial « demeure pleinement responsable ») ; § 9 (forme écrite, y compris électronique).
Source : https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre4

**Précisions du CEPD (LD 07/2020)** :
- § 112 : le contrat « should not merely restate the provisions of the GDPR: rather, it should include more
  specific, concrete information as to how the requirements will be met and which level of security is
  required ».
- § 114 (durée) : « the exact period of time, or the criteria used to determine it, should be specified » ;
  note 52 : « The duration of the processing is not necessarily equivalent to the duration of the
  agreement ».
- § 114 (type de données) : à préciser « in the most detailed manner as possible » ; il « would not be
  adequate merely to specify that it is “personal data pursuant to Article 4(1) GDPR” ».
- § 126 : le contrat doit inclure ou référencer les mesures de sécurité, une obligation d'obtenir
  l'accord du responsable avant de les modifier, et leur revue régulière.

**Modèle disponible.** Clauses contractuelles types responsable / sous-traitant : décision d'exécution
(UE) 2021/915 de la Commission du 4 juin 2021 (art. 28.7). Selon la CNIL, elles ne sont pas obligatoires
et « ne peuvent servir aux fins du chapitre V » (transferts) — il faut alors les clauses dédiées aux
transferts. Source : https://www.cnil.fr/fr/clauses-contractuelles-types-entre-responsable-de-traitement-et-sous-traitant
(page datée du 30 juin 2021) ; ELI : https://eur-lex.europa.eu/eli/dec_impl/2021/915/oj

---

## (b) Données de tiers visibles sur les captures : base légale et obligations du responsable

### Textes

- **Art. 4.1 RGPD** : est une donnée personnelle « toute information se rapportant à une personne physique
  identifiée ou identifiable ». Le texte ne fait pas d'exception pour les données publiées en ligne ; un
  nom ou un avis client signé capturé sur une page reste une donnée personnelle.
- **Art. 6.1 RGPD** : la base la plus proche du cas est le point f) : « le traitement est nécessaire aux
  fins des intérêts légitimes poursuivis par le responsable du traitement ou par un tiers, à moins que ne
  prévalent les intérêts ou les libertés et droits fondamentaux de la personne concernée ». Le point b)
  (exécution d'un contrat « auquel la personne concernée est partie ») ne vise pas les tiers dont le nom
  apparaît sur une page. Le choix de la base appartient au responsable du traitement (le client), pas au
  sous-traitant. Source : `.../chapitre2`.
- **Art. 5.1.c** (minimisation) et **5.1.e** (limitation de la conservation, voir (c)).
- **Art. 14 RGPD** (données non collectées auprès de la personne) : obligation d'information, mais le
  § 5 b) prévoit une exception lorsque « la fourniture de telles informations se révèle impossible ou
  exigerait des efforts disproportionnés […]. En pareils cas, le responsable du traitement prend des
  mesures appropriées pour protéger les droits et libertés […], y compris en rendant les informations
  publiquement disponibles ». Source : `.../chapitre3`.

### Doctrine CNIL la plus proche

CNIL, « La base légale de l'intérêt légitime : fiche focus sur les mesures à prendre en cas de collecte
des données par moissonnage » (19 juin 2025) : https://www.cnil.fr/fr/focus-interet-legitime-collecte-par-moissonnage

**Attention au champ** : la fiche porte sur le développement de systèmes d'IA (constitution de bases
d'apprentissage) ; elle n'est pas écrite pour l'audit d'accessibilité. Elle est citée faute de doctrine
plus spécifique trouvée. Mesures listées :
- critères de collecte définis en amont ; exclusion de catégories de données par filtres ;
- suppression des données non pertinentes « immédiatement après leur collecte » ;
- données sensibles collectées de façon incidente : non illégal en soi, mais « suppression immédiate et
  automatisée » dès qu'elles sont connues ;
- prise en compte des attentes raisonnables des personnes ;
- garanties supplémentaires : limiter aux données accessibles sans compte, information large,
  pseudonymisation juste après la collecte.

**Lecture produit (à valider).** Ces mesures pointent vers des fonctions que le SaaS pourrait offrir au
responsable : masquage / floutage des zones de contenu utilisateur avant stockage, minimisation des
extraits DOM aux nœuds nécessaires à la preuve, purge automatique. Les formulaires remplis par l'audit
avec des données fictives ne posent pas ce problème ; les audits authentifiés (espaces clients) exposent
en revanche des données de comptes réels et changent l'analyse.

---

## (c) Durée de conservation

### Textes

- **Art. 5.1.e RGPD** : données « conservées sous une forme permettant l'identification des personnes
  concernées pendant une durée n'excédant pas celle nécessaire au regard des finalités pour lesquelles
  elles sont traitées » (exceptions : archivage dans l'intérêt public, recherche, statistiques, art. 89).
- **Art. 5.2** : le responsable « est en mesure de démontrer » le respect du § 1 (responsabilité).
- **Art. 28.3 g** : suppression ou restitution au terme de la prestation, au choix du responsable.
- **Art. 30.1 f** : le registre mentionne « dans la mesure du possible, les délais prévus pour l'effacement
  des différentes catégories de données ».

### CNIL

« Les durées de conservation des données » (page datée du 2 avril 2026) :
https://www.cnil.fr/fr/les-durees-de-conservation-des-donnees
- La durée est fixée par le responsable de traitement, en fonction de la finalité ; si un texte impose
  une durée, elle s'applique ; sinon « il appartient alors au responsable du fichier de la déterminer en
  fonction de la finalité du traitement ».
- Cycle en trois phases : base active, archivage intermédiaire (accès restreint, séparé de la base active,
  par exemple pour un contentieux ou une obligation légale), archivage définitif.
- Questions à se poser : jusqu'à quand les données sont nécessaires à l'objectif ; obligation légale de
  conservation ; besoin de preuve en cas de contentieux et délai de recours.
- Outils : guide pratique CNIL / Service interministériel des archives de France ; référentiels sectoriels
  (santé, recherche en santé, social et médico-social, ressources humaines). Les durées « recommandées »
  de ces référentiels valent présomption de conformité.

**Constat** : aucun référentiel CNIL trouvé ne fixe de durée pour des captures d'écran ou des preuves
d'audit d'accessibilité. **Aucune source primaire consultée ne fournit ni ne valide la valeur de 90 jours**
du PRD ; elle reste une hypothèse. La méthode CNIL suggère de la dériver de la finalité (par exemple :
durée de la campagne d'audit et de la correction, puis éventuelle conservation probatoire en archivage
intermédiaire si le client doit pouvoir justifier une déclaration d'accessibilité), et de documenter le
choix (art. 5.2). Ce lien entre preuve d'accessibilité et durée n'a pas été vérifié dans un texte.

Rappel CEPD (LD 07/2020, note 52) : la durée du traitement peut différer de celle du contrat ; le DPA doit
l'indiquer, ou en donner les critères.

---

## (d) Transferts hors UE : captures lues par l'agent d'un éditeur américain (offre Confort)

### Textes

- **Art. 44 RGPD** : un transfert vers un pays tiers « ne peut avoir lieu que si […] les conditions définies
  dans le présent chapitre sont respectées par le responsable du traitement et le sous-traitant, y compris
  pour les transferts ultérieurs ». Source : `.../chapitre5`.
- **Art. 45** : transfert sur la base d'une décision d'adéquation ; **art. 46** : à défaut, garanties
  appropriées (dont clauses contractuelles types de transfert, décision d'exécution (UE) 2021/914).
- **Art. 28.3 a** : le sous-traitant ne transfère que sur instruction documentée du responsable.
- **Art. 30.1 e / 30.2 c** : les transferts figurent aux registres.

### EU-US Data Privacy Framework — état au 2026-11-20

- Décision d'adéquation (UE) 2023/1795 adoptée le **10 juillet 2023** ; elle couvre les transferts vers les
  organisations américaines **qui participent au DPF** (auto-certification), pas les États-Unis en général.
  Source : https://commission.europa.eu/law/law-topic/data-protection/international-dimension-data-protection/eu-us-data-transfers_en
- **Premier réexamen périodique** : rapport COM(2024) 451 final du **9 octobre 2024** — « the U.S. authorities
  have put in place the necessary structures and procedures to ensure that the Data Privacy Framework
  functions effectively » ; suivi annoncé de la DPRC, de la section 702 FISA et des nominations au PCLOB ;
  « the Commission considers it appropriate to carry out the next periodic review after three years ».
  Source : https://commission.europa.eu/document/25695177-8073-4ce3-bf81-eb816dc6b468_en (PDF via
  https://data.consilium.europa.eu/doc/document/ST-14403-2024-INIT/en/pdf).
- **Tribunal de l'UE, 3 septembre 2025, T-553/23, Latombe c/ Commission** : recours en annulation **rejeté** ;
  le Tribunal confirme qu'à la date d'adoption les États-Unis assuraient un niveau de protection adéquat
  et relève que la Commission « is required to monitor continuously » le cadre et peut suspendre, modifier
  ou abroger la décision. Source : communiqué CURIA n° 106/25,
  https://curia.europa.eu/jcms/upload/docs/application/pdf/2025-09/cp250106en.pdf
- **Pourvoi** : un pourvoi de M. Latombe devant la Cour de justice (affaire **C-703/25 P**, introduit le
  31 octobre 2025, publié au JO C/2025/6610) est rapporté par des sources secondaires ; **non vérifié sur
  source primaire** (EUR-Lex et InfoCuria inaccessibles en automatique ce jour-là). Son issue n'est pas
  connue de ce document.

**Lecture produit (à valider).** Dans l'offre Confort, si un agent d'un éditeur américain (Claude for
Chrome, Codex for Chrome, computer use) lit des captures contenant des données personnelles, il y a
vraisemblablement transfert (ou accès depuis un pays tiers) relevant du chapitre V. Deux questions de fait
restent ouvertes : (1) qui déclenche ce flux — le client utilisant son propre compte chez l'éditeur, ou le
SaaS (auquel cas l'éditeur serait un sous-traitant ultérieur, art. 28.2 et 28.4) ; (2) la participation
active au DPF de l'entité qui reçoit les données, à vérifier sur https://www.dataprivacyframework.gov/list
(liste illisible par l'outil de consultation ce jour). La fiche F-SEC-5 du PRD (flux par client agent) est
l'endroit naturel pour consigner ces réponses. L'offre Souveraine, sans agent tiers, n'est pas concernée.

---

## (e) Registre et AIPD

### Registre (art. 30)

- **Responsable** (client) — § 1 : nom et coordonnées, finalités, catégories de personnes et de données,
  destinataires, transferts, délais d'effacement, mesures de sécurité.
- **Sous-traitant** (SaaS) — § 2 : nom et coordonnées du sous-traitant et de chaque responsable pour le
  compte duquel il agit, catégories de traitements effectués pour chacun, transferts hors UE, description
  générale des mesures de sécurité.
- **§ 5 (exemption < 250 salariés)** : elle ne joue pas si le traitement « est susceptible de comporter un
  risque pour les droits et des libertés », « s'il n'est pas occasionnel » ou s'il porte sur des données
  des art. 9 ou 10. **Lecture produit** : un stockage continu de captures pour des clients n'apparaît pas
  occasionnel — à confirmer.

Source : https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre4

### AIPD (art. 35)

- **§ 1** : AIPD requise par le responsable du traitement lorsqu'un traitement « est susceptible
  d'engendrer un risque élevé pour les droits et libertés des personnes physiques ». L'AIPD est une
  obligation du **responsable** ; le sous-traitant l'aide (art. 28.3 f).
- **§ 3** : cas où elle est en particulier requise — évaluation systématique avec effets juridiques
  (profilage), traitement à grande échelle de données sensibles ou pénales, surveillance systématique à
  grande échelle d'une zone publique.
- **Liste CNIL des traitements soumis à AIPD** (délibération n° 2018-327 du 11 octobre 2018) :
  https://www.cnil.fr/sites/default/files/atoms/files/liste-traitements-avec-aipd-requise-v2.pdf —
  14 types (santé, génétique, profilage RH, surveillance des salariés, alertes, profilage avec exclusion
  de contrat, biométrie, logement social, accompagnement social, localisation à large échelle, etc.).
  **Aucun type de la liste ne correspond à des captures de pages web pour un audit d'accessibilité.**
- **Règle des neuf critères** (lignes directrices G29 WP248, reprises par la CNIL :
  https://www.cnil.fr/fr/ce-quil-faut-savoir-sur-lanalyse-dimpact-relative-la-protection-des-donnees-aipd,
  page du 18 octobre 2017) : AIPD nécessaire si le traitement remplit « au moins deux des neuf critères » :
  évaluation/scoring ; décision automatique avec effet légal ; surveillance systématique ; données sensibles
  ou hautement personnelles ; large échelle ; croisement de données ; personnes vulnérables ; usage
  innovant ; exclusion du bénéfice d'un droit/contrat.

**Lecture produit (à valider).** Pour un audit de pages publiques, les critères qui pourraient être
discutés sont « large échelle » (nombre de sites et de captures) et « usage innovant » (lecture par agents
IA, offre Confort) ; la présence de données sensibles dépend des sites audités (santé, social, espaces
authentifiés). Cela ne permet pas de conclure ici ; c'est au responsable de trancher, avec l'aide du SaaS.

---

## Non trouvé / à faire valider par un juriste

1. **Durée de 90 jours** : aucune source primaire ne la fournit ni ne la valide. Pas de référentiel CNIL
   sur les preuves d'audit ou les captures d'écran. À fixer et documenter par finalité (méthode CNIL).
2. **Lien entre conservation des preuves et obligations d'accessibilité** (déclaration d'accessibilité,
   contrôles, contentieux) : non recherché ici ; il pourrait justifier une phase d'archivage intermédiaire
   plus longue que la base active.
3. **Rôle exact selon le client** : exploitant du site audité vs agence auditant le site d'un tiers
   (chaîne responsable / sous-traitant / sous-traitant ultérieur ; éventuelle responsabilité conjointe).
4. **Réutilisation par l'éditeur** (calibration, corpus de tests, amélioration de modèle à partir des
   captures clients) : basculerait l'éditeur en responsable pour ce traitement (art. 28.10) ; à exclure
   ou à encadrer séparément.
5. **Base légale du client pour les données de tiers** : l'intérêt légitime (art. 6.1 f) est l'hypothèse
   naturelle, mais la mise en balance n'est pas faite ; la fiche CNIL moissonnage est hors champ (IA).
6. **Offre Confort** : qualification du flux vers l'éditeur de l'agent (transfert du client ou du SaaS),
   statut DPF de chaque éditeur, besoin de CCT 2021/914 en complément — non vérifiés.
7. **Pourvoi C-703/25 P** : existence et état non vérifiés sur source primaire ; à suivre, de même que le
   prochain réexamen DPF (environ trois ans après octobre 2024).
8. **AIPD** : nécessité non tranchée (ni liste CNIL ni deux critères établis) ; dépend des secteurs audités
   et du volume.
9. **Instance dédiée (D4)** : réduit le risque de mélange entre clients mais ne change pas, en soi, la
   qualification de sous-traitant ni les obligations ci-dessus ; non examiné plus avant.
10. **Loi Informatique et Libertés** et doctrine d'autres autorités (hors CNIL) : non consultées.
