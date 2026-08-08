#!/usr/bin/env python3
"""
Benchmark Holo3-35B rapide : test sur 5 critères représentatifs seulement.
"""

import os
import time
import json
import statistics
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, asdict
from openai import OpenAI

API_KEY = "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1"
BASE_URL = "https://api.hcompany.ai/v1/"
MODEL = "holo3-1-35b-a3b"

client = OpenAI(base_url=BASE_URL, api_key=API_KEY)


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


# 5 critères représentatifs (diversité thèmes + difficulté)
TEST_CRITERIA = [
    {"id": "1.3", "title": "Alternative textuelle pertinente (image porteuse d'info)", "theme": "Images"},
    {"id": "11.2", "title": "Étiquette champ formulaire pertinente", "theme": "Formulaires"},
    {"id": "9.2", "title": "Structure document cohérente", "theme": "Structuration"},
    {"id": "12.8", "title": "Ordre de tabulation cohérent", "theme": "Navigation"},
    {"id": "3.1", "title": "Information non donnée uniquement par la couleur", "theme": "Couleurs"},
]

SYSTEM_PROMPT = """Tu es un expert accessibilité numérique RGAA 4.1.2. Évalue le critère sur la base du contexte HTML fourni.

Réponds UNIQUEMENT en JSON valide :
{
  "verdict": "CONFORME" | "NON_CONFORME" | "NE_PAS_SAVOIR",
  "confidence": 0.0-1.0,
  "justification": "Explication courte (max 2 phrases)",
  "elements_manquants": ["liste", "d'éléments", "nécessaires"]
}

Règles :
- CONFORME seulement si preuve évidente
- NON_CONFORME seulement si violation évidente  
- NE_PAS_SAVOIR par défaut si contexte insuffisant
- confidence ≥ 0.85 pour CONFORME/NON_CONFORME
- confidence < 0.85 → NE_PAS_SAVOIR
"""

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
    "11.2": '''<form>
  <div class="field">
    <label for="email">Adresse email <span aria-hidden="true">*</span></label>
    <input type="email" id="email" name="email" required autocomplete="email" aria-describedby="email-hint">
    <span id="email-hint">Format : prenom.nom@domaine.fr</span>
  </div>
</form>''',
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
    "12.8": '''<a href="#main" class="skip-link">Aller au contenu principal</a>
<header>...</header>
<nav>Menu principal</nav>
<main id="main">
  <h1>Titre page</h1>
  <article>Contenu</article>
</main>''',
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
    if not content:
        return None
    content = content.strip()
    if content.startswith("```json"):
        content = content[7:]
    if content.endswith("```"):
        content = content[:-3]
    content = content.strip()
    try:
        return json.loads(content)
    except json.JSONDecodeError:
        start = content.find("{")
        end = content.rfind("}")
        if start >= 0 and end > start:
            try:
                return json.loads(content[start:end+1])
            except json.JSONDecodeError:
                pass
    return None


def run_criterion_with_retry(criterion: Dict[str, str], max_retries: int = 2) -> BenchmarkResult:
    context = REAL_CONTEXTS.get(criterion["id"], "Contexte non disponible")
    prompt = f"""Critère RGAA : {criterion['id']} - {criterion['title']}

Contexte HTML :
{context}

Évalue ce critère. Réponds en JSON uniquement."""
    
    for attempt in range(max_retries + 1):
        start = time.perf_counter()
        try:
            response = client.chat.completions.create(
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
                raise ValueError(f"JSON invalide: {content[:200]}")
                
        except Exception as e:
            latency = time.perf_counter() - start
            if attempt == max_retries:
                return BenchmarkResult(
                    criterion_id=criterion["id"],
                    criterion_title=criterion["title"],
                    prompt_tokens=0, completion_tokens=0,
                    latency_seconds=latency, cost_usd=0.0,
                    verdict="ERROR", confidence=0.0,
                    success=False, error=str(e), retries=attempt
                )
            time.sleep(1.0 * (attempt + 1))
    
    return BenchmarkResult(criterion_id=criterion["id"], criterion_title=criterion["title"],
                          prompt_tokens=0, completion_tokens=0, latency_seconds=0, cost_usd=0,
                          verdict="ERROR", confidence=0, success=False, error="Max retries")


def main():
    print(f"=== Benchmark Holo3-35B Rapide (5 critères) ===")
    print(f"Modèle: {MODEL}\n")
    
    results = []
    for i, criterion in enumerate(TEST_CRITERIA, 1):
        print(f"[{i}/{len(TEST_CRITERIA)}] {criterion['id']} - {criterion['title'][:55]}...")
        result = run_criterion_with_retry(criterion)
        results.append(result)
        
        if result.success:
            print(f"  ✓ {result.latency_seconds:.2f}s | {result.prompt_tokens}+{result.completion_tokens} tok | ${result.cost_usd:.6f} | {result.verdict} (conf: {result.confidence:.2f}) [retries: {result.retries}]")
        else:
            print(f"  ✗ {result.error}")
        
        if i < len(TEST_CRITERIA):
            time.sleep(6.5)  # rate limit
    
    successful = [r for r in results if r.success]
    print(f"\n=== RÉSUMÉ ===")
    print(f"Réussis: {len(successful)}/{len(results)}")
    
    if successful:
        latencies = [r.latency_seconds for r in successful]
        costs = [r.cost_usd for r in successful]
        confidences = [r.confidence for r in successful]
        verdicts = [r.verdict for r in successful]
        
        print(f"Latence: moy={statistics.mean(latencies):.2f}s | méd={statistics.median(latencies):.2f}s | max={max(latencies):.2f}s")
        print(f"Coût/test: moy=${statistics.mean(costs):.6f} | total 5=${sum(costs):.4f} | proj 29≈${sum(costs)/5*29:.4f}")
        print(f"Confiance: moy={statistics.mean(confidences):.2f} | min={min(confidences):.2f}")
        print(f"Verdicts: {dict((v, verdicts.count(v)) for v in set(verdicts))}")
        
        print(f"\n--- PROJECTION AUDIT COMPLET ---")
        proj_cost = sum(costs) / 5 * 29
        print(f"29 critères IA_ASSISTE: ${proj_cost:.4f}/audit")
        print(f"100 audits/mois: ${proj_cost*100:.2f}")
        print(f"1000 audits/mois: ${proj_cost*1000:.2f}")
    
    with open("holo3_benchmark_quick.json", "w") as f:
        json.dump([asdict(r) for r in results], f, indent=2, ensure_ascii=False)
    print("\nSauvé: holo3_benchmark_quick.json")


if __name__ == "__main__":
    main()