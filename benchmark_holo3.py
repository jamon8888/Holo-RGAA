#!/usr/bin/env python3
"""
Benchmark Holo3-35B (holo3-1-35b-a3b) for RGAA IA_ASSISTE criteria evaluation.
Tests latency, cost estimation, and structured output quality on 29 criteria.
"""

import os
import time
import json
import statistics
from typing import List, Dict, Any
from dataclasses import dataclass, asdict
from openai import OpenAI

# Configuration
API_KEY = "hk-a73b030c64aac335fc3651c280c95694beb8df95c4a5d8b1"
BASE_URL = "https://api.hcompany.ai/v1/"
MODEL = "holo3-1-35b-a3b"
RATE_LIMIT_RPM = 10  # Free tier

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


SYSTEM_PROMPT = """Tu es un expert accessibilité numérique RGAA 4.1.2. Évalue le critère suivant sur la base du contexte fourni (HTML, texte, image).

Réponds UNIQUEMENT en JSON structuré :
{
  "verdict": "CONFORME | NON_CONFORME | NE_PAS_SAVOIR",
  "confidence": 0.0-1.0,
  "justification": "Explication courte (max 2 phrases) de ton verdict",
  "elements_manquants": ["liste", "d'éléments", "nécessaires", "pour", "décider"]
}

Règles :
- CONFORME seulement si preuve évidente dans le contexte
- NON_CONFORME seulement si violation évidente
- NE_PAS_SAVOIR si contexte insuffisant (par défaut)
- confidence ≥ 0.85 pour verdicts CONFORME/NON_CONFORME
- confidence < 0.85 → NE_PAS_SAVOIR
"""

USER_PROMPT_TEMPLATE = """Critère RGAA : {criterion_id} - {criterion_title}

Contexte (extrait HTML/page) :
{context}

Évalue ce critère. Réponds en JSON uniquement."""


# Contextes de test représentatifs (simulés)
TEST_CONTEXTS = {
    "1.3": '<img src="chart.png" alt="Graphique des ventes 2024">',
    "1.7": '<img src="complex-diagram.png" alt="Diagramme complexe" longdesc="diagram-desc.html">',
    "2.2": '<iframe title="Paiement sécurisé Stripe" src="..."></iframe>',
    "4.2": '<video><track kind="captions" src="subs.vtt" srclang="fr"></video>',
    "4.4": '<video><track kind="captions" src="subs.vtt" srclang="fr"></video>',
    "4.6": '<video><track kind="descriptions" src="ad.vtt" srclang="fr"></video>',
    "4.9": '<canvas aria-label="Graphique interactif des ventes"></canvas>',
    "5.2": '<table><caption>Ventes 2024</caption><tr><th scope="col">Mois</th>...</table>',
    "5.3": '<table role="presentation"><tr><td>Col 1</td><td>Col 2</td></tr></table>',
    "5.5": '<table><caption>Budget 2024</caption>...</table>',
    "7.2": '<div role="alert" aria-live="polite">Erreur: email invalide</div>',
    "8.4": '<html lang="fr">',
    "8.6": '<title>Accueil - Mon Service Public</title>',
    "8.8": '<p lang="en">Welcome to our website</p>',
    "9.2": '<h1>Titre principal</h1><h2>Sous-section</h2><h3>Détail</h3>',
    "10.3": '<div class="content">Texte important</div>',
    "10.10": '<div style="color:red; font-weight:bold">Erreur</div>',
    "11.2": '<label for="email">Adresse email</label><input id="email" type="email">',
    "11.3": '<label for="email1">Email</label><input id="email1"><label for="email2">Email</label><input id="email2">',
    "11.7": '<fieldset><legend>Coordonnées</legend><label>Nom</label><input>...</fieldset>',
    "11.8": '<select><optgroup label="France"><option>Paris</option></optgroup></select>',
    "11.9": '<button type="submit">Valider mon inscription</button>',
    "11.10": '<input type="email" required aria-describedby="err-email"><span id="err-email">Format invalide</span>',
    "12.3": '<nav><ul><li><a href="/plan">Plan du site</a></li></ul></nav>',
    "12.8": '<a href="#content">Aller au contenu</a><nav>...<main id="content">',
    "13.6": '<p>ASCII art: ¯\\_(ツ)_/¯</p>',
    "3.1": '<span style="color:red">Erreur</span> <span class="icon">⚠</span>',
}


def build_prompt(criterion: Dict[str, str]) -> str:
    context = TEST_CONTEXTS.get(criterion["id"], "Contexte non fourni pour ce test")
    return USER_PROMPT_TEMPLATE.format(
        criterion_id=criterion["id"],
        criterion_title=criterion["title"],
        context=context
    )


def estimate_cost(prompt_tokens: int, completion_tokens: int) -> float:
    # holo3-1-35b-a3b: $0.25 / $1.80 per 1M tokens
    input_cost = (prompt_tokens / 1_000_000) * 0.25
    output_cost = (completion_tokens / 1_000_000) * 1.80
    return input_cost + output_cost


def run_single_benchmark(criterion: Dict[str, str]) -> BenchmarkResult:
    prompt = build_prompt(criterion)
    
    start = time.perf_counter()
    try:
        response = client.chat.completions.create(
            model=MODEL,
            messages=[
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": prompt}
            ],
            temperature=0.1,
            max_tokens=500,
            response_format={"type": "json_object"}
        )
        latency = time.perf_counter() - start
        
        usage = response.usage
        prompt_tokens = usage.prompt_tokens
        completion_tokens = usage.completion_tokens
        cost = estimate_cost(prompt_tokens, completion_tokens)
        
        content = response.choices[0].message.content
        parsed = json.loads(content)
        
        return BenchmarkResult(
            criterion_id=criterion["id"],
            criterion_title=criterion["title"],
            prompt_tokens=prompt_tokens,
            completion_tokens=completion_tokens,
            latency_seconds=latency,
            cost_usd=cost,
            verdict=parsed.get("verdict", "ERROR"),
            confidence=parsed.get("confidence", 0.0),
            success=True
        )
    except Exception as e:
        latency = time.perf_counter() - start
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
            error=str(e)
        )


def main():
    print(f"=== Benchmark Holo3-35B pour RGAA IA_ASSISTE ===")
    print(f"Modèle: {MODEL}")
    print(f"Critères à tester: {len(IA_ASSISTE_CRITERIA)}")
    print(f"Rate limit: {RATE_LIMIT_RPM} RPM (free tier)")
    print()
    
    results: List[BenchmarkResult] = []
    
    for i, criterion in enumerate(IA_ASSISTE_CRITERIA, 1):
        print(f"[{i}/{len(IA_ASSISTE_CRITERIA)}] Test {criterion['id']} - {criterion['title'][:50]}...")
        
        result = run_single_benchmark(criterion)
        results.append(result)
        
        if result.success:
            print(f"  ✓ {result.latency_seconds:.2f}s | {result.prompt_tokens}+{result.completion_tokens} tokens | ${result.cost_usd:.6f} | {result.verdict} (conf: {result.confidence:.2f})")
        else:
            print(f"  ✗ Échec: {result.error}")
        
        # Respect rate limit: 6 seconds between requests (10 RPM = 6s interval)
        if i < len(IA_ASSISTE_CRITERIA):
            time.sleep(6.5)
    
    # Analyse résultats
    successful = [r for r in results if r.success]
    failed = [r for r in results if not r.success]
    
    print("\n=== RÉSUMÉ ===")
    print(f"Réussis: {len(successful)}/{len(results)}")
    print(f"Échoués: {len(failed)}")
    
    if successful:
        latencies = [r.latency_seconds for r in successful]
        costs = [r.cost_usd for r in successful]
        prompt_tokens = [r.prompt_tokens for r in successful]
        completion_tokens = [r.completion_tokens for r in successful]
        confidences = [r.confidence for r in successful]
        verdicts = [r.verdict for r in successful]
        
        print(f"\nLatence: moy={statistics.mean(latencies):.2f}s | méd={statistics.median(latencies):.2f}s | min={min(latencies):.2f}s | max={max(latencies):.2f}s")
        print(f"Tokens: prompt moy={statistics.mean(prompt_tokens):.0f} | completion moy={statistics.mean(completion_tokens):.0f}")
        print(f"Coût/test: moy=${statistics.mean(costs):.6f} | total 29 critères ≈ ${sum(costs):.4f}")
        print(f"Confiance: moy={statistics.mean(confidences):.2f} | min={min(confidences):.2f}")
        print(f"Verdicts: {dict((v, verdicts.count(v)) for v in set(verdicts))}")
        
        # Projection audit complet (29 critères IA + 73 déterministes = 106)
        # Mais seul IA utilise Holo3
        print(f"\n--- PROJECTION COÛT AUDIT ---")
        print(f"29 critères IA_ASSISTE: ${sum(costs):.4f}")
        print(f"Coût par audit (estimation): ${sum(costs):.4f}")
        print(f"À 10 audits/jour: ${sum(costs)*10:.2f}/jour | ${sum(costs)*300:.2f}/mois")
    
    # Sauvegarde détaillée
    with open("holo3_benchmark_results.json", "w") as f:
        json.dump([asdict(r) for r in results], f, indent=2, ensure_ascii=False)
    print("\nRésultats détaillés sauvés dans holo3_benchmark_results.json")


if __name__ == "__main__":
    main()