# YBOS

> **Status**: Pre-MVP, fundațiile arhitecturale livrate. Y0-Y7 done (PR #1-8). Următoarele faze: primii agenți seed (News Digest, Calendar), User-Context Memory subsystem, Privacy Firewall Layer 2/3.

YBOS este un sistem de operare AI-native, **mobile-first** cu **laptop companion**, în care un agent LLM principal orchestrează agenți specializați (calendar, business trip, learning, market intel, news) și permite utilizatorului să-și creeze proprii agenți pentru orice task. Datele user-ului rămân pe device prin design.

## Caracteristici cheie

- **AI Executive Assistant OS** — agentul LLM principal e și asistent, și paznic privacy
- **Per-user identity** — fiecare device personalizat la primul boot (nume + PIN + biometric opțional)
- **Privacy by design** — firewall în 3 layere (Layer 1 capabilities + path normalization + audit log = livrat; Layer 2 eBPF redactor + Layer 3 LLM judge = TBD)
- **Agent Builder Framework** — user creează agenți noi pentru orice task, fără release nou (planificat)
- **User-Context Memory** — sistemul învață preferințe și recurrențe (zboruri, calendar, contacts; planificat)
- **Laptop Companion** — Tauri app cross-platform, session-based (paradigma WhatsApp Web), folosește RAM/GPU laptop pentru LLM mari (planificat)
- **Android compatibility** — Google Play apps merg nativ (baza AOSP)
- **Rust everywhere** — userland nou și kernel modules pentru performance + safety

## Structură workspace

```
YBOS/
├── l0/             # Reflex Layer — daemon Rust kernel-adjacent (identity, telemetry, gRPC)
├── orchestrator/   # Agentic Layer (L1) — Agent trait, AgentRuntime, capability enforcement, audit log
├── inference/      # Cognitive Layer (L2) — Inference trait, MockInference, LocalLlama (llama-cpp-2)
├── memory/         # Memory Layer — VectorStore + Embedder traits, sqlite-vec store, fastembed embedder
├── proto/          # Shared gRPC schemas (consumed by l0 + orchestrator)
├── platform/       # AOSP build scaffolds (overlay, sync scripts, flash procedure)
└── docs/           # ARCHITECTURE, ROADMAP, HARDWARE, L0_SACRED
```

## Ce funcționează acum

- **Identity & onboarding**: enrollment dinamic per-user cu Argon2id (envelope A) + BIP39 paper backup + HMAC-signed identity blob + boot integrity tripwire pe L0 SACRED files
- **Session tokens**: API L0 cu HKDF derivation, expuse via gRPC (`SessionService`); orchestrator are client real
- **AOSP build scaffolds**: setup script Ubuntu, AOSP sync, device-agnostic overlay (init.rc, sepolicy, system.prop), flash procedure documentată
- **Cross-compile**: `ybos-l0` se compilează pentru `aarch64-linux-android` în CI
- **L1 orchestrator**: Agent trait + InProcessRuntime + AgentRegistry (static + runtime); capability enforcement cu path normalization (`..` bypass blocat) și audit log structurat (tracing target `ybos.audit`)
- **L2 inference**: trait `Inference` (sync `complete()` + streaming `complete_stream()`); LocalLlama via `llama-cpp-2` CPU; MockInference pentru teste; RemoteAPI stub pentru cloud burst viitor
- **Memory**: trait `VectorStore` + `Embedder`; `SqliteVecStore` (rusqlite + sqlite-vec) și `FastEmbedEmbedder` (BGE-small-en-v1.5); mocks pentru teste
- **AgentContext injection**: agenți primesc `Arc<dyn Inference>` + `Arc<dyn VectorStore>` + `Arc<dyn Embedder>` cu capability gates (`llm`, `memory: Read|ReadWrite`)
- **CI**: 7 jobs (workspace test, cross-compile aarch64, inference mock + LocalLlama smoke, memory mock + fastembed smoke, ShellCheck)

## Ce vine

Vezi `docs/ROADMAP.md` pentru detalii. Faza activă: Y8 (primii agenți seed) după ce achiziționăm device-ul de test pentru Y2.b (flash + boot verification).

## Pentru dezvoltatori

- `YBOSClaude.md` — instrucțiuni Claude Code (source of truth context pentru AI codegen)
- `docs/ARCHITECTURE.md` — arhitectură detailed (3-layer brain + laptop companion + user-context)
- `docs/HARDWARE.md` — device test specs (flexibile, NU hardcodate)
- `docs/L0_SACRED.md` — protocol securitate L0 (refuz hard la modificare runtime)
- `docs/ROADMAP.md` — faze (Y0-Y7 done detaliat, Y8+ enumerate)
- `reference/REMUS_PORT_NOTES.md` — istoric: ce am portat din [RemusOS3](https://github.com/PGC22/RemusOS3) (prototipul inițial)
- `l0/`, `orchestrator/`, `inference/`, `memory/`, `proto/` — fiecare crate are README propriu cu detalii features + how-to-run

### Build local

```bash
# Workspace complet (mock-uri default — rapid):
cargo build --workspace
cargo test --workspace --features ybos-l0/dev_test_init

# LocalLlama real (download TinyLlama Q4_K_M ~600MB):
cargo test -p ybos-inference --features local_llama

# Memory real (download BGE-small ONNX ~130MB + sqlite-vec):
cargo test -p ybos-memory --features fastembed,sqlite_vec

# Cross-compile aarch64-linux-android (necesită cross crate + Docker):
cross build -p ybos-l0 --release --target aarch64-linux-android
```

## Licență

TBD. Acest repo e public ca planning în desfășurare; license-ul final va fi stabilit înainte de release.

---

*Code implemented with help from AI Agents Claude, Codex, Jules.*
