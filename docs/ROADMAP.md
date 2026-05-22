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

---

## Y1 — L0 generalizare ✅ Done (PR #1 merged)

- Identity generalization (struct generic, UUID v4, zero hardcoded owner)
- Onboarding state machine + Argon2id envelope A + BIP39 24-word display + HMAC-signed identity blob
- Session token issuance API (HKDF + scope + revoke + list)
- L0 SACRED tripwire pe layout `${YBOS_DATA}/identity/`
- Trait stubs envelope B (TEE) + C (YubiKey) pentru fază viitoare

**Known carry-over flag**: Envelope A folosește Argon2id-XOR + HMAC tag custom (acceptabil pentru dev scaffold). Înainte de production: înlocuit cu AEAD vetted (AES-GCM-SIV sau ChaCha20-Poly1305).

---

## Y2 — Build environment + cross-compile + AOSP customization scaffolding ✅ Done (PR #2 merged)

- AOSP build host setup + sync workflow + manifest scaffold
- Cross-compile `ybos-l0` aarch64-linux-android via `cross` cu `Cross.toml`
- AOSP overlay device-agnostic (BoardConfig, system.prop, init.ybos.rc, sepolicy)
- FLASH_PROCEDURE.md generic ARM64

**Known carry-overs Y2.b**: capabilities re-evaluation pe device, ADB/verity dev→userdebug overlay split, Cross.toml image pin la stabil, ybos user/group definit.

---

## Y2.b — Flash + boot verification (BLOCKED pe achiziție device)

---

## Y3 — L1 orchestrator skeleton + L0 SessionService gRPC + Cargo workspace ✅ Done (PR #3 + PR #4 cleanup merged)

- Cargo workspace conversion + L0 SessionService gRPC + l0/src/lib.rs pentru testing in-process
- orchestrator crate cu Agent trait + AgentRuntime hybrid + Manifest + Capabilities + capability enforce + AgentRegistry + L0Client + HelloAgent
- End-to-end test orchestrator ⇌ L0
- PR #4 cleanup: revoke_all count, Runtime factory caching, RwLock messages

**Known carry-overs**:
- Proto duplicate compilation — **adresat în Y5 (`ybos-proto` extract)** ✅
- Capability path normalization absent — adresat în Y7 firewall hardening

---

## Y4 — LLM inference layer (skeleton + LocalLlama CPU) ✅ Done (PR #5 merged)

- Crate nou `inference/` (ybos-inference)
- Inference trait `complete()` sync + `complete_stream()` streaming
- MockInference + LocalLlama via llama-cpp-2 + RemoteAPI stub cu SecretString
- CI: Workspace build rapid (no llama în default), inference mock + LocalLlama smoke separate

**Known carry-overs Y4**:
- `LocalLlama::complete()` lipsea `spawn_blocking` — **adresat în Y5** ✅
- `FinishReason::StopSequence/Error` fără payload — **adresat în Y5** ✅
- `model_name` placeholder — **adresat în Y5 (GGUF metadata read)** ✅
- Seed truncat la u32 (API constraint llama-cpp-2) — deferat, requires lib change
- Context recreate per `complete()` call — deferat, optimization
- `Token.logprob: None` mereu — deferat
- **Token text format inconsistency Mock vs LocalLlama** — **adresat în Y6**

---

## Y4.b — NPU acceleration + cross-compile aarch64 (BLOCKED pe device + Y2.b)

---

## Y5 — Orchestrator ⇌ Inference integration + ybos-proto extraction + Y4 carry-over fixes ✅ Done (PR #6 merged)

- `ybos-proto` shared workspace crate (consolidează codegen-ul l0 + orchestrator)
- LocalLlama: `spawn_blocking` wrap pe `complete()`, `FinishReason::StopSequence(String)`/`Error(String)` payloads, model_name din GGUF metadata
- `AgentContext { inference: Arc<dyn Inference> }` + `Agent::invoke(call, ctx)` BREAKING change
- `Capabilities.llm: bool` + `Operation::LlmCall` enforce
- HelloAgent cu `use_llm` option (`new_with_llm`)
- End-to-end tests: agent cu LLM cap reușește; capability denies LlmCall fără declarație

**Known carry-overs Y5**:
- Token format Mock prefixează `" "` manual, LocalLlama emite raw cu spacing intrinsec — **adresat în Y6**
- `[build-dependencies]` empty sections în l0/Cargo.toml + orchestrator/Cargo.toml — **adresat în Y6**
- `proto/README.md` poate fi enrich-uit cu WHY și usage — **adresat în Y6**
- Context pool optimization în LocalLlama — deferat
- Mock stop sequence NU enforce — deferat (Mock e doar pentru teste, nu prod logic)

---

## Y6 — Memory layer (vector store + embedder + orchestrator integration) ⭐ NEXT

> Decizii agreate (2026-05-22 sesiune Y6):
> - **Vector store backend** = `sqlite-vec` (modern înlocuitor pentru sqlite-vss, mature, SQLite-based, embeddable).
> - **Embedder real** = `fastembed` Rust crate (folosește ONNX Runtime + sentence-transformer models, mai ușor de gestionat decât llama-cpp-2 pentru embeddings). Mock embedder furnizat pentru teste.
> - **Workspace location** = crate nou `memory/` (package `ybos-memory`).
> - **Integration pattern** = identic cu Y5 inference: `AgentContext` extins, capability nouă, BREAKING dacă necesar.
> - **Carry-overs Y4+Y5 incluse**: token format alignment + cosmetic cleanups (NU context pool optimization, deferat).

### Scope Y6

#### A. Crate nou `memory/` (ybos-memory)

Layout:
```
memory/
├── Cargo.toml                  # package = ybos-memory; features pentru embedder backends
├── README.md                   # explică crate, features, how-to-run
├── src/
│   ├── lib.rs                  # public re-exports
│   ├── trait_def.rs            # VectorStore + Embedder traits
│   ├── types.rs                # VectorItem, VectorQuery, VectorMatch, MemoryError, etc.
│   ├── mock_store.rs           # MockVectorStore (in-memory HashMap, linear KNN)
│   ├── sqlite_vec_store.rs     # SqliteVecStore via sqlite-vec extension (feature `sqlite_vec`)
│   ├── mock_embedder.rs        # MockEmbedder (deterministic hash → fake vector, default feature)
│   └── fastembed_embedder.rs   # FastEmbedEmbedder via fastembed crate (feature `fastembed`)
└── tests/
    ├── mock_smoke.rs           # mock store + mock embedder, default features
    └── fastembed_smoke.rs      # cfg-gated, descarcă model embedding, query real
```

#### B. Traits

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn insert(&self, item: VectorItem) -> Result<VectorId, MemoryError>;
    async fn insert_batch(&self, items: Vec<VectorItem>) -> Result<Vec<VectorId>, MemoryError>;
    async fn query_top_k(&self, query: VectorQuery, k: usize) -> Result<Vec<VectorMatch>, MemoryError>;
    async fn delete(&self, id: VectorId) -> Result<(), MemoryError>;
    async fn count(&self) -> Result<usize, MemoryError>;
}

#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed(&self, text: &str) -> Result<Vec<f32>, MemoryError>;
    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, MemoryError>;
    fn dimension(&self) -> usize;
    fn model_info(&self) -> EmbedderInfo;
}

pub struct VectorItem {
    pub embedding: Vec<f32>,
    pub text: String,
    pub metadata: serde_json::Value,
}

pub struct VectorQuery {
    pub embedding: Vec<f32>,
    // future extensions: pre-filter prin metadata.
}

pub struct VectorMatch {
    pub id: VectorId,
    pub text: String,
    pub metadata: serde_json::Value,
    pub score: f32, // cosine similarity sau distance — documentat clar care
}

pub type VectorId = uuid::Uuid;

pub struct EmbedderInfo {
    pub backend: String,    // "mock" | "fastembed"
    pub model_name: String,
    pub dimension: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum MemoryError { /* Storage, InvalidEmbedding, NotFound, EmbedderError, ... */ }
```

#### C. Implementations

1. **MockVectorStore** (default feature):
   - In-memory `RwLock<HashMap<VectorId, VectorItem>>`
   - `query_top_k`: linear scan, computes cosine similarity vs all stored, sort, take k. NU production grade, doar test.
   - Useful pentru orchestrator tests without sqlite-vec dep.

2. **SqliteVecStore** (feature `sqlite_vec`):
   - `rusqlite` + `sqlite-vec` extension (loaded at connection setup)
   - Table schema:
     ```sql
     CREATE TABLE IF NOT EXISTS memories (
         id BLOB PRIMARY KEY,
         text TEXT NOT NULL,
         metadata TEXT NOT NULL,
         embedding BLOB NOT NULL
     );
     CREATE VIRTUAL TABLE IF NOT EXISTS memories_vec USING vec0(
         embedding float[<DIM>]
     );
     ```
   - `query_top_k`: foloseste `vec_search` din sqlite-vec
   - Embedding dimension fixat la load — orice insert cu dim diferită → eroare
   - Path: file-based (constructor accept `&Path`) + in-memory option (`SqliteVecStore::in_memory(dim)`)

3. **MockEmbedder** (default feature):
   - `embed(text)`: returneaz `Vec<f32>` deterministic din SHA-256 hash al textului, mapped to f32 range
   - `dimension()`: configurat la `new(dim)`, default 384 (matches BGE-small)
   - Pentru teste: două input identice → embedding identic; inputs diferite → embeddings diferite (predictably)

4. **FastEmbedEmbedder** (feature `fastembed`):
   - Foloseste `fastembed` crate (cea mai recentă versiune stabilă)
   - Default model: `BAAI/bge-small-en-v1.5` (384 dim, ~130MB ONNX download la prima rulare)
   - Constructor: `FastEmbedEmbedder::load(model_name: Option<String>, cache_dir: Option<PathBuf>)`
   - Async wrap peste fastembed's synchronous API (similar pattern cu LocalLlama: `spawn_blocking`)

#### D. Cargo features

`memory/Cargo.toml`:
```toml
[features]
default = ["mock_store", "mock_embedder"]
mock_store = []
mock_embedder = []
sqlite_vec = ["dep:rusqlite", "dep:sqlite-vec"]
fastembed = ["dep:fastembed"]
```

Heavy deps (`rusqlite`, `sqlite-vec`, `fastembed`) optional. Default feature set rămâne lightweight.

Workspace `Cargo.toml` adăugă `"memory"` la members. Adăugă în `workspace.dependencies`:
```toml
ybos-memory = { path = "memory" }
```

#### E. Orchestrator integration

1. **`orchestrator/Cargo.toml`**: add `ybos-memory = { workspace = true }`.

2. **`orchestrator/src/agent.rs`**: extend `AgentContext`:
   ```rust
   #[derive(Clone)]
   pub struct AgentContext {
       pub inference: Arc<dyn Inference>,
       pub memory: Arc<dyn VectorStore>,
       pub embedder: Arc<dyn Embedder>,
   }
   ```
   NU schimb signature `Agent::invoke` (rămâne `invoke(call, ctx)` — context-ul își crește înăuntru, nu trait-ul).

3. **`orchestrator/src/manifest.rs`**: extend `Capabilities`:
   ```rust
   pub struct Capabilities {
       // ... existing ...
       #[serde(default)]
       pub memory: MemoryAccess,   // NEW
   }

   #[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
   #[serde(rename_all = "snake_case")]
   pub enum MemoryAccess {
       #[default]
       None,
       Read,
       ReadWrite,
   }
   ```

4. **`orchestrator/src/capability.rs`**: extend `Operation`:
   ```rust
   pub enum Operation {
       // ... existing ...
       MemoryRead,
       MemoryWrite,
   }
   ```
   `enforce`: MemoryRead → cere `MemoryAccess::Read` sau `ReadWrite`; MemoryWrite → cere `ReadWrite`.

5. **`orchestrator/src/main.rs`**: instantiate MockVectorStore + MockEmbedder default; wrap in `AgentContext`.

6. **`orchestrator/src/agents/hello.rs`**: extend HelloAgent cu opțional `use_memory` constructor:
   - `HelloAgent::new_with_memory(name, text_to_remember)`
   - In `invoke`: dacă `use_memory`, enforce `MemoryRead` + `MemoryWrite`, embed `text_to_remember`, insert în store, query top_k(1) cu același text, return matched text înapoi. Demonstrează round-trip.

7. **`orchestrator/tests/end_to_end.rs`** — adaugă scenarii noi:
   - `test_agent_with_memory_capability`: HelloAgent cu memory + Mock store/embedder → insert + query → verifică round-trip
   - `test_capability_denies_memory_without_declaration`: similar cu LlmCall pattern

#### F. CI updates

- **Existing jobs**: rămân verzi cu memory crate adăugat (default features lightweight, nu compile fastembed/sqlite-vec by default)
- **New job** `Build & Test memory (mock)`: `cargo test -p ybos-memory` — default features, fast
- **New job** `Memory smoke test (fastembed)`: `cargo test -p ybos-memory --features fastembed,sqlite_vec` — cu cache pentru ONNX model
  - Install dependencies: nimic specific (fastembed are ort bundled — verifică Jules)
  - Cache `target/test-models/` (deja există pentru llama)
- **Cross-compile**: rămâne `cross build -p ybos-l0` — memory NU intră în cross-compile Y6 (deferat similar cu inference)

#### G. Carry-over flag fixes incluse în Y6

1. **Token format alignment Mock ⇌ LocalLlama**:
   - `inference/src/mock.rs`: drop manual `" "` prefix la subsequent tokens. Emit raw word (fără leading space).
   - Mock-ul devine consumer-side responsibility de a concatena cu `join("")` sau cu `join(" ")` — alegerea e clară din context.
   - Update `inference/tests/mock_smoke.rs`: schimbă `tokens.join(" ")` în `tokens.join("")` și ajustează assert string ("four five six" → "fourfivesix"? sau folosește newline-separated mock responses?).
   - Alternativă mai curată: configurabil — mock primește un separator în constructor: `MockInference::new_with_separator(responses, separator)`. Default `" "` pentru backward compat. NU recomandat pentru Y6 — adaugă API, mai bine fix bug semantic.
   - **Decizie**: emit raw tokens consistent (LocalLlama style); consumer (test sau real) concatenează cu `""`. Update mock_smoke.rs assertions.

2. **Empty `[build-dependencies]` cleanup**:
   - `l0/Cargo.toml` și `orchestrator/Cargo.toml`: șterge complet sectiunea `[build-dependencies]` (e empty după Y5)

3. **`proto/README.md` enrich**:
   - Adaugă paragraf "Why this crate exists" (avoid duplicate compilation across consumer crates)
   - Adaugă "Consumers" section listing l0 + orchestrator (+ future ones)

### Acceptance criteria Y6

- [ ] `memory/` workspace member creat, `cargo build -p ybos-memory` verde
- [ ] `cargo test --workspace --features ybos-l0/dev_test_init` verde (toate testele anterioare + memory mock tests + 2 noi end-to-end orchestrator tests)
- [ ] `cargo test -p ybos-memory --features fastembed,sqlite_vec` verde în CI (cu cache model)
- [ ] VectorStore trait + MockVectorStore + SqliteVecStore (cfg-gated) implementate
- [ ] Embedder trait + MockEmbedder + FastEmbedEmbedder (cfg-gated) implementate
- [ ] `AgentContext` extins cu `memory` + `embedder`
- [ ] `Capabilities.memory: MemoryAccess` enum (None/Read/ReadWrite) + `Operation::MemoryRead`/`MemoryWrite` + enforce
- [ ] HelloAgent cu `new_with_memory` constructor + demo round-trip
- [ ] 2 noi end-to-end tests: agent cu memory cap reușește; capability denies fără declarație
- [ ] Carry-overs Y4+Y5 incluse: token format Mock alignment, `[build-dependencies]` cleanup, proto/README enrich
- [ ] Zero modificări în `l0/src/main.rs`, `l0/src/identity/**` (L0 SACRED preserved)
- [ ] Zero modificări în `docs/`, `YBOSClaude.md`, `README.md` root, `reference/`, `platform/`, `Cross.toml`
- [ ] All 5 existing CI jobs verzi + 2 jobs noi (memory mock + memory fastembed)
- [ ] `memory/README.md` documentează crate + features + how-to-run

### Ce NU intra în Y6

- Real production use în agenți seed (Calendar / News) — Y7+
- Context pool optimization în LocalLlama — deferat
- Capability path normalization — Y7 firewall hardening
- User-Context Memory subsystem (preferințe persistente, recurrențe) — separat, va folosi memory crate intern
- Cross-compile memory pentru aarch64-android — deferat (mlc-llm/NPU = Y4.b family)
- Metadata pre-filtering în VectorQuery — Y6 doar embedding-based; metadata filtering ulterior

---

## Y7+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Semne de întrebare doar unde **chiar afectează faza activă (Y6)**.

- **Agent seed: News Digest** — primul agent end-to-end cu LLM + Vector store (folosește Y4 + Y6).
- **Agent seed: Calendar** — Google Calendar OAuth + LLM tool calling.
- **Privacy firewall Layer 1 hardenizare** (Y7): path normalization (FsRead/FsWrite cu `..` bypass), audit log, UI vizualizare capabilities, enforcement consistent pe toate operațiile.
- **Privacy firewall Layer 2 (eBPF redactor)**.
- **Privacy firewall Layer 3 (LLM judge)** — folosește Inference stack.
- **Agent seed: Trip Planner**.
- **Agent seed: Market Intel**.
- **Agent seed: Learning Curator**.
- **Agent Builder Framework** — template + LLM-assisted configurator (folosește Inference + AgentContext + Memory).
- **User-Context Memory subsystem** — storage + sync, va folosi memory crate Y6 ca backend.
- **Laptop Companion (Tauri)** — pairing QR/NFC + session crypto + task offload.
- **UI native YBOS mobile**.
- **Cross-device extins** — post-MVP.
- **Cloud burst activation** — v0.2+; activează RemoteAPI cu API key real.
- **VM Mode (Tier 1) laptop** — research, post-MVP.
- **Split inference layer-by-layer** ❓ research item.
- **SubprocessRuntime impl real** — process isolation agenți.

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

Documentul a avut estimări de săptămâni/luni anterior. Au fost scoase intenționat (decizie 2026-05-21 sesiunea 2). Motiv: livrabilitate viabilă (vandabilă) depinde de calitate + acceptanță, nu de un calendar arbitrar.
