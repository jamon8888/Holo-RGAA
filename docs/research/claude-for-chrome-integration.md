# Claude for Chrome : surface d'intégration pour une autre extension (#491)

Date : 2026-11-23. Parent : #490. Sources primaires uniquement (Anthropic, Chrome).

## Réponse courte

Non : aucune API, aucun protocole ni mécanisme documenté et stable ne permet à une extension tierce de piloter le panneau « Claude in Chrome » (prompt + capture par critère, récupération de la réponse). **Recommandation : contourner**, en appelant Claude directement (API Messages avec images ; Agent SDK si besoin d'une boucle agentique), pas en s'intégrant à l'extension Claude.

## Constats (chaque point cite sa source)

1. **Pas d'API publique documentée pour l'extension.** L'article d'aide décrit l'extension (lecture, clics, navigation) et ses usages (panneau latéral, Cowork, Claude Code) sans API tierce. Il note que la permission `nativeMessaging` sert à « préparer une intégration avec d'autres produits desktop Anthropic ».
   Source : https://support.claude.com/en/articles/12012173-getting-started-with-claude-in-chrome
2. **Le seul pont documenté est Claude Code <-> extension**, via un native messaging host (`com.anthropic.claude_code_browser_extension.json`) et un serveur MCP interne `claude-in-chrome` ; relais via `bridge.claudeusercontent.com`. Il exige `/login` claude.ai (plan Pro/Max/Ultra/Team/Enterprise) : clé API ou `setup-token` désactivent l'intégration ; indisponible via Bedrock/Vertex/Foundry, ZDR, et WSL ; extension et Claude Code doivent être sur le même compte/organisation.
   Source : https://code.claude.com/docs/en/chrome
3. **Une extension tierce ne peut pas se brancher sur ce host.** `allowed_origins` liste explicitement les extensions autorisées, sans joker ; les autres reçoivent une erreur d'accès refusé.
   Source : https://developer.chrome.com/docs/extensions/develop/concepts/native-messaging
4. **Messagerie inter-extensions Chrome : possible seulement si le destinataire l'expose** (`runtime.onMessageExternal` / `onConnectExternal`, filtrage par `sender.id`). La doc Anthropic ne documente aucun tel point d'entrée pour l'extension Claude ; s'appuyer sur des messages non documentés serait fragile et peut casser à chaque mise à jour. (Inférence : absence de documentation, pas preuve d'absence technique.)
   Source : https://developer.chrome.com/docs/extensions/develop/concepts/messaging
5. **Extension Claude : Chrome uniquement** selon l'aide (autres navigateurs « non pris en charge »), alors que la doc Claude Code mentionne aussi Edge et d'autres Chromium. Contradiction entre pages Anthropic ; à ne pas tenir pour acquise.
   Sources : pages des points 1 et 2.
6. **Risque de conformité** : Anthropic n'autorise pas les tiers à proposer le login claude.ai ou ses limites de débit dans leurs produits (agents Agent SDK compris) ; utiliser l'authentification par clé API.
   Source : https://code.claude.com/docs/en/agent-sdk/overview

## Alternatives officielles

- **API Messages + vision** : envoi d'images (base64/URL/Files API) avec le prompt par critère ; réponse dans la réponse API. Simple, stable, authentification par clé API. Source : https://platform.claude.com/docs/en/build-with-claude/vision
- **Agent SDK (Python/TS)** : boucle d'agent, outils, MCP, hooks, dans un processus que nous exploitons ; clé API requise. Source : https://code.claude.com/docs/en/agent-sdk/overview
- **Outil computer use** : capture/clics côté application dans un environnement que nous contrôlons (VM/conteneur) ; pour les tâches limitées au web la page recommande le « browser use tool ». Risques d'injection de prompt, confirmation humaine conseillée. Source : https://platform.claude.com/docs/en/agents-and-tools/tool-use/computer-use-tool
- **MCP** : exposer nos outils (capture, règles RGAA) à Claude Code / Agent SDK. Source : https://code.claude.com/docs/en/agent-sdk/overview

## Risques de la voie « contourner »

- Coût par token et par image (l'historique de captures grossit vite : environ 1 000-1 800 jetons/capture selon la page computer use) ; plafonds d'images par requête.
- Clé API et données (captures de pages client) transmises à Anthropic : cadrer RGPD/DPA.
- Nous maintenons la capture et l'orchestration nous-mêmes (l'extension ne fait rien pour nous).

## Risques de la voie « intégrer » (déconseillée)

Aucune garantie de stabilité, dépendance au compte claude.ai de l'utilisateur, non-conformité probable aux conditions (point 6), pas d'accès programmatique documenté à la réponse.

## Non vérifié

Le comportement réel de `onMessageExternal` de l'extension Claude n'a pas été testé (hors périmètre des sources documentaires) ; la liste de modèles et le jeton exact de l'outil computer use évoluent vite : relire les pages avant implémentation.
