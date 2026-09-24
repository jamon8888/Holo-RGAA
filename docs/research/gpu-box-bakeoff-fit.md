# gpu-box capacity and model fit for bake-off (ticket #187, part of map #183)

Date: 2026-09-24 · Branch: `research/gpu-box-bakeoff-fit` · Facts only — roster/protocol decided in #192.

## 0. Verdict

| Question | Answer |
|---|---|
| gpu-box reachable? | **No** — DNS resolution fails (SERVFAIL), no hosts entry, no VPN. Hardware/VRAM/Ollama version/loaded models: **unknown from this environment**; human access required (checklist §1). |
| Do the models fit? | Conditional on VRAM we could not read: Q4 Holo3-35B-A3B ≈ 21.4 GB and Q4 Qwen3.8-27B ≈ 18 GB each fit a 24 GB card **separately**, not together; BF16 of either needs ≥ 55–72 GB. NVFP4 of Qwen3.8-27B needs Blackwell. |
| Ollama vs vLLM for JSON verdicts? | Ollama enforces `response_format` json_schema locally (v0.5.0+), but does **not** honour `enable_thinking=false` as a request field — its equivalent is `think:false`, and the OpenAI-compat `/v1` path has open bugs disabling thinking. vLLM supports both `chat_template_kwargs.enable_thinking` and json_schema natively. |
| Latency order of magnitude | Dense 27B ≈ 10–95 tok/s single-GPU (Q4, hardware-dependent); 35B-A3B (3B active) ≈ 27–139 tok/s — roughly **2–4× faster decode** than dense 27B at similar quant. |

## 1. Reachability of gpu-box (measured here, 2026-09-24)

Commands and exact errors:

```
$ curl -sS -m 5 http://gpu-box:11434/api/tags
curl: (6) Could not resolve host: gpu-box            # exit 6

$ curl -sS -m 5 http://gpu-box:11434/v1/models
curl: (6) Could not resolve host: gpu-box            # exit 6

$ getent hosts gpu-box
                                                   # exit 2 (not found)

$ nslookup gpu-box
** server can't find gpu-box: SERVFAIL              # via 127.0.0.53 (systemd-resolved)

$ resolvectl query gpu-box
gpu-box: resolve call failed: No appropriate name servers or networks for name found
```

Environment facts:

- Uplink DNS: `172.20.202.23` on Wi-Fi link `wlp3s0` (`172.20.202.242/24`); no search domain that could qualify `gpu-box`.
- `/etc/hosts`: no `gpu-box` entry (only localhost + this host).
- No VPN configured: `/etc/openvpn` and `/etc/wireguard` absent, `wg` CLI absent; active connections are wifi + loopback + libvirt `virbr0`.
- No local fallback: `localhost:11434` refuses connection (no Ollama running here), `nvidia-smi` and `ollama` binaries not installed on this machine.
- No `.env` files in repo root or `rgaa-rs/`.

Repo references to gpu-box / Ollama / Holo3 (intended access):

- `rgaa-rs/crates/rgaa-holo/src/backend.rs:170` — the **only** `gpu-box` string: a test fixture endpoint `http://gpu-box:11434/v1/chat/completions` proving `OLLAMA_ENDPOINT` is honoured. Not a runtime default.
- Runtime Ollama config is env-only: `RGAA_LLM_BACKEND=ollama` + `OLLAMA_MODEL` (required) + optional `OLLAMA_ENDPOINT` (`backend.rs:99-102`); default endpoint if unset is `http://localhost:11434/v1/chat/completions` (`ollama.rs:18`).
- Holo3 config: `RGAA_LLM_BACKEND=holo3` + `HOLO3_API_KEY` (`backend.rs:96-98`); agent path also reads optional `HOLO3_BASE_URL` / `HOLO3_MODEL` (default `holo3-1-35b-a3b`, `rgaa-agent/src/config.rs:188-192`); TUI default base `https://api.holo3.ai/v1` (`rgaa-tui/src/tui/setup.rs:9`). CI uses `HOLO3_*` secrets. No hardcoded `gpu-box` outside the test.

**Human access checklist** (in order of least effort):

1. DNS: either add `GFX_IP  gpu-box` to this machine's `/etc/hosts`, point this machine's resolver at an internal DNS that knows `gpu-box`, or hand over the box's IP/hostname directly.
2. Network: confirm port `11434/tcp` reachable from `172.20.202.0/24` (or over VPN) and that Ollama listens on `0.0.0.0:11434`, not loopback (`OLLAMA_HOST`).
3. If a VPN is the intended path: provide the VPN endpoint/config + credentials, then re-run the two curls above.
4. For hardware facts (question 1 of the ticket), run on gpu-box (or provide SSH):
   ```bash
   nvidia-smi                                   # GPU model + VRAM
   ollama --version                             # server version
   curl -s localhost:11434/api/tags             # pullable/known models
   curl -s localhost:11434/api/ps               # currently loaded models + VRAM in use
   curl -s localhost:11434/v1/models            # OpenAI-compat model list
   ```
5. Verification from this machine after any of the above:
   ```bash
   curl -sS -m 5 http://gpu-box:11434/api/tags && echo OK
   curl -sS -m 5 http://gpu-box:11434/v1/models && echo OK
   ```

No credentials are expected for Ollama itself (no auth by default); access gate is network/DNS/VPN only.

## 2. Model fit (public model cards; VRAM of gpu-box unknown)

### Holo3-35B-A3B (base: Qwen3.5-35B-A3B, MoE 35B total / 3B active)

- Official card `Hcompany/Holo3-35B-A3B`: BF16 safetensors, 35B params; Apache 2.0; official quickstarts for vLLM and SGLang (`vllm serve "Hcompany/Holo3-35B-A3B"`). Community quantizations listed on the card (9): FP8, MLX-8bit, GGUF builds etc.
- Memory by precision for the Qwen3.5-35B-A3B base (GGUF, weights only; +1–2 GB KV/runtime at default context):
  | Quant | Weights | Fits 24 GB? |
  |---|---|---|
  | Q4_K_M | ~21.4 GB | yes, tight (~2.5 GB headroom) |
  | Q5_K_M | ~25.2 GB | no |
  | Q6_K | ~28.7 GB | no |
  | Q8_0 | ~37.5 GB | no (needs 48 GB class) |
  | FP16/BF16 | ~71.8 GB | no (multi-GPU / 80 GB class) |
  | INT4 (marlin-style) | ~20 GB | yes |
  | INT8 | ~39 GB | no |
  (Source: willitrunai.com Qwen3.5-35B-A3B VRAM table; spheron INT4 20 / INT8 39 / FP16 78 GB estimates.)
- Holo3.1 successor card states Holo3-family quantizations shipped: BF16, FP8, NVFP4, Q4 GGUF — NVFP4 path exists for the family.

### Qwen3.8-27B (dense 27B, hybrid Gated-DeltaNet attention)

- Official card `Qwen/Qwen3.8-27B`: BF16, 27B params, 56 GB class checkpoint; 262K native context (KV cheap: only 16/64 layers full-attention).
- Footprint by precision:
  | Precision | Size | Fits 24 GB? |
  |---|---|---|
  | Ollama default `qwen3.8:27b` (Q4_K_M) | 18 GB (17.1–17.9 GB file) | yes |
  | Unsloth 4-bit band | 17–19 GB | yes |
  | 6-bit | ~24 GB | borderline |
  | 8-bit / INT8 | ~30–31 GB | no (32 GB card, tight) |
  | NVFP4 (W4A4) | 18.4 GB (vrfai) / ~22 GB weights; ~24.6 GiB working set in vLLM recipe | only on **Blackwell** (compute capability ≥ 10.0 / sm120; `--enforce-eager` on 1×5090) |
  | BF16 | 54.66–56 GB | no |
  (Sources: localaimaster quant table, spheron, vrfai HF card, vLLM recipes `Qwen/Qwen3.8-27B`.)
- The ticket's "≈30 GB NVFP4" figure matches the INT8/8-bit band more than published NVFP4 builds (18.4–24.6 GB); NVFP4 availability depends on Blackwell silicon on gpu-box.

### Coexistence

- Q4+Q4 (21.4 + 18 ≈ 40 GB) needs a 48 GB-class card to be resident together; on 24 GB only one model loaded at a time — Ollama swaps on demand (cold-load latency per switch), which is acceptable for a sequential bake-off but not for interleaved runs.

## 3. Ollama vs vLLM for structured JSON verdicts

| Capability | Ollama (local) | vLLM |
|---|---|---|
| JSON schema enforcement | yes — `format` on `/api/chat`, and OpenAI-compat `response_format` json_schema on `/v1/chat/completions` (docs.ollama.com/structured-outputs; self-hosted v0.5.0+ grammar-enforced per Pydantic docs). Ollama **Cloud** does not enforce (ollama#12362). | yes — `response_format` json_schema by default, xgrammar/guidance backends (docs.vllm.ai/structured_outputs). |
| `enable_thinking=false` as specified | **no** — that's a Qwen chat-template kwarg; Ollama ignores it (llama_index#18635, ollama#10809). Ollama's own switch is `think:false` on `/api/chat`, but the OpenAI `/v1` path has open bugs disabling it for Qwen (ollama#17969, #17588), and `think:false` has broken `format` on at least one model (ollama#15260). | **yes** — `chat_template_kwargs: {"enable_thinking": false}` is the documented mechanism (docs.vllm.ai/reasoning_outputs); works alongside structured outputs (watch vllm#50948 when combining thinking + `enable_in_reasoning`). |
| Fit with `rgaa-holo` today | `OllamaClient` posts plain prompts to `/v1/chat/completions` and parses JSON out of content — works on either server as long as the model emits the verdict JSON. | Same wire path (OpenAI-compat base URL); drop-in via `OLLAMA_ENDPOINT`-style URL or `HOLO3_BASE_URL`. |

Bottom line: for a bake-off protocol that mandates `enable_thinking=false` **and** `response_format: json_schema` over the OpenAI-compatible endpoint, **vLLM is the safe choice**; Ollama covers the schema half but its thinking-off switch on `/v1` is unreliable for Qwen-family models as of open issues. If the protocol relaxes to "no thinking requirement" or uses Ollama's native `/api/chat` `think:false`, Ollama suffices.

## 4. Latency order of magnitude (dense 27B vs 3B-active MoE, single GPU)

Sourced measurements (Q4-class unless noted):

| Setup | Dense 27–32B | MoE 35B/30B-A3B (3–3.3B active) | Ratio |
|---|---|---|---|
| 1× RTX 3090, vLLM, AWQ 4-bit, c=1 (local-llm-eval) | 39 tok/s (Qwen2.5-Coder-32B) | 167 tok/s (Qwen3-Coder-30B-A3B) | 4.3× |
| 3090×2, llama.cpp Q4_K_M, gen-512 (BAEM1N/llm-bench) | 41.4 tok/s (27B) | 138.9 tok/s (35B-A3B) | 3.4× |
| Same, Q8_0 | 27.5 tok/s | 130.3 tok/s | 4.7× |
| M5 Max 128 GB, Ollama | 15.7 tok/s | 57.0 tok/s | 3.6× |
| Mac iq4 Ollama (BatiAI model page) | 17.0 tok/s | 26.6 tok/s | 1.6× |
| RTX 4090, llama.cpp MTP4 (vikesh-c, Qwen3.8-27B) | 54–93 tok/s decode (context 131K→4K) | — | — |

Order of magnitude for budgeting on a single 24 GB consumer GPU: **dense 27B ≈ 15–90 tok/s; 3B-active MoE ≈ 60–170 tok/s (≈2–4× faster decode)**. Prefill is similarly MoE-favoured (llm-bench: 35B-A3B 3,372 vs 27B 3,258 tok/s at 1K on 3090×2 — roughly par at short prompts, MoE pulls ahead at longer ones). Actual gpu-box numbers require access (§1).

## 5. Open items blocked on human access

1. `nvidia-smi` / `ollama --version` / `/api/ps` on gpu-box → answers ticket question 1 and confirms which quant rows of §2 are usable.
2. Re-run §1 verification curls once DNS/VPN/IP is provided.
3. Roster/protocol details → #192 (out of scope here).
