# Formats de dépôt des déclarations d'accessibilité — 12 pays UE (ticket #99, part of #97)

Date : 2026-09-19 · Branche : `research/formats-depots-12-pays` · Sources primaires citées par pays.
Question : pour FR, DE, ES, IT, BE, NL, LU, PT, AT, IE, SE, DK — modèle officiel de déclaration,
exigences PDF, schéma JSON d'export, téléservice de dépôt, retour d'information/contact,
schéma pluriannuel + plan d'action. Comparaison au pipeline FR
(HTML embarquable sans CSS intrusif + PDF/A-1a + JSON Ara + webhook DINUM) et écarts bloquants
pour un trio commun HTML+PDF+JSON.

## 0. Socle commun UE (s'applique aux 12 pays)

- Directive (UE) 2016/2102 (WAD) : déclaration d'accessibilité par site/app, mécanisme de retour,
  contrôle par l'organisme national, revue régulière (recommandé : annuelle).
- Décision d'exécution (UE) 2018/1523 : **modèle officiel unique** — sections obligatoires :
  engagement, périmètre, état de conformité (conforme / partiellement / non conforme),
  contenu non accessible (a. non-conformité, b. charge disproportionnée art. 5, c. hors champ),
  alternatives accessibles, date d'établissement + méthode d'évaluation, date de dernière revue,
  **retour d'information + contact**, procédure de recours. Contenu optionnel : engagement renforcé,
  **mesures correctives + calendrier**, validation formelle.
  Source : https://eur-lex.europa.eu/eli/dec_impl/2018/1523/oj/eng
- La déclaration doit être en **format accessible** (art. 4 WAD), lien depuis chaque page
  (recommandé), et « le cas échéant » en **format lisible par machine** (directive 2003/98/CE).
  Source : considérants 2018/1523 ; https://digital-strategy.ec.europa.eu/en/policies/web-accessibility
- Norme harmonisée : **EN 301 549 v3.2.1** (JO 18/08/2021, remplace v2.1.2), présomption de conformité ;
  contenu web = WCAG 2.1 AA repris verbatim ; documents non-web/logiciels = WCAG 2.0 AA via WCAG2ICT.
  Source : https://digital-strategy.ec.europa.eu/en/policies/web-accessibility-directive-standards-and-harmonisation ;
  https://www.w3.org/WAI/policies/european-union
- Monitoring national + rapport triennal à la Commission (décision 2018/1524) :
  https://digital-strategy.ec.europa.eu/en/policies/web-accessibility
- PDF : pas d'exigence « PDF/A-1a » dans la WAD. Sont **hors champ** : formats bureautiques publiés
  avant le 23/09/2018. Les PDF **dans le champ** doivent satisfaire EN 301 549 ch. 10
  (≈ WCAG via WCAG2ICT + PDF balisé, équivalent pratique **PDF/UA ISO 14289**).
  Source : WAD art. 1(4) ; EN 301 549 ; https://testparty.ai/blog/eaa-pdf-accessibility (secondaire,
  cohérent avec la norme).
- EAA (UE) 2019/882, applicable 28/06/2025 : secteur privé ; chaque État transpose
  (ex. DE BFSG, AT BaFG BGBl. I Nr. 76/2023). Hors périmètre direct du pipeline déclaration publique,
  mais aligne le privé sur EN 301 549.

## 1. FR — France (référence pipeline)

- Modèle : RGAA 4.x + **générateur Ara** (ara.numerique.gouv.fr) ; déclaration HTML publiée sur le site,
  ex. https://ara-dev.osc-secnum-fr1.scalingo.io/accessibilite (audit 100 % RGAA 4.1.2, 23/04/2025, MAJ 09/10/2025).
- HTML embarquable : Ara fournit une page déclaration consultable
  (`/declaration/xxx`) + saisie (`/audits/xxx/declaration`) ; pas de preuve trouvée d'un « embed sans CSS
  intrusif » officiel — point à confirmer côté DINUM (écart doc, pas bloquant).
- PDF : RGAA 13.3 exige version accessible des documents bureautiques ; pratique Ara = export PDF,
  objectif PDF/A-1a balisé. Exigence **nationale**, pas UE.
- JSON : export Ara (schéma DINUM). Équivalent : aucun dans les 11 autres pays (voir §3).
- Téléservice : Ara = saisie + publication + suivi ; pas de « webhook DINUM » documenté publiquement —
  à confirmer (écart doc).
- Retour/contact : email + recours via Défenseur des droits.
- Schéma pluriannuel (3 ans) + plans annuels d'action : **obligatoires** (art. 47 loi 2005-102).
  Ex. Culture 2024-2026 (https://www.culture.gouv.fr/schema-pluriannuel-de-mise-en-accessibilite-2024-2026),
  France Titres/ANTS (contact ants-accessibilite@interieur.gouv.fr), Inserm 2024-2026. **Spécificité FR.**

## 2. DE — Allemagne

- Base : BGG §12d + **BITV 2.0** (transposition WAD + modèle UE 2018/1523) ; norme EN 301 549.
  Source : https://www.gesetze-im-internet.de/bitv_2_0/BJNR184300011.html
- Modèle : **Mustererklärung** publiée par la Überwachungsstelle fédérale (BFIT-Bund) +
  Leitfäden déclaration & feedback : https://www.bfit-bund.de/DE/Downloads/downloads
- HTML exigé ? Déclaration publiée sur le site / l'app-store, format accessible ; pas de gabarit HTML
  embarquable imposé (chaque organisme publie sa page, ex. BMJ : https://www.bmjv.de/DE/service/barrierefreiheit/barrierefreiheit_node.html).
- PDF : BITV 2.0 n'impose pas PDF/A-1a ; PDF dans le champ = EN 301 549. BFIT publie ses rapports en
  « PDF barrierefrei nach BITV 2.0 ».
- JSON : **aucun schéma national** ; les rapports triennaux DE→Commission suivent 2018/1524 (pas un export par organisme).
- Téléservice : **aucun dépôt central** — publication locale + surveillance BFIT-Bund
  (fédéral) + Überwachungsstellen des Länder (ex. Bayern : bitv@bayern.de).
- Retour/contact : formulaire/contact de l'organisme + **Schlichtungsstelle BGG** (§16 BGG,
  info@schlichtungsstelle-bgg.de, www.schlichtungsstelle-bgg.de) ; procédure d'exécution Länder
  (délai 6 semaines, ex. FAU : https://sz.fau.de/barrierefreiheit).
- Schéma pluriannuel : **non** ; seules mesures correctives + calendrier (optionnel UE, §7 BITV 2.0).

## 3. ES — Espagne

- Base : **Real Decreto 1112/2018** (transposition WAD).
- Modèle : modèle UE 2018/1523 en vigueur depuis 01/11/2018 ; norme nationale **UNE 139803:2012**
  (≈ WCAG 2.0). Source : https://administracionelectronica.gob.es/dam/jcr:26caea26-c01c-45f6-ba6b-c23c8918cf8e/2019-02-Nota-tecnica-OBSAE-rd1112-2018-accesibilidad-web.pdf
- HTML : déclaration publiée sur le portail (ex. Sénat https://www.senado.es/web/accesibilidad/index.html,
  CIS https://www.cis.es/es/accesibilidad) ; pas de gabarit embarquable imposé.
- PDF : pas de PDF/A imposé ; même régime hors-champ pré-20/09/2018 (date d'entrée en vigueur RD 1112/2018).
- JSON : **aucun** ; l'**Observatorio de Accesibilidad Web** (Secretaría General de Administración Digital)
  crawl et note les portails (méthodologie UNE 2012 v2 :
  https://administracionelectronica.gob.es/PAe/accesibilidad/metodologiaUNE2012v2). C'est du monitoring
  descendant, pas un dépôt montant.
- Téléservice : **aucun dépôt** ; auto-évaluation + diagnostic en ligne communautaire.
- Retour/contact : art. 10 (communications : « Accesibilidad Web » + URL) et art. 13 (réclamation).
- Schéma pluriannuel : **non**.

## 4. IT — Italie (le plus proche du modèle FR)

- Base : loi 4/2004 (« Legge Stanca ») + Linee Guida AgID accessibilità (vigueur 10/01/2020).
- Modèle : **Allegato 1 AgID** (2 sections : UE 2018/1523 + infos nationales : CMS, date publication,
  tests usabilité, effectifs handicapés). Conformité vs **UNI CEI EN 301549 app. A** (= WCAG 2.1 AA).
  Source : https://www.agid.gov.it/it/dichiarazione-di-accessibilita ;
  https://www.agid.gov.it/sites/agid/files/2025-04/Allegato%201%20-%20Modello_di_dichiarazione_accessibilit%C3%A0.pdf
- **Téléservice OBLIGATOIRE : https://form.agid.gov.it** — seule la compilation du modèle en ligne vaut
  conformité ; email de retour avec **lien à exposer dans le footer** (sites) / fiche store (apps) ;
  revue annuelle avant le 23/09. Pré-requis : RTD nommé + email RTD publié sur IndicePA.
  Source : https://www.agid.gov.it/index.php/en/node/103016
- HTML : le lien AgID expose une déclaration hébergée/validée AgID — équivalent fonctionnel d'un embed ;
  formats exacts (HTML nu vs iframe) à vérifier sur form.agid.gov.it (accès IPA requis).
- PDF : exemples réels de déclarations PDF générées depuis le modèle (doValue 29/07/2025, Vittoria/RCamper
  10/10/2025) ; pas de mention PDF/A-1a imposée.
- JSON : **aucun export public documenté** ; mais l'AgID expose un **monitoring open data**
  (65 681 déclarations au 16/01/2025, ventilations conformes/partielles/non conformes, CSV/PNG/SVG) :
  https://monitoraggio.accessibilita.agid.gov.it (via https://www.accessiweb.it/monitoraggio-accessibilita).
  Piste d'interop : moissonner l'open data plutôt qu'un webhook.
- Retour/contact : meccanismo di feedback obligatoire dans la déclaration + **Difensore civico digitale**
  (30 jours sans réponse satisfaisante → signalement PEC protocollo@pec.agid.gov.it) :
  https://www.agid.gov.it/it/agenzia/difensore-civico-il-digitale/dichiarazioni-accessibilita
- Schéma pluriannuel : **non** ; équivalent partiel = **« Obiettivi di accessibilità » annuels**
  publiés par chaque PA (ex. ACI : https://www.aci.it/dichiarazione-di-accessibilita). À modéliser comme
  plan d'action annuel, pas triennal.

## 5. BE — Belgique

- Base : loi du 19/07/2018 (fédéral) + décrets régionaux/communautaires ; norme EN 301 549.
- Modèle : modèle UE ; exemples BOSA : https://bosa.belgium.be/en/accessibility-statement,
  https://data.gov.be/en/accessibility-statement (23 critères, 5 non-conformités EN 301 549 listées),
  https://economy.fgov.be/en/accessibility-statement.
- Particularité : rapports d'audit BOSA publiés en HTML :
  `https://reports.accessibility.belgium.be/...` (in-depth + simplified), liés depuis la déclaration
  (ex. MyGov.be : https://mygov.be/accessibility-policy, https://mygov.be/access).
- HTML : pas de gabarit embarquable imposé.
- PDF : pas de PDF/A imposé ; PDF archivés non accessibles signalés comme tels.
- JSON : **aucun**.
- Téléservice : **aucun dépôt** ; monitoring BOSA (DG Transformation digitale / Simplification).
- Retour/contact : Service Desk BOSA (02 740 79 92) + procédure de plainte BOSA + **Médiateur fédéral**
  (contact@mediateurfederal.be). Amélioration planifiée (« improvement plan ») mentionnée dans les déclarations.
- Schéma pluriannuel : **non** (amélioration continue seulement).

## 6. NL — Pays-Bas (2e plus proche : registre central)

- Base : **Tijdelijk besluit digitale toegankelijkheid overheid** (01/07/2018) ; EN 301 549 / WCAG 2.1 AA.
  Source : https://www.logius.nl/over-ons/digitale-inclusie/toegankelijkheid
- Modèle : **modèle Logius** (invulassistent) conforme 2018/1523 ; déclaration par site/app.
- **Registre central OBLIGATOIRE : https://www.toegankelijkheidsverklaring.nl/register** — chaque organisme
  y publie sa déclaration (+ label SVG, ex. `.../register/9751`). Statuts WCAG A/B/C/D via audit.
  Dashboard DigiToegankelijk. Source : https://www.logius.nl/actueel/logius-werkt-aan-toegankelijkheid-apps ;
  monitor Eerste Kamer : https://www.eerstekamer.nl/overig/20211221/monitor_toegankelijkheid_2021/document
  (3 183 déclarations au 11/2021).
- HTML : le registre expose la déclaration ; pas de snippet embed documenté (à vérifier).
- PDF : objectif WCAG 2.1 sur les PDF du site ; pas de PDF/A imposé.
- JSON : **aucun schéma public** ; le registre est moissonnable (pages HTML par déclaration) — pas d'API
  documentée ici (à vérifier).
- Téléservice : le **registre = dépôt** (le plus proche d'un téléservice après IT).
- Retour/contact : formulaire de contact de l'organisme ; monitoring Logius mandaté par BZK.
- Schéma pluriannuel : **non** (jalons de mise en conformité dans la déclaration).

## 7. LU — Luxembourg

- Base : loi du 28/05/2019 ; norme **EN 301 549 V3.2.1** + référentiel national **RAWeb v1**.
  Source : https://guichet.public.lu/en/citoyens/support/accessibilite.html
- Modèle : modèle UE enrichi RAWeb ; ex. CTIE pour Guichet.lu et MyGuichet.lu :
  https://guichet.public.lu/en/entreprises/support/accessibilite/accessibilite-guichet.html
  (charge disproportionnée invoquée pour volume documentaire + contenus éditoriaux anciens).
- Cadre de développement centralisé **Renow** (checklist + audits experts) :
  https://guichet.public.lu/en/entreprises/support/accessibilite/accessibilite-myguichet.html
- HTML : pas d'embed imposé ; déclarations hébergées sur guichet.public.lu.
- PDF : pas de PDF/A imposé ; engagement de fourniture d'une version accessible **sur demande au cas par cas**.
- JSON : **aucun**.
- Téléservice : **aucun dépôt** ; supervision = **Service information et presse (SIP)** + Médiateur
  (Ombudsperson). Volet EAA : nouvel **OSAPS** + démarches MyGuichet.lu (signalement non-conformité
  produit/service, LuxTrust/eID) depuis 28/06/2025 :
  https://guichet.public.lu/fr/citoyens/actualites/2025/juillet/04-accessibilite-produits-services.html
- Retour/contact : accessibilite@guichet.public.lu (réponse 1 mois), formats alternatifs au choix
  (écrit/oral).
- Schéma pluriannuel : **non**.

## 8. PT — Portugal (outillage le plus réutilisable)

- Base : **Decreto-Lei n.º 83/2018** (transposition WAD) ; WCAG 2.1.
- Modèle : **« Declaração de Acessibilidade e Usabilidade »** — modèle UE + volet national obligatoire :
  A. évaluations automatiques (outil + échantillon + score), B. évaluations manuelles (obligatoires),
  C. tests usabilité avec personnes handicapées (recommandés) — art. 9 DL 83/2018.
- **Générateur national : Gerador WAI-Tools PT v1.5** (consortium AMA/WAI-Tools) + suffixe d'URL
  `/acessibilidade` ; ex. Présidence : https://www.presidencia.pt/acessibilidade (76 pages, score > 9.0),
  AMT : https://www.amt-autoridade.pt/acessibilidade, APAV : https://apav.pt/acessibilidade.
- **Observatoire : https://observatorio.acessibilidade.gov.pt** (moteur QualWeb commun, outils
  AccessMonitor/AccessMonitor Plus, MyMonitor) ; échantillon = page d'accueil + pages liées (~60 pages).
  Source : AMA via https://www.w3.org/WAI/about/projects/wai-tools/session3 ;
  https://www.acessibilidade.gov.pt/acessibilidade ; https://www.acessibilidade.gov.pt/ferramentas
- HTML : pas d'embed ; déclaration hébergée sur le site.
- PDF : pas de PDF/A imposé.
- JSON : **aucun schéma d'export** ; données moissonnables via l'observatoire (scores 1-10 par page).
- Téléservice : générateur + observatoire = dépôt de fait (déclarations + rapports chargés et publics).
- Retour/contact : email de l'organisme (ex. equipa.eed@ama.pt, acessibilidade@amt-autoridade.pt) +
  dénonciation discrimination (section V du modèle).
- Schéma pluriannuel : **non** ; labels **Selo Ouro/Prata** (usabilité+accessibilité) comme horizon —
  ex. Registo Criminal (Selo Prata) : https://registocriminal.justica.gov.pt/acessibilidade.

## 9. AT — Autriche

- Base : **Web-Zugänglichkeits-Gesetz (WZG, BGBl. I Nr. 59/2019)** fédéral + lois des Länder ; WCAG 2.1 AA
  + EN 301 549 V2.1.2/V3.2.1. Ex. BMF : https://www.bmf.gv.at/public/barrierefreiheitserklaerung.html
  (audit externe 04/2020), https://findok.bmf.gv.at/findok/barrierefreiheitserklaerung (auto-évaluation 09/2025).
- EAA transposé par **BaFG (BGBl. I Nr. 76/2023)**, en vigueur 28/06/2025 :
  https://ris.bka.gv.at/Dokumente/BgblAuth/BGBLA_2023_I_76/BGBLA_2023_I_76.html
- HTML : pas d'embed imposé.
- PDF : « ältere PDFs » explicitement listés comme barrières (Findok) ; pas de PDF/A imposé.
- JSON : **aucun**.
- Téléservice : **aucun dépôt** ; monitoring fédéral. Recours : plainte **FFG** (recommandations +
  mesures), puis Behindertenanwalt/Klagsverband (BGStG).
- Schéma pluriannuel : **non** (référence NAP 2012-2020 côté ONU-CRPD, pas un schéma par organisme).

## 10. IE — Irlande

- Base : **European Union (Accessibility of Websites and Mobile Applications of Public Sector Bodies)
  Regulations 2020** ; EN 301 549 v3.2.1 / WCAG 2.1 AA.
- Modèle : modèle UE ; ex. NDA : https://nda.ie/accessibility/accessibility-statement (préparée
  08/11/2022, revue 03/11/2023, PDF et iframes YouTube listés), NSAI :
  https://www.nsai.ie/accessibility (WCAG 2.2 AA visé, audit Axe Monitor 91,39 %),
  Citizens Information : https://www.citizensinformation.ie/en/about/accessibility (revue NDA in-depth 2026).
- Monitoring : **NDA** (Centre for Excellence in Universal Design avec NSAI) :
  https://nda.ie/monitoring ; rapports annuels AccessibleEU :
  https://accessible-eu-centre.ec.europa.eu/accessibility-monitoring_en
- HTML : pas d'embed imposé.
- PDF : PDF explicitement listés comme contenus non accessibles (NDA) ; pas de PDF/A imposé.
- JSON : **aucun**.
- Téléservice : **aucun dépôt**.
- Retour/contact : **Access Officer** obligatoire par organisme (ex. AccessOfficer@nda.ie, +353 1 6080 400)
  + réclamation ; NDA In-depth Review comme recours de surveillance.
- Schéma pluriannuel : **non**.

## 11. SE — Suède

- Base : **lagen (2018:1937) om tillgänglighet till digital offentlig service (DOS-lagen)** ; EN 301 549.
- Modèle : **tillgänglighetsredogörelse** (modèle UE) ; ex. DIGG :
  https://www.digg.se/om-oss/om-webbplatsen/tillganglighetsredogorelse-digg.se/tillganglighetsredogorelse-for-webbriktlinjer
- Monitoring : **DIGG** (Myndigheten för digital förvaltning) — 1 111 sites contrôlés 2022-2024
  (1 081 simplifiés + 30 approfondis, 15 apps), rapport 31/01/2025 :
  https://www.digg.se/analys-och-uppfoljning/publikationer/publikationer/2025-01-31-overvakning-av-digital-offentlig-service-i-sverige-2022-2024
  Constat : peu de sites/apps « bons », beaucoup de déclarations manquantes.
- HTML : pas d'embed imposé.
- PDF : pas de PDF/A imposé.
- JSON : **aucun**.
- Téléservice : **aucun dépôt** ; signalement à DIGG (anmälan bristande tillgänglighet).
- Retour/contact : contact organisme + signalement DIGG.
- Schéma pluriannuel : **non**.

## 12. DK — Danemark

- Base : **webtilgængelighedsloven** ; EN 301 549.
- Modèle : modèle UE ; **générateur central was.digst.dk** (formulaire de signalement intégré en tête de
  déclaration + 3 catégories : non-conforme / charge disproportionnée / hors champ).
  Ex. https://was.digst.dk/stpk-dk, https://was.digst.dk/sikkerhedsnet-dk, https://was.digst.dk/mssb-dk
  (guide : https://digst.dk/tilsyn/webtilgaengelighed/vejledning/udfyldelse-af-tilgaengelighedserklaering).
- Monitoring : **Digitaliseringsstyrelsen** — 255 simplifiés + 23 approfondis sites + 12 apps **par an**,
  score 0-20/21-100/>100 : https://digst.dk/tilsyn/webtilgaengelighed/monitorering-og-tilsyn
- HTML : déclarations hébergées sur was.digst.dk (donc « embarquables » par lien, pas de snippet).
- PDF : pas de PDF/A imposé ; documents en cours de mise en accessibilité signalés.
- JSON : **aucun schéma public**.
- Téléservice : **was.digst.dk = dépôt de fait** (déclarations centralisées + supervision).
- Retour/contact : contact organisme (en-tête) puis **webtilsyn@digst.dk / +45 20 16 36 12** (retour sous
  10 jours ouvrés).
- Schéma pluriannuel : **non**.

## 13. Comparaison au pipeline FR et écarts bloquants (trio HTML+PDF+JSON)

| Exigence FR | Statut hors FR | Verdict |
|---|---|---|
| HTML embarquable sans CSS intrusif | Aucun État n'impose un snippet ; IT (lien footer form.agid.gov.it), NL (registre), DK (was.digst.dk), PT (générateur) exposent une URL canonique référençable | **Non bloquant** : standardiser sur « URL canonique + lien footer/store » (modèle UE) plutôt qu'un embed ; adapter le CSS au cas FR/Ara |
| PDF/A-1a balisé | Exigence **FR-only** ; UE = hors-champ pré-23/09/2018 + EN 301 549 ch.10 (≈ PDF balisé/PDF/UA) pour le reste | **Bloquant si imposé tel quel** : les 11 autres pays rejetteraient un PDF/A-1a obligatoire. Trio commun viable = **PDF balisé (PDF/UA)** ; PDF/A-1a = option FR |
| JSON type Ara DINUM | **Aucun équivalent** : IT = open data agrégé (CSV) moissonnable ; NL = registre HTML moissonnable ; PT = observatoire (scores) ; autres = rien | **Bloquant** : pas de schéma cible commun. Définir un schéma pivot Holo (champs UE 2018/1523 + scores + URL preuves) et des adaptateurs par pays (parse IT/NL/PT/DK, génération locale ailleurs) |
| Webhook DINUM | Aucun webhook/téléservice push dans les 11 pays ; les 4 « dépôts de fait » (IT form.agid.gov.it, NL registre, DK was.digst.dk, PT générateur+observatoire) sont des **portails de saisie + publication**, pas des API d'ingestion | **Bloquant si push exigé** : passer en **pull/moissonnage** (ou saisie assistée) hors FR ; négocier API pays par pays |
| Schéma pluriannuel + plan d'action | **FR-only** (art. 47 loi 2005-102). Équivalents partiels : IT obiettivi annuali, PT Selo, BE/NL/LU/DK jalons dans la déclaration, modèle UE §optionnel « mesures + calendrier » | **Bloquant si 3 ans + annuel exigé partout** : trio commun = section « mesures correctives + calendrier » du modèle UE ; module FR séparé pour schéma triennal |
| Retour d'information + contact | Couvert partout (modèle UE obligatoire), canaux variables (Access Officer IE, Schlichtungsstelle DE, FFG AT, Médiateur BE/LU, DIGG SE, webtilsyn DK, Difensore IT, RD1112 art.10/13 ES, SIP LU) | **Non bloquant** : champ structuré {canal, email/tel/formulaire, recours, délais} |
| Revue annuelle | Recommandation UE reprise partout (IT : 23/09 ; DK : supervision annuelle ; SE/DIGG cycles 3 ans) | **Non bloquant** : champ date revue + rappel |

### Recommandation trio commun minimal (sans régression FR)

1. **HTML** : gabarit modèle UE 2018/1523 (+ extensions nationales : section B manuelle PT, RTD/IPA IT,
   RAWeb LU, WZG AT, Access Officer IE) ; publication locale + enregistrement URL canonique
   (footer/store). Pas de snippet imposé hors FR.
2. **PDF** : export PDF **balisé PDF/UA** systématique ; PDF/A-1a = profil FR uniquement.
3. **JSON** : schéma pivot Holo aligné 2018/1523 (+ §optionnel mesures/calendrier) ; adaptateurs :
   FR Ara (natif), IT (form.agid.gov.it + moisson open data CSV), NL (moisson registre), DK (was.digst.dk),
   PT (générateur + observatoire QualWeb), autres (génération locale + push manuel).
4. Points à lever côté métier : confirmer « embed sans CSS intrusif » et « webhook DINUM » (non trouvés
   dans les sources publiques) ; statuer PDF/A-1a (FR-only) vs PDF/UA (commun) ; statuer schéma
   triennal (FR-only) vs mesures+calendrier UE (commun).

## Sources principales

- EUR-Lex 2018/1523 : https://eur-lex.europa.eu/eli/dec_impl/2018/1523/oj/eng
- CE politique accessibilité web : https://digital-strategy.ec.europa.eu/en/policies/web-accessibility
- CE normes/harmonisation : https://digital-strategy.ec.europa.eu/en/policies/web-accessibility-directive-standards-and-harmonisation
- W3C WAI UE : https://www.w3.org/WAI/policies/european-union
- DE BITV 2.0 : https://www.gesetze-im-internet.de/bitv_2_0/BJNR184300011.html · BFIT : https://www.bfit-bund.de/DE/Downloads/downloads
- ES RD 1112/2018 (note OBSAE) : https://administracionelectronica.gob.es/dam/jcr:26caea26-c01c-45f6-ba6b-c23c8918cf8e/2019-02-Nota-tecnica-OBSAE-rd1112-2018-accesibilidad-web.pdf
- IT AgID : https://www.agid.gov.it/it/dichiarazione-di-accessibilita · https://www.agid.gov.it/index.php/en/node/103016 · open data : https://monitoraggio.accessibilita.agid.gov.it
- BE BOSA : https://bosa.belgium.be/en/accessibility-statement · https://data.gov.be/en/accessibility-statement · rapports : https://reports.accessibility.belgium.be
- NL Logius : https://www.logius.nl/over-ons/digitale-inclusie/toegankelijkheid · registre : https://www.toegankelijkheidsverklaring.nl/register
- LU : https://guichet.public.lu/en/citoyens/support/accessibilite.html · Renow/MyGuichet pages liées · OSAPS : https://guichet.public.lu/fr/citoyens/actualites/2025/juillet/04-accessibilite-produits-services.html
- PT AMA : https://www.acessibilidade.gov.pt/acessibilidade · https://www.acessibilidade.gov.pt/ferramentas · WAI-Tools : https://www.w3.org/WAI/about/projects/wai-tools/session3 · ex. : https://www.presidencia.pt/acessibilidade
- AT WZG/BaFG : https://www.bmf.gv.at/public/barrierefreiheitserklaerung.html · https://ris.bka.gv.at/Dokumente/BgblAuth/BGBLA_2023_I_76/BGBLA_2023_I_76.html
- IE NDA : https://nda.ie/accessibility/accessibility-statement · https://nda.ie/monitoring · NSAI : https://www.nsai.ie/accessibility
- SE DIGG : https://www.digg.se/analys-och-uppfoljning/publikationer/publikationer/2025-01-31-overvakning-av-digital-offentlig-service-i-sverige-2022-2024
- DK : https://digst.dk/tilsyn/webtilgaengelighed/monitorering-og-tilsyn · https://was.digst.dk/stpk-dk · guide : https://digst.dk/tilsyn/webtilgaengelighed/vejledning/udfyldelse-af-tilgaengelighedserklaering
- FR : https://ara-dev.osc-secnum-fr1.scalingo.io/accessibilite · https://www.culture.gouv.fr/schema-pluriannuel-de-mise-en-accessibilite-2024-2026
