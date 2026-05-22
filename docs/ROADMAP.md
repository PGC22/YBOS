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

**Known carry-over flag** (notat post-merge):
- Envelope A folosește Argon2id-XOR + HMAC tag custom (acceptabil pentru dev scaffold). Înainte de production: înlocuit cu AEAD vetted (AES-GCM-SIV sau ChaCha20-Poly1305).

---

## Y2 — Build environment + cross-compile + AOSP customization scaffolding ✅ Done (PR #2 merged)

- AOSP build host setup script (Ubuntu 22.04 LTS), shellcheck-clean, idempotent
- AOSP source sync workflow + custom manifest scaffold (`ybos-aosp.xml`)
- Cross-compile `ybos-l0` pentru `aarch64-linux-android` via `cross` crate cu `Cross.toml` (image `:edge` cu NDK modern); CI verifică binary ELF aarch64
- AOSP overlay device-agnostic: `BoardConfigCommon.mk`, `system.prop` (cu disclaimer dev-only pe ADB/verity), `init.ybos.rc`, SELinux policy `ybos_l0.te`
- `apply_overlay.sh` cu backup option
- `FLASH_PROCEDURE.md` generic ARM64

**Known carry-over flags** (pentru Y2.b post-device):
- Capabilities `CHOWN`/`DAC_OVERRIDE` la ybos-l0 service: re-evaluate cu strace/auditd pe device
- ADB-on + verity-off din `system.prop`: mută într-un `system_dev.prop` per build variants
- `Cross.toml` image pin: schimbă din `:edge` la SHA fix când cross-rs publică stabil cu NDK modern
- `ybos` user/group: definește în AOSP system files

---

## Y2.b — Flash + boot verification (BLOCKED pe achiziție device)

- Selectare device-specific BoardConfig (Pixel / OnePlus / etc.)
- Kernel config adaptat
- Build complet AOSP YBOS image
- Flash + boot
- Verificare ybos-l0 daemon rulează, telemetria curge

---

## Y3 — L1 orchestrator skeleton + L0 SessionService gRPC + Cargo workspace ✅ Done (PR #3 merged + PR #4 follow-up cleanup)

- Cargo workspace conversion (members `[l0, orchestrator]`, shared deps)
- L0 SessionService gRPC nouă: 5 RPCs (IssueToken / RevokeSession / RevokeAll / ListActive / InitializeForTest cu feature gate `dev_test_init`)
- `l0/src/lib.rs` adăugat pentru testing in-process (main.rs intact, L0 SACRED preserved)
- `YBOS_L0_GRPC_LISTEN` env var pentru port override
- orchestrator crate cu: Agent trait + AgentRuntime hybrid (InProcessRuntime + SubprocessRuntime placeholder), Manifest + Capabilities + AccessLevel, capability enforce, AgentRegistry, L0Client, HelloAgent
- End-to-end test: orchestrator obține token real via gRPC + register/invoke agents + capability enforcement
- `Cross.toml` mutat la root (workspace-friendly)
- Follow-up cleanup PR #4: `revoke_all` returnează count (no TOCTOU), AgentRegistry Runtime factory cache-uiește instance (parity cu Static), RwLock messages descriptive, tokio-stream relocat în dev-deps

**Known carry-over flags** (rămase după PR #4):
- Proto duplicate compilation (l0 + orchestrator generează tipuri Rust distincte din `l0.proto`) — **adresat în Y5 prin extract `ybos-proto` shared crate**
- Capability path normalization absent (`FsRead(../../../etc/passwd)` bypass risk) — adresat în Y7 firewall hardening

---

## Y4 — LLM inference layer (skeleton + LocalLlama CPU) ✅ Done (PR #5 merged)

- Crate nou `inference/` (ybos-inference) ca workspace member
- Inference trait cu `complete()` sync + `complete_stream()` streaming (Pin<Box<dyn Stream>>)
- MockInference cu canned responses cycling, simulated streaming delay
- LocalLlama via `llama-cpp-2 0.1.146`, CPU-only, EOG token check, stop sequence post-token, sampler chain (top_p + temp + seed)
- RemoteAPI stub cu `SecretString` + Debug redacted (returns NotImplemented pentru cloud burst viitor)
- CI: `Build & Test Workspace` rapid (no llama compile in default), `Build & Test inference (mock)`, `LocalLlama smoke test` (cu model cache, TinyLlama-1.1B Q4_K_M ~600MB)
- LlamaBackend singleton via OnceLock + INIT_MUTEX

**Known carry-over flags** (post-PR #5):
- `LocalLlama::complete()` rulează blocking llama-cpp-2 calls direct în corpul `async fn` (NU în `spawn_blocking`) — blochează tokio worker la load concurent; **adresat în Y5** (prerequisite pentru orchestrator integration)
- `FinishReason::StopSequence` și `FinishReason::Error` sunt unit variants fără payload `String` (lossy info despre care stop sequence sau ce eroare); **adresat în Y5**
- Seed truncat la `u32` la `LlamaSampler::dist(seed as u32)` — minor entropy reduction; păstrat (API constraint llama-cpp-2 0.1.x)
- `model_name: "GGUF Model"` placeholder (nu citește GGUF metadata) — adresat în Y5 dacă fezabil cu llama-cpp-2 API
- Context recreate la fiecare `complete()` call (expensive pentru repeat invocations) — deferat, optimization separată
- `Token.logprob: None` mereu — deferat, neimplementat
- Token text format diferit între Mock (manual `" " + token`) și LocalLlama (decoded raw cu spacing intrinsec din tokenizer) — minor inconsistență semantică

---

## Y4.b — NPU acceleration + cross-compile aarch64 (BLOCKED pe achiziție device + Y2.b)

- mlc-llm integration pentru NPU acceleration (Tensor G2/G3, Hexagon, Mediatek APU)
- Cross-compile `ybos-inference` pentru aarch64-linux-android
- Benchmark CPU vs NPU pe device real
- RAM usage profiling (<3GB target)
- Validare prompt → response în <5s cu model 3B

---

## Y5 — Orchestrator ⇌ Inference integration + ybos-proto extraction + Y4 carry-over fixes ⭐ NEXT

> Decizii agreate (2026-05-22 sesiune Y5):
> - **Inference injection model** = `AgentContext` (struct cu `inference: Arc<dyn Inference>`, extensibil pentru servicii viitoare: memory, http_client). `Agent::invoke` primește `&AgentContext` ca al doilea parametru (BREAKING change to Agent trait).
> - **Capability nouă** = `llm: bool` în `Capabilities` struct + nou `Operation::LlmCall` în capability enforce. Agenți fără `llm = true` în manifest primesc `CapabilityDenied` dacă cer inference.
> - **ybos-proto extraction**: da, în Y5. Consolidează tipurile gRPC într-un singur workspace crate.
> - **Y4 LocalLlama bug**: `spawn_blocking` wrap obligatoriu în Y5 (prerequisite pentru concurent agent invocation).

### Scope Y5

#### A. `ybos-proto` shared crate (carry-over Y3 flag #1)

Layout:
```
proto/
├── Cargo.toml                  # package = ybos-proto
├── build.rs                    # tonic-build compilează ambele proto
├── proto/
│   ├── l0.proto                # MUTAT din l0/proto/l0.proto (identic)
│   └── orchestrator.proto      # MUTAT din orchestrator/proto/orchestrator.proto (identic)
└── src/
    └── lib.rs                  # re-exports: ybos_proto::l0::*, ybos_proto::orchestrator::*
```

`src/lib.rs`:
```rust
pub mod l0 {
    tonic::include_proto!("ybos.l0.v1");
}
pub mod orchestrator {
    tonic::include_proto!("ybos.orchestrator.v1");
}
```

Consumers:
- `l0/Cargo.toml`: add `ybos-proto = { path = "../proto" }` (or workspace path), remove `tonic-build` + `protoc-bin-vendored` build-deps + `build.rs` content gone (or build.rs deleted entirely if no other codegen)
- `l0/src/grpc/mod.rs`: replace `pub mod pb { tonic::include_proto!("ybos.l0.v1"); }` cu `pub use ybos_proto::l0 as pb;`
- `l0/src/grpc/{identity,telemetry,reflex,session}_service.rs`, `convert.rs`: update `use super::pb::...` paths (no source change if pb is the same alias)
- `orchestrator/Cargo.toml`: add `ybos-proto = { path = "../proto" }`, remove `tonic-build` + `protoc-bin-vendored`, delete `build.rs` content
- `orchestrator/src/lib.rs`: replace `pub mod pb { mod l0 { tonic::include_proto!(...); } mod orchestrator { ... } }` cu `pub use ybos_proto::{l0, orchestrator};`
- `orchestrator/src/l0_client.rs`: update import paths (e.g. `use crate::pb::l0::session_service_client::*` → `use ybos_proto::l0::session_service_client::*`)
- Workspace `Cargo.toml`: adăugă `"proto"` la members

After extraction:
- DELETE `l0/proto/l0.proto` (moved)
- DELETE `orchestrator/proto/orchestrator.proto` (moved)
- DELETE `l0/build.rs` și `orchestrator/build.rs` IF they only handled proto codegen; păstrează dacă au alte purpose
- L0 SACRED files NU atinse (`l0/src/main.rs`, `l0/src/identity/*`)

#### B. Y4 LocalLlama carry-over fixes (`inference/` crate)

1. `inference/src/types.rs` — FinishReason cu payloads:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FinishReason {
    Stop,
    MaxTokens,
    StopSequence(String),  // which stop sequence matched
    Error(String),         // error message
}
```
   - Drop `Eq, Copy` derive (String nu e Eq/Copy). Păstrează `PartialEq` (String e PartialEq).

2. `inference/src/local_llama.rs`:
   - **Wrap `complete()` body în `tokio::task::spawn_blocking`** — toată munca llama-cpp-2 trebuie să ruleze pe blocking thread pool, nu pe tokio worker. Pattern:
   ```rust
   async fn complete(&self, req: CompleteRequest) -> Result<CompleteResponse, InferenceError> {
       let model = self.model.clone();
       let backend = self.backend.clone();
       let context_size = self.params.context_size;
       let n_threads = self.params.n_threads;
       tokio::task::spawn_blocking(move || {
           // ...all the existing sync logic, returning Result<CompleteResponse, InferenceError>
       })
       .await
       .map_err(|e| InferenceError::Generation(format!("spawn_blocking failed: {}", e)))?
   }
   ```
   - Update calls to use new `FinishReason::StopSequence(stop_seq.clone())` și `FinishReason::Error(msg)` în loc de unit variants.
   - **Read `model_name` din GGUF metadata dacă fezabil**: cercetează llama-cpp-2 0.1.146 API pentru `LlamaModel::meta_val_str(model, key)` sau echivalent; cheia GGUF standard e `"general.name"`. Dacă API expune, folosește; altfel fallback la `"GGUF Model (path: <basename>)"`.

3. `inference/src/mock.rs`:
   - Update calls to `FinishReason::StopSequence`/`Error` cu payload-uri appropriate (e.g. error message "no canned").

4. `inference/src/remote_api.rs`:
   - Update calls dacă există. (probabil doar `NotImplemented`, neaffected)

5. `inference/tests/mock_smoke.rs`:
   - Update orice `assert_eq!(..., FinishReason::MaxTokens)` continuă să meargă (unit variant); pentru `StopSequence` folosește `matches!` pattern.

#### C. Orchestrator ⇌ Inference integration

1. **`orchestrator/Cargo.toml`**: add `ybos-inference = { path = "../inference" }` (default features, no `local_llama` — orchestrator core uses trait, MockInference în tests).

2. **`orchestrator/src/manifest.rs`** — extend `Capabilities`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Capabilities {
    #[serde(default)]
    pub net_domains: Vec<String>,
    #[serde(default)]
    pub fs_paths: Vec<PathBuf>,
    #[serde(default)]
    pub data_types: Vec<String>,
    #[serde(default)]
    pub data_user_prefs: AccessLevel,
    #[serde(default)]
    pub llm: bool,                          // NEW
}
```

3. **`orchestrator/src/capability.rs`** — extend `Operation`:
```rust
pub enum Operation {
    NetConnect(String),
    FsRead(PathBuf),
    FsWrite(PathBuf),
    UserContextRead,
    UserContextWrite,
    LlmCall,                                // NEW
}
```
   - `enforce` pentru `LlmCall`: returns `Ok` dacă `manifest.capabilities.llm == true`, altfel `Err(CapabilityDenied("LlmCall"))`.

4. **`orchestrator/src/agent.rs`** — adăugă `AgentContext` + modifică Agent trait:
```rust
use std::sync::Arc;
use ybos_inference::Inference;

#[derive(Clone)]
pub struct AgentContext {
    pub inference: Arc<dyn Inference>,
    // Future: pub memory: Arc<dyn Memory>, pub http: Arc<dyn HttpClient>, etc.
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn manifest(&self) -> &Manifest;
    async fn invoke(&self, call: AgentCall, ctx: &AgentContext) -> Result<AgentResponse>;
}
```
   **BREAKING change** — all Agent implementors must update `invoke` signature.

5. **`orchestrator/src/runtime.rs`** — thread `AgentContext` through:
   - `InProcessRuntime::new(registry: Arc<AgentRegistry>, context: AgentContext)` — store context in struct
   - `AgentRuntime::invoke(&self, handle, call) -> Result<AgentResponse>` păstrează semnătura, intern pasează `&self.context` la `agent.invoke(call, ctx)`
   - Alternativ: change trait signature to accept `ctx: &AgentContext`; alegem pattern-ul mai puțin perturbator pentru caller.

6. **`orchestrator/src/agents/hello.rs`** — demonstrate LLM use:
   - Modify HelloAgent să aibă opțional `use_llm: bool` în constructor
   - If `use_llm == true`, `invoke` apelează `ctx.inference.complete(...)` cu un prompt simplu și returnează rezultatul concatenat cu "hello from {name}"
   - Manifestul cu `use_llm=true` declară `llm: true` în Capabilities
   - Default constructor `new(name, manifest=None)` keeps `use_llm = false` (no Inference dependency at default)

7. **`orchestrator/src/main.rs`** (binary):
   - Construct `MockInference` ca placeholder default
   - Build `AgentContext { inference: Arc::new(mock) }`
   - Instantiate `InProcessRuntime::new(registry, ctx)`
   - (Daemon loop rămâne minimal — ca în Y3)

8. **`orchestrator/tests/end_to_end.rs`** — update + adăugă scenariu nou:
   - Update existing test: pass `AgentContext` to runtime constructor
   - New test `test_agent_with_llm_capability`:
     - Build MockInference cu canned `"42"`
     - HelloAgent cu `use_llm = true`, manifest declares `llm = true`
     - Agent invoked → response contains both "hello from..." și "42"
   - New test `test_agent_without_llm_capability_denied`:
     - Use `capability::enforce(manifest, &Operation::LlmCall)` direct
     - Manifest with `llm = false` returns `CapabilityDenied`
     - Manifest with `llm = true` returns Ok

#### D. CI

- Existing jobs adapt:
  - `Build & Test Workspace`: `cargo test --workspace --features ybos-l0/dev_test_init` rămâne; va testa și `ybos-proto`
  - `Cross-compile l0 for Android`: rămâne `cross build -p ybos-l0 ...`; verifică că ybos-proto se cross-compilează corect ca dep tranzitivă a l0
  - `Build & Test inference (mock)`: rămâne `cargo test -p ybos-inference`
  - `LocalLlama smoke test`: rămâne
  - `ShellCheck`: rămâne
- No new CI jobs necesare în Y5 (toate testele acoperite de workspace job)
- IMPORTANT: după extract `ybos-proto`, verifică că `Build & Test Workspace` runtime NU crește semnificativ (proto codegen e mecanic, nu duce mult timp)

#### E. Documentation

- NU update YBOSClaude.md, ARCHITECTURE.md, ROADMAP.md (out of scope — Lead Dev face dacă necesar în review post-merge)
- `proto/README.md` (NEW, scurt): explică ce e ybos-proto, cum se folosește din alte crates
- Update `inference/README.md` minor IF necessary pentru FinishReason payloads (probabil OK ca-i)

### Acceptance criteria Y5

- [ ] `proto/` workspace member creat, `cargo build -p ybos-proto` verde
- [ ] `l0/proto/` și `orchestrator/proto/` directories ȘTERSE (proto-urile mutate în `proto/proto/`)
- [ ] `l0/build.rs` și `orchestrator/build.rs` ȘTERSE sau goale (codegen acum în ybos-proto)
- [ ] l0 + orchestrator consumă `ybos-proto` ca dep, ZERO `tonic-build` în build-deps
- [ ] `cargo test --workspace --features ybos-l0/dev_test_init` verde (Y1 tests + Y3 tests + Y4 tests + Y5 new tests)
- [ ] `LocalLlama::complete()` wrap-uit în `spawn_blocking` — verificabil prin code inspection (nu poate fi acoperit doar prin teste)
- [ ] `FinishReason::StopSequence(String)` și `Error(String)` carry payloads — tests actualizate
- [ ] `Capabilities.llm: bool` adăugat, `Operation::LlmCall` adăugat, enforce funcționează
- [ ] `AgentContext { inference: Arc<dyn Inference> }` creat
- [ ] `Agent::invoke(call, ctx)` BREAKING change aplicat consistent
- [ ] HelloAgent cu `use_llm` opt actualizat
- [ ] 2 noi teste end-to-end: agent cu LLM (mock) reușește; agent fără LLM capability primește `CapabilityDenied`
- [ ] Zero modificări în `l0/src/main.rs`, `l0/src/identity/**` (L0 SACRED preserved)
- [ ] Zero modificări în `docs/`, `YBOSClaude.md`, `README.md` root, `reference/`, `platform/`, `Cross.toml`
- [ ] CI: all 5 jobs existing verzi după Y5 changes
- [ ] `proto/README.md` documentează scopul crate-ului

### Ce NU intra în Y5

- LLM judge sub-agent (Privacy Firewall Layer 3) — Y9
- Vector store integration — fază separată (Y6+)
- Persistent agent memory — fază separată
- Tool calling / function calling API — fază separată
- Real RemoteAPI impl (Anthropic / OpenAI calls) — Y15 cloud burst activation
- Streaming inference exposed via orchestrator gRPC API — Y5 doar wire-up trait-level
- Process isolation (SubprocessRuntime impl real)
- Capability path normalization — Y7 firewall hardening
- Multi-agent collaboration (one agent calls another via L1 mediator) — fază separată

---

## Y6+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Semne de întrebare doar unde **chiar afectează faza activă (Y5)**.

- **Vector store** (sqlite-vec sau qdrant embedded) — pentru memorie semantică per-agent. Independent de Y5.
- **Agent seed: Calendar** — primul agent end-to-end demo cu LLM tools (consumă Inference + Vector store).
- **Agent seed: News Digest**.
- **Privacy firewall Layer 1 (capabilities)** — Y3 + Y5 livrat schelet; Y7 hardenizează: path normalization, audit log, UI, enforcement pe toate operațiile.
- **Privacy firewall Layer 2 (eBPF redactor)**.
- **Privacy firewall Layer 3 (LLM judge)** — folosește Inference (sub-model mic via orchestrator stack din Y5).
- **Agent seed: Trip Planner**.
- **Agent seed: Market Intel**.
- **Agent seed: Learning Curator**.
- **Agent Builder Framework** — template `agents/_template/` + LLM-assisted configurator (folosește Inference + AgentContext din Y5).
- **User-Context Memory subsystem** — storage + sync + capability `data.user_prefs` (e deja declarat în Capabilities Y3; Y5 lasă AgentContext extensibil pentru a injecta și memory handle).
- **Laptop Companion (Tauri)** — pairing QR/NFC + session crypto + task offload + cache sync.
- **UI native YBOS mobile**.
- **Cross-device extins** (multi-phone, tabletă) — post-MVP.
- **Cloud burst activation** — v0.2+; activează RemoteAPI cu API key real.
- **VM Mode (Tier 1) laptop** — Linux VM minim, GPU passthrough, SEV-SNP/TDX integration. Research, post-MVP.
- **Split inference layer-by-layer** ❓ research item. Independent.
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
