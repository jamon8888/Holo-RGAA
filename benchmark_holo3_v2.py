#!/usr/bin/env python3
"""
Benchmark Holo3-35B amélioré : retry JSON, contextes DOM réels, async batch.
"""

import os
import time
import json
import asyncio
import statistics
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, asdict
from openai import AsyncOpenAI

API_KEY = "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1"
BASE_URL = "https://api.hcompany.ai/v1/"
MODEL = "holo3-1-35b-a3b"

client = AsyncOpenAI(base_url=BASE_URL, api_key=API_KEY)


@dataclass
class BenchmarkResult:
    criterion_id: str
    criterion_title: str
    prompt_tokens: int
    completion_tokens: int
    latency_seconds: float
    cost_usd: float
    verdict: str
    confidence: float
    success: bool
    error: str = ""
    retries: int = 0


IA_ASSISTE_CRITERIA = [
    {"id": "1.3", "title": "Alternative textuelle pertinente (image porteuse d'info)", "theme": "Images"},
    {"id": "1.7", "title": "Description détaillée pertinente (image)", "theme": "Images"},
    {"id": "2.2", "title": "Titre de cadre pertinent", "theme": "Cadres"},
    {"id": "4.2", "title": "Transcription/audiodescription pertinente (média temporel)", "theme": "Multimédia"},
    {"id": "4.4", "title": "Sous-titres pertinents", "theme": "Multimédia"},
    {"id": "4.6", "title": "Audiodescription pertinente", "theme": "Multimédia"},
    {"id": "4.9", "title": "Alternative pertinente (média non temporel)", "theme": "Multimédia"},
    {"id": "5.2", "title": "Résumé pertinent (tableau complexe)", "theme": "Tableaux"},
    {"id": "5.3", "title": "Contenu linéarisé compréhensible (tableau mise en forme)", "theme": "Tableaux"},
    {"id": "5.5", "title": "Titre pertinent (tableau de données)", "theme": "Tableaux"},
    {"id": "7.2", "title": "Alternative pertinente (script)", "theme": "Scripts"},
    {"id": "8.4", "title": "Code de langue pertinent", "theme": "Éléments obligatoires"},
    {"id": "8.6", "title": "Titre de page pertinent", "theme": "Éléments obligatoires"},
    {"id": "8.8", "title": "Code de langue changement pertinent", "theme": "Éléments obligatoires"},
    {"id": "9.2", "title": "Structure document cohérente", "theme": "Structuration"},
    {"id": "10.3", "title": "Information compréhensible sans CSS", "theme": "Présentation"},
    {"id": "10.10", "title": "Information par forme/taille/position implémentée pertinemment", "theme": "Présentation"},
    {"id": "11.2", "title": "Étiquette champ formulaire pertinente", "theme": "Formulaires"},
    {"id": "11.3", "title": "Étiquettes cohérentes (même fonction répétée)", "theme": "Formulaires"},
    {"id": "11.7", "title": "Légende regroupement champs pertinente", "theme": "Formulaires"},
    {"id": "11.8", "title": "Items liste choix regroupés pertinemment", "theme": "Formulaires"},
    {"id": "11.9", "title": "Intitulé bouton pertinent", "theme": "Formulaires"},
    {"id": "11.10", "title": "Contrôle saisie utilisé pertinemment", "theme": "Formulaires"},
    {"id": "12.3", "title": "Plan du site pertinent", "theme": "Navigation"},
    {"id": "12.8", "title": "Ordre de tabulation cohérent", "theme": "Navigation"},
    {"id": "13.6", "title": "Alternative pertinente (contenu cryptique)", "theme": "Consultation"},
    {"id": "3.1", "title": "Information non donnée uniquement par la couleur", "theme": "Couleurs"},
]

SYSTEM_PROMPT = """Tu es un expert accessibilité numérique RGAA 4.1.2. Évalue le critère sur la base du contexte HTML fourni.

Réponds UNIQUEMENT en JSON valide (pas de markdown, pas de texte hors JSON) :
{
  "verdict": "CONFORME" | "NON_CONFORME" | "NE_PAS_SAVOIR",
  "confidence": 0.0-1.0,
  "justification": "Explication courte (max 2 phrases)",
  "elements_manquants": ["liste", "d'éléments", "nécessaires"]
}

Règles strictes :
- CONFORME seulement si preuve évidente dans le contexte
- NON_CONFORME seulement si violation évidente  
- NE_PAS_SAVOIR par défaut si contexte insuffisant
- confidence ≥ 0.85 pour CONFORME/NON_CONFORME
- confidence < 0.85 → NE_PAS_SAVOIR
- JSON doit être parsable par json.loads()
"""

# Contextes DOM réalistes (extraits de vraies pages)
REAL_CONTEXTS = {
    "1.3": '''<main>
  <article>
    <h1>Bilan annuel 2024</h1>
    <figure>
      <img src="/stats/ventes-2024.png" alt="Graphique en barres montrant les ventes mensuelles de janvier à décembre 2024, pic en novembre à 2.4M€">
      <figcaption>Figure 1 : Évolution des ventes 2024</figcaption>
    </figure>
  </article>
</main>''',
    "1.7": '''<figure>
  <img src="/schema-reseau.png" alt="Schéma de l'architecture réseau" longdesc="/desc/schema-reseau.html">
  <figcaption>Architecture réseau du SI</figcaption>
</figure>
<!-- longdesc contient : description détaillée de 3 paragraphes des zones DMZ, LAN, serveurs, flux chiffrés -->''',
    "2.2": '''<iframe title="Paiement sécurisé - Carte bancaire" src="https://pay.stripe.com/..." sandbox="allow-scripts allow-same-origin allow-forms"></iframe>''',
    "4.2": '''<video controls>
  <source src="/video/formation.mp4" type="video/mp4">
  <track kind="captions" src="/video/formation.fr.vtt" srclang="fr" label="Français" default>
  <track kind="descriptions" src="/video/formation.ad.vtt" srclang="fr" label="Audiodescription">
</video>''',
    "4.4": '''<video>
  <track kind="captions" src="/video/news.vtt" srclang="fr">
  <!-- VTT contient : timestamps synchronisés, identification locuteurs, bruits [musique], [applaudissements] -->''',
    "4.6": '''<video>
  <track kind="descriptions" src="/video/doc.ad.vtt" srclang="fr">
  <!-- Audiodescription : décrit actions visuelles non verbales pendant pauses dialogue -->''',
    "4.9": '''<canvas id="chart" aria-label="Graphique interactif : chiffre d'affaires par région, cliquez pour filtrer" role="img"></canvas>
<script>/* Chart.js initialise le canvas avec données JSON embarquées */</script>''',
    "5.2": '''<table>
  <caption>Budget 2024 par département (en K€)</caption>
  <thead>
    <tr><th scope="col">Département</th><th scope="col">Q1</th><th scope="col">Q2</th><th scope="col">Q3</th><th scope="col">Q4</th><th scope="col">Total</th></tr>
  </thead>
  <tbody>
    <tr><th scope="row">Marketing</th><td>120</td><td>135</td><td>140</td><td>155</td><td>550</td></tr>
  </tbody>
</table>''',
    "5.3": '''<table role="presentation" aria-hidden="true">
  <tr><td><img src="logo.png" alt=""></td><td>Navigation principale</td></tr>
  <tr><td>Colonne gauche</td><td>Contenu central</td></tr>
</table>''',
    "5.5": '''<table>
  <caption>Liste des agents habilités - Mise à jour 01/2024</caption>
  <thead><tr><th scope="row">Nom</th><th scope="col">Service</th><th scope="col">Habilitation</th></tr></thead>
  <tbody>...</tbody>
</table>''',
    "7.2": '''<div role="alert" aria-live="assertive" aria-atomic="true">
  <strong>Erreur :</strong> L'adresse email "user@domain" est invalide. Format attendu : nom@domaine.tld
</div>''',
    "8.4": '''<!DOCTYPE html>
<html lang="fr-FR">''',
    "8.6": '''<head>
  <title>Demande de carte d'identité - Service-Public.fr</title>
</head>''',
    "8.8": '''<p>La page d'accueil est disponible en <a href="/en" lang="en">English</a> et <a href="/es" lang="es">Español</a>.</p>''',
    "9.2": '''<body>
  <header><h1>Ministère de l'Économie</h1></header>
  <nav aria-label="Navigation principale"><ul><li><a href="/">Accueil</a></li></ul></nav>
  <main>
    <h1>Actualités</h1>
    <article><h2>Nouvelle réforme fiscale</h2><h3>Mesures pour les PME</h3></article>
    <article><h2>Plan d'investissement</h2></article>
  </main>
  <footer><h2>Mentions légales</h2></footer>
</body>''',
    "10.3": '''<div class="container">
  <article class="content">
    <h1>Rapport annuel</h1>
    <p>Le chiffre d'affaires 2024 s'élève à 12,4 M€.</p>
    <aside class="sidebar"><h2>Documents joints</h2><ul><li><a href="/pdf/rapport.pdf">PDF complet</a></li></ul></aside>
  </article>
</div>
<!-- Sans CSS : ordre DOM = header, nav, main(h1, p, aside), footer -->''',
    "10.10": '''<fieldset>
  <legend>Type de demande</legend>
  <div class="radio-group">
    <input type="radio" id="new" name="type" value="new"><label for="new">➕ Nouvelle demande</label>
    <input type="radio" id="renew" name="type" value="renew"><label for="renew">🔄 Renouvellement</label>
    <input type="radio" id="lost" name="type" value="lost"><label for="lost">🔍 Perte/Vol</label>
  </div>
</fieldset>''',
    "11.2": '''<form>
  <div class="field">
    <label for="email">Adresse email <span aria-hidden="true">*</span></label>
    <input type="email" id="email" name="email" required autocomplete="email" aria-describedby="email-hint">
    <span id="email-hint">Format : prenom.nom@domaine.fr</span>
  </div>
</form>''',
    "11.3": '''<form>
  <fieldset><legend>Coordonnées professionnelles</legend>
    <label for="pro-email">Email professionnel</label><input type="email" id="pro-email" autocomplete="email">
    <label for="pro-tel">Téléphone professionnel</label><input type="tel" id="pro-tel" autocomplete="tel">
  </fieldset>
  <fieldset><legend>Coordonnées personnelles</legend>
    <label for="perso-email">Email personnel</label><input type="email" id="perso-email" autocomplete="email">
    <label for="perso-tel">Téléphone personnel</label><input type="tel" id="perso-tel" autocomplete="tel">
  </fieldset>
</form>''',
    "11.7": '''<fieldset>
  <legend>Informations bancaires</legend>
  <div class="field"><label for="iban">IBAN</label><input id="iban" pattern="[A-Z]{2}[0-9]{2}[A-Z0-9]{11,30}"></div>
  <div class="field"><label for="bic">BIC</label><input id="bic" pattern="[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?"></div>
</fieldset>''',
    "11.8": '''<select name="region" aria-label="Région de résidence">
  <optgroup label="Île-de-France">
    <option value="75">Paris</option>
    <option value="77">Seine-et-Marne</option>
  </optgroup>
  <optgroup label="Auvergne-Rhône-Alpes">
    <option value="69">Rhône</option>
    <option value="38">Isère</option>
  </optgroup>
</select>''',
    "11.9": '''<button type="submit" class="btn-primary">Valider et payer 45,00 €</button>''',
    "11.10": '''<div class="field error">
  <label for="password">Mot de passe</label>
  <input type="password" id="password" aria-invalid="true" aria-describedby="pwd-error" required>
  <span id="pwd-error" role="alert">Minimum 12 caractères, une majuscule, un chiffre, un symbole</span>
</div>''',
    "12.3": '''<nav aria-label="Navigation secondaire">
  <ul>
    <li><a href="/plan-du-site">Plan du site</a></li>
    <li><a href="/accessibilite">Accessibilité</a></li>
  </ul>
</nav>
<!-- Plan du site : structure hiérarchique complète, max 3 niveaux, liens vers toutes sections -->''',
    "12.8": '''<a href="#main" class="skip-link">Aller au contenu principal</a>
<header>...</header>
<nav>Menu principal</nav>
<main id="main">
  <h1>Titre page</h1>
  <article>Contenu</article>
</main>''',
    "13.6": '''<p>Statut système : <code>OK</code> <span role="img" aria-label="tout fonctionne">✅</span></p>''',
    "3.1": '''<div class="alert alert-error" role="alert">
  <span class="icon" aria-hidden="true">⚠</span>
  <span class="text">Erreur de saisie</span>
  <span class="sr-only">Erreur : champ email invalide</span>
</div>''',
}


def estimate_cost(prompt_tokens: int, completion_tokens: int) -> float:
    input_cost = (prompt_tokens / 1_000_000) * 0.25
    output_cost = (completion_tokens / 1_000_000) * 1.80
    return input_cost + output_cost


def parse_json_safe(content: str) -> Optional[Dict]:
    """Tente de parser JSON, nettoie les artefacts courants."""
    if not content:
        return None
    # Nettoyage basique
    content = content.strip()
    if content.startswith("```json"):
        content = content[7:]
    if content.endswith("```"):
        content = content[:-3]
    content = content.strip()
    try:
        return json.loads(content)
    except json.JSONDecodeError:
        # Tentative réparation simple : trouver premier { et dernier }
        start = content.find("{")
        end = content.rfind("}")
        if start >= 0 and end > start:
            try:
                return json.loads(content[start:end+1])
            except json.JSONDecodeError:
                pass
    return None


async def run_criterion_with_retry(criterion: Dict[str, str], max_retries: int = 2) -> BenchmarkResult:
    context = REAL_CONTEXTS.get(criterion["id"], "Contexte non disponible")
    prompt = f"""Critère RGAA : {criterion['id']} - {criterion['title']}

Contexte HTML :
{context}

Évalue ce critère. Réponds en JSON uniquement."""
    
    for attempt in range(max_retries + 1):
        start = time.perf_counter()
        try:
            response = await client.chat.completions.create(
                model=MODEL,
                messages=[
                    {"role": "system", "content": SYSTEM_PROMPT},
                    {"role": "user", "content": prompt}
                ],
                temperature=0.0,
                max_tokens=500,
                response_format={"type": "json_object"}
            )
            latency = time.perf_counter() - start
            
            usage = response.usage
            prompt_tokens = usage.prompt_tokens
            completion_tokens = usage.completion_tokens
            cost = estimate_cost(prompt_tokens, completion_tokens)
            
            content = response.choices[0].message.content
            parsed = parse_json_safe(content)
            
            if parsed and "verdict" in parsed:
                verdict = parsed.get("verdict", "ERROR")
                confidence = parsed.get("confidence", 0.0)
                # Validation confiance
                if verdict in ("CONFORME", "NON_CONFORME") and confidence < 0.85:
                    verdict = "NE_PAS_SAVOIR"
                return BenchmarkResult(
                    criterion_id=criterion["id"],
                    criterion_title=criterion["title"],
                    prompt_tokens=prompt_tokens,
                    completion_tokens=completion_tokens,
                    latency_seconds=latency,
                    cost_usd=cost,
                    verdict=verdict,
                    confidence=confidence,
                    success=True,
                    retries=attempt
                )
            else:
                raise ValueError(f"JSON invalide ou verdict manquant: {content[:200]}")
                
        except Exception as e:
            latency = time.perf_counter() - start
            if attempt == max_retries:
                return BenchmarkResult(
                    criterion_id=criterion["id"],
                    criterion_title=criterion["title"],
                    prompt_tokens=0,
                    completion_tokens=0,
                    latency_seconds=latency,
                    cost_usd=0.0,
                    verdict="ERROR",
                    confidence=0.0,
                    success=False,
                    error=str(e),
                    retries=attempt
                )
            await asyncio.sleep(1.0 * (attempt + 1))  # backoff
    
    return BenchmarkResult(criterion_id=criterion["id"], criterion_title=criterion["title"], 
                          prompt_tokens=0, completion_tokens=0, latency_seconds=0, cost_usd=0,
                          verdict="ERROR", confidence=0, success=False, error="Max retries")


async def main():
    print(f"=== Benchmark Holo3-35B Async + Retry ===")
    print(f"Critères: {len(IA_ASSISTE_CRITERIA)} | Modèle: {MODEL}")
    print()
    
    # Exécution séquentielle (rate limit 10 RPM = 6s intervalle)
    results = []
    semaphore = asyncio.Semaphore(1)  # 1 à la fois pour respecter rate limit
    
    async def run_with_limit(crit):
        async with semaphore:
            result = await run_criterion_with_retry(crit)
            # Rate limit: 6.5s entre requêtes
            await asyncio.sleep(6.5)
            return result
    
    for i, criterion in enumerate(IA_ASSISTE_CRITERIA, 1):
        print(f"[{i}/{len(IA_ASSISTE_CRITERIA)}] {criterion['id']} - {criterion['title'][:55]}...")
        result = await run_with_limit(criterion)
        results.append(result)
        
        if result.success:
            print(f"  ✓ {result.latency_seconds:.2f}s | {result.prompt_tokens}+{result.completion_tokens} tok | ${result.cost_usd:.6f} | {result.verdict} (conf: {result.confidence:.2f}) [retries: {result.retries}]")
        else:
            print(f"  ✗ {result.error}")
    
    # Analyse
    successful = [r for r in results if r.success]
    print(f"\n=== RÉSUMÉ ===")
    print(f"Réussis: {len(successful)}/{len(results)}")
    
    if successful:
        latencies = [r.latency_seconds for r in successful]
        costs = [r.cost_usd for r in successful]
        confidences = [r.confidence for r in successful]
        verdicts = [r.verdict for r in successful]
        retries = [r.retries for r in successful]
        
        print(f"Latence: moy={statistics.mean(latencies):.2f}s | méd={statistics.median(latencies):.2f}s | max={max(latencies):.2f}s")
        print(f"Coût/test: moy=${statistics.mean(costs):.6f} | total=${sum(costs):.4f}")
        print(f"Confiance: moy={statistics.mean(confidences):.2f} | min={min(confidences):.2f}")
        print(f"Retries: total={sum(retries)} | max={max(retries)}")
        print(f"Verdicts: {{", end="")
        for v in sorted(set(verdicts)):
            print(f" {v}: {verdicts.count(v)}", end="")
        print(" }")
        
        print(f"\n--- PROJECTION ---")
        print(f"29 critères IA: ${sum(costs):.4f}/audit")
        print(f"100 audits/mois: ${sum(costs)*100:.2f}")
    
    with open("holo3_benchmark_v2_results.json", "w") as f:
        json.dump([asdict(r) for r in results], f, indent=2, ensure_ascii=False)
    print("\nSauvé: holo3_benchmark_v2_results.json")


if __name__ == "__main__":
    asyncio.run(main())