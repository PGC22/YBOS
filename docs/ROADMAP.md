# YBOS Roadmap

> Detaliat doar pentru faza curentă / următoare. Restul fazelor sunt enumerate succint, cu semne de întrebare doar acolo unde decizia afectează arhitectura/implementarea fazei active.
>
> **Fără estimări de timp.** Ordinea fazelor + dependențele contează; timpul real e irelevant până când produsul devine vandabil.

---

## Y0 — Bootstrap ✅ Done

- Repo YBOS creat (public, github.com/PGC22/YBOS)
- Structură directoare + docs scrise
- l0/ portat din RemusOS3 (Cargo.toml rebrand `ybos-l0`)
- YBOSClaude.md = source of truth context
- Arhitectură detailed (inclusiv laptop companion + user-context memory + task offload)

---

## Y1 — L0 generalizare ✅ Done (PR #1 merged)

- Identity generalization (struct generic, UUID v4, zero hardcoded owner)
- Onboarding state machine + Argon2id envelope A + BIP39 24-word display + HMAC-signed identity blob
- Session token issuance API (HKDF + scope + revoke + list)
- L0 SACRED tripwire pe layout `${YBOS_DATA}/identity/`
- Trait stubs envelope B (TEE) + C (YubiKey) pentru fază viitoare
- 48 tests pass

**Known carry-over flag** (notat post-merge):
- Envelope A folosește Argon2id-XOR + HMAC tag custom (acceptabil pentru dev scaffold). Înainte de production: înlocuit cu AEAD vetted (AES-GCM-SIV sau ChaCha20-Poly1305).

---

## Y2 — Build environment + cross-compile + AOSP customization scaffolding ✅ Done (PR #2 merged)

- AOSP build host setup script (Ubuntu 22.04 LTS), shellcheck-clean, idempotent
- AOSP source sync workflow + custom manifest scaffold (`ybos-aosp.xml`)
- Cross-compile `ybos-l0` pentru `aarch64-linux-android` via `cross` crate cu `Cross.toml` (image `:edge` cu NDK modern); CI verifică binary ELF aarch64
- AOSP overlay device-agnostic: `BoardConfigCommon.mk`, `system.prop` (cu disclaimer dev-only pe ADB/verity), `init.ybos.rc` (service ybos-l0 cu capabilities documentate), SELinux policy `ybos_l0.te`
- `apply_overlay.sh` cu backup option
- `FLASH_PROCEDURE.md` generic ARM64 cu secțiuni per-OEM marcate "[verificat când achiziționăm X]"
- CI: `Build & Test l0`, `Cross-compile l0 for Android`, `ShellCheck` toate verzi

**Known carry-over flags** (pentru Y2.b post-device):
- Capabilities `CHOWN`/`DAC_OVERRIDE` la ybos-l0 service: re-evaluate cu strace/auditd evidence pe device real
- ADB-on + verity-off din `system.prop`: mută într-un `system_dev.prop` aplicat doar pentru build variants `eng`/`userdebug`
- `Cross.toml` image pin: schimbă din `:edge` (rolling) la SHA fix sau release tag când cross-rs publică versiune stabilă cu NDK modern
- `ybos` user/group: definește în AOSP system files

---

## Y2.b — Flash + boot verification (BLOCKED pe achiziție device)

> Execută George manual când ajunge device-ul. Folosește scaffolds + documentația din Y2.

- Selectare device-specific BoardConfig (Pixel / OnePlus / etc.)
- Kernel config adaptat
- Build complet AOSP YBOS image pentru device-ul achiziționat
- Flash + boot
- Verificare ybos-l0 daemon rulează, telemetria curge

---

## Y3 — L1 orchestrator skeleton + L0 SessionService gRPC ✅ Done (PR #3 merged)

- Cargo workspace conversion (members `[l0, orchestrator]`, shared deps)
- L0 SessionService gRPC nouă: 5 RPCs (IssueToken / RevokeSession / RevokeAll / ListActive / InitializeForTest cu feature gate `dev_test_init`)
- `l0/src/lib.rs` adăugat pentru testing in-process (main.rs intact, L0 SACRED preserved)
- `YBOS_L0_GRPC_LISTEN` env var pentru port override
- orchestrator crate cu: Agent trait + AgentRuntime hybrid (InProcessRuntime impl + SubprocessRuntime placeholder), Manifest cu Capabilities + AccessLevel, capability enforce, AgentRegistry static+runtime, L0Client cu issue_session_token + get_identity, HelloAgent demo
- End-to-end test: orchestrator obține token real via gRPC + register/invoke agents + capability enforcement
- `Cross.toml` mutat la root (workspace-friendly), CI `cross build -p ybos-l0`

**Known carry-over flags** (post-merge):
- Proto duplicate compilation (l0 + orchestrator generează tipuri Rust distincte din același `l0.proto`) — follow-up: shared `ybos-proto` crate
- `revoke_all` count via list-then-revoke (TOCTOU minor) — fix necesită update în Y1 session.rs
- AgentRegistry runtime factory creează agent nou la fiecare `get()` (vs Static shared Arc) — inconsistent lifecycle
- `tokio-stream` cu feature "net" inocuu (feature nu există, silently ignorat)
- `.unwrap()` pe RwLock în orchestrator (panică la poison)
- Capability path normalization absent (`FsRead(../../../etc/passwd)` bypass risk) — adresat în Y7 firewall hardening

---

## Y4 — LLM inference layer (skeleton + LocalLlama CPU) ⭐ NEXT

> Decizii agreate (2026-05-22 sesiune Y4):
> - **Scope Y4**: skeleton COMPLET + LocalLlama real via `llama-cpp-2` Rust crate (CPU-only, no NPU). Cross-compile aarch64 deferat (NPU acceleration mlc-llm e Y4.b post-device).
> - **Workspace**: crate nou `inference/` (package `ybos-inference`).
> - **Streaming**: trait Inference are AMBELE — `complete()` sync + `complete_stream()` streaming.

### Scope Y4

#### A. Crate nou `inference/` (workspace member)

Layout:
```
inference/
├── Cargo.toml                # package = ybos-inference; features pentru LocalLlama heavy build
├── src/
│   ├── lib.rs                # public re-exports
│   ├── trait_def.rs          # Inference trait (sync + streaming)
│   ├── types.rs              # CompleteRequest, CompleteResponse, Token, InferenceError
│   ├── mock.rs               # MockInference (canned responses, no LLM)
│   ├── local_llama.rs        # LocalLlama via llama-cpp-2 (cfg-gated cu feature `local_llama`)
│   └── remote_api.rs         # RemoteAPI stub (returnează NotImplemented; cloud burst design Y... ulterior)
└── tests/
    ├── mock_smoke.rs         # rulează default, fără model
    └── llama_smoke.rs        # cfg-gated cu feature `local_llama`, descarcă model mic, rulează inference real
```

#### B. Inference trait design

```rust
#[async_trait]
pub trait Inference: Send + Sync {
    async fn complete(&self, req: CompleteRequest) -> Result<CompleteResponse, InferenceError>;
    async fn complete_stream(
        &self,
        req: CompleteRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Token, InferenceError>> + Send>>, InferenceError>;
    fn model_info(&self) -> ModelInfo;
}

pub struct CompleteRequest {
    pub prompt: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub stop: Vec<String>,
    pub seed: Option<u64>,
}

pub struct CompleteResponse {
    pub text: String,
    pub finish_reason: FinishReason,
    pub tokens_in: usize,
    pub tokens_out: usize,
}

pub struct Token {
    pub text: String,
    pub logprob: Option<f32>,
}

pub enum FinishReason { Stop, MaxTokens, StopSequence(String), Error(String) }

pub struct ModelInfo {
    pub backend: String,        // "mock" | "local-llama" | "remote-api"
    pub model_name: String,
    pub context_window: usize,
}

pub enum InferenceError {
    ModelLoad(String),
    Generation(String),
    InvalidRequest(String),
    NotImplemented,
}
```

#### C. Implementations

1. **MockInference** (`src/mock.rs`):
   - Constructor: `MockInference::new(canned_responses: Vec<String>)`
   - `complete`: returnează următorul canned response, cycling
   - `complete_stream`: împarte canned response în "tokens" (whitespace split) + simulate delay (50ms/token)
   - Zero dependențe heavy

2. **LocalLlama** (`src/local_llama.rs`, gated `#[cfg(feature = "local_llama")]`):
   - Folosește `llama-cpp-2` crate ([docs.rs/llama-cpp-2](https://docs.rs/llama-cpp-2/))
   - Constructor: `LocalLlama::load(model_path: &Path, params: LlamaParams)` — încarcă model GGUF
   - `LlamaParams`: context_size (default 8192), n_threads (default num_cpus), n_gpu_layers (0 = CPU)
   - `complete`: tokenize prompt → batch eval → sample → decode → return text
   - `complete_stream`: token-by-token via callback, yielded prin `tokio::sync::mpsc` channel
   - Stop sequences enforced via post-token check
   - `model_info()` returnează info despre modelul încărcat (din metadata GGUF)
   - Error mapping: llama-cpp-2 errors → InferenceError

3. **RemoteAPI stub** (`src/remote_api.rs`):
   - Struct cu `endpoint: String, api_key: SecretString` (NU implementat call efectiv în Y4)
   - `complete`/`complete_stream` returnează `Err(InferenceError::NotImplemented)`
   - Existență la nivel de trait pentru viitor cloud burst — design ready, no API leak risk

#### D. Cargo features pentru a izola greutatea llama.cpp

`inference/Cargo.toml`:
```toml
[features]
default = ["mock"]
mock = []                       # always available, no deps
local_llama = ["llama-cpp-2"]   # heavy: cmake + clang + native build
remote_api = []                 # always available, stub for now

[dependencies]
llama-cpp-2 = { version = "...", optional = true }
# trait infra:
async-trait = { workspace = true }
tokio-stream = { workspace = true }
futures = "0.3"
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
secrecy = "0.8"                # pentru SecretString în RemoteAPI
```

Workspace `Cargo.toml` adăugă membru:
```toml
[workspace]
members = ["l0", "orchestrator", "inference"]
```

#### E. Smoke tests

1. **`inference/tests/mock_smoke.rs`** (default, fără feature):
   - Construct MockInference cu 2 canned responses
   - `complete()` → assert text egal cu canned[0]
   - `complete_stream()` → consum stream → join tokens → assert egal cu canned[1]
   - Test FinishReason::MaxTokens când canned > max_tokens

2. **`inference/tests/llama_smoke.rs`** (cfg `feature = "local_llama"`):
   - Download `TinyLlama-1.1B-Chat-v1.0.Q4_K_M.gguf` (~600MB) într-un cache dir (`OUT_DIR/models/` sau `target/test-models/`) DOAR DACĂ nu există deja
   - LocalLlama::load(path) → asserts ok
   - `complete(prompt = "Hello, what is 2+2?", max_tokens = 32)` → asserts response non-gol + nu panică
   - `complete_stream(...)` → consumă stream → asserts > 0 tokens emise

3. **Mock tests pot rula în orice job CI.** Llama smoke test = nou CI job opt-in.

#### F. CI

- **Job nou**: `Build & Test inference (mock)` — `cargo test -p ybos-inference` (default features, no llama-cpp-2)
- **Job nou** (opt-in, mai lent): `LocalLlama smoke test` — `cargo test -p ybos-inference --features local_llama`
  - Caching: model file salvat în GitHub Actions cache (key bazat pe model name + version)
  - cmake + clang necesare (preinstalate pe ubuntu-latest)
  - Timeout generos (10 min) pentru cmake build llama.cpp + download model
- **Job existent**: `Build & Test Workspace` — invocă `cargo test --workspace` cu defaults (mock only, no llama dep) ca să rămână rapid
- **Job existent**: `Cross-compile l0 for Android` — neschimbat, NU adăugăm inference la cross-compile (NPU/Android-specific = Y4.b)
- **Job existent**: `ShellCheck` — neschimbat

#### G. Documentation

- `inference/README.md` — explain crate features, cum se download model, cum se rulează LocalLlama smoke local
- NU update YBOSClaude.md / ARCHITECTURE.md / etc. (out of scope; Lead Dev face în review post-merge dacă necesar)

### Acceptance criteria Y4

- [ ] `inference/` crate creat ca workspace member, `cargo build -p ybos-inference` verde
- [ ] `cargo test --workspace` verde (mock tests pass, llama tests filtered out fără feature)
- [ ] `cargo test -p ybos-inference --features local_llama` verde (CI cu cache model, rulează inference real cu TinyLlama)
- [ ] Inference trait cu `complete()` + `complete_stream()` definit corect
- [ ] MockInference, LocalLlama (cfg `local_llama`), RemoteAPI stub toate implementate
- [ ] Zero modificări în `l0/**`, `orchestrator/**`, `docs/**`, `YBOSClaude.md`, `README.md` root, `reference/**`, `platform/**`
- [ ] Workspace `Cargo.toml` actualizat doar cu noul membru
- [ ] CI: toate jobs existing verzi + 2 jobs noi (inference mock + LocalLlama)
- [ ] `inference/README.md` documentat cu features + how-to local run

### Ce NU intra în Y4

- NPU acceleration via mlc-llm — Y4.b post-device
- Cross-compile inference pentru aarch64-linux-android — Y4.b
- Orchestrator integration (Agent → Inference injection) — fază separată (Y4.c sau Y5)
- Vector store (sqlite-vec / qdrant) — fază separată (Y4.d)
- RemoteAPI real impl (Anthropic / OpenAI calls) — Y15 (cloud burst activation)
- LLM judge sub-agent (Privacy Firewall Layer 3) — Y9
- Streaming over gRPC (orchestrator-side wrapper) — fază separată
- Tool calling / function calling API — fază separată

---

## Y4.b — NPU acceleration + cross-compile aarch64 (BLOCKED pe achiziție device + Y2.b)

- mlc-llm integration pentru NPU acceleration (Tensor G2/G3, Hexagon, Mediatek APU)
- Cross-compile `ybos-inference` pentru aarch64-linux-android
- Benchmark CPU vs NPU pe device real
- RAM usage profiling (<3GB target per ROADMAP acceptance)
- Cu device disponibil: validare prompt → response în <5s cu model 3B

---

## Y5+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Semne de întrebare doar unde **chiar afectează faza activă (Y4)**.

- **Orchestrator integration cu Inference** — Agent → Inference handle prin AgentRuntime; ❓ design API: injection prin context-passing sau global handle?
- **Vector store** (sqlite-vec sau qdrant embedded) — pentru memorie semantică per-agent. Independent de Y4.
- **Agent seed: Calendar** — primul agent end-to-end demo cu LLM tools (consumă Inference + Vector store).
- **Agent seed: News Digest**.
- **Privacy firewall Layer 1 (capabilities)** — Y3 livrat skeleton; Y7 hardenizează enforcement pe toate operațiile + audit log + UI.
- **Privacy firewall Layer 2 (eBPF redactor)**.
- **Privacy firewall Layer 3 (LLM judge)** — folosește Inference (un sub-model mic).
- **Agent seed: Trip Planner**.
- **Agent seed: Market Intel**.
- **Agent seed: Learning Curator**.
- **Agent Builder Framework** — template `agents/_template/` + LLM-assisted configurator (folosește Inference).
- **User-Context Memory subsystem** — storage + sync + capability `data.user_prefs`.
- **Laptop Companion (Tauri)** — pairing QR/NFC + session crypto + task offload + cache sync.
- **UI native YBOS mobile**.
- **Cross-device extins** (multi-phone, tabletă) — post-MVP.
- **Cloud burst activation** — v0.2+; activează RemoteAPI cu API key real, per-category opt-in user.
- **VM Mode (Tier 1) laptop** — Linux VM minim, GPU passthrough, SEV-SNP/TDX integration. Research, post-MVP.
- **Split inference layer-by-layer** ❓ research item (vezi ARCHITECTURE.md §4.5). Independent.
- **SubprocessRuntime impl real** — pentru process isolation agenți.

---

## Post-MVP (TBD)

- iOS app companion (read-only, view dashboards)
- Linux distro twin (laptop) — reutilizat pentru VM Mode laptop
- Multi-tenancy laptop (multi-user per device)
- Plugin SDK pentru agenți third-party
- Marketplace agenți (community, sandboxed)
- B2B enterprise features
- Hardware research: split inference, FHE accelerator, custom secure element

---

## Notă pe estimare

Documentul a avut estimări de săptămâni/luni anterior. Au fost scoase intenționat (decizie 2026-05-21 sesiunea 2). Motiv: livrabilitate viabilă (vandabilă) depinde de calitate + acceptanță, nu de un calendar arbitrar. Adăugăm milestone-uri reale când avem semnale (alfa privată, feedback testers, etc.).
