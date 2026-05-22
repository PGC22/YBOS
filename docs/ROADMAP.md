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
- Capability path normalization absent — **adresat în Y7 firewall hardening**

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
- Context recreate per `complete()` call — deferat, optimization (NU în Y7)
- `Token.logprob: None` mereu — deferat
- Token text format inconsistency Mock vs LocalLlama — **adresat în Y6** ✅

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
- Token format Mock prefixează `" "` manual — **adresat în Y6** ✅
- `[build-dependencies]` empty sections — **adresat în Y6** ✅
- `proto/README.md` enrich — **adresat în Y6** ✅
- Mock stop sequence NU enforce — deferat (Mock e doar pentru teste)

---

## Y6 — Memory layer (vector store + embedder + orchestrator integration) ✅ Done (PR #7 merged)

- Crate nou `memory/` (ybos-memory)
- `VectorStore` trait (insert/insert_batch/query_top_k/delete/count) + `Embedder` trait (embed/embed_batch/dimension/model_info)
- Implementări:
  - `MockVectorStore` (in-memory + cosine similarity linear scan)
  - `SqliteVecStore` (rusqlite + sqlite-vec extension cu `sqlite3_auto_extension` registration via `Once`)
  - `MockEmbedder` (SHA-256 chain deterministic)
  - `FastEmbedEmbedder` (fastembed v4 + BGE-small-en-v1.5)
- Orchestrator integration: `AgentContext.memory + embedder`, `Capabilities.memory: MemoryAccess` (None/Read/ReadWrite), `Operation::MemoryRead/MemoryWrite` enforce
- HelloAgent `new_with_memory` cu demo round-trip
- 5 end-to-end tests + 2 noi memory tests + token format alignment Y5 carry-over + cosmetic cleanups

**Known carry-overs Y6**:
- `SqliteVecStore` query path: `Uuid::from_slice(...).unwrap()` și `serde_json::from_str(...).unwrap()` — **adresat în Y7**
- `SqliteVecStore` score: `1.0 - distance` doc comment ambiguu (L2 vs cosine) — **adresat în Y7**
- `MockVectorStore` `.unwrap()` pe RwLock (vs `.expect()`) — **adresat în Y7**
- `MockEmbedder.model_name: "mock-bge-small"` misleading (e SHA hash, nu BGE) — **adresat în Y7**
- `FastEmbedEmbedder` `model_name` param ignorat — **adresat în Y7**
- `HelloAgent` memory flow embed-uiește textul de 2 ori (insert + query) — **adresat în Y7**
- Metadata pre-filtering în VectorQuery — deferat
- Cross-compile memory pentru aarch64-android — deferat (Y4.b family)

---

## Y7 — Privacy Firewall Layer 1 hardening + carry-overs cleanup ⭐ NEXT

> Decizii agreate (2026-05-22 sesiune Y7):
> - **Scope Y7** = hardening Layer 1 (path normalization + audit log) + bundle carry-overs Y6 (+ unele Y4/Y5/Y3) într-un singur PR coerent.
> - **Path normalization** = crate extern `path-clean` (well-tested lexical normalizer cu `..` handling). NU duplicăm logica din `l0/src/identity/paths.rs::normalize_lexical` (L0 SACRED).
> - **Audit log** = tracing structured events cu target `ybos.audit`, fields `agent`, `op`, `outcome`, `reason`. Allow = `tracing::info!`, Deny = `tracing::warn!`. Capturat în teste via `tracing-test` crate.
> - **UI vizualizare capabilities** = deferat la fază UI dedicată (Y... când avem UI framework decis).
> - **Enforcement consistent pe toate operațiile** = audit log îmbunătățește vizibilitatea, dar enforcement-ul per-op rămâne by-convention la agent (Rust nu suportă aspect-oriented). Documentat clar în comments + cookbook.

### Scope Y7

#### A. Privacy Firewall Layer 1 hardening (orchestrator)

1. **Path normalization** în `orchestrator/src/capability.rs`:
   - Dep nou: `path-clean = "1"` (cea mai recentă versiune stabilă pe crates.io)
   - În arms-urile `FsRead(path)` și `FsWrite(path)` ale `enforce`:
     - Normalize lexical requested `path` via `path_clean::clean(path)`
     - Normalize lexical fiecare `declared_path` din `manifest.capabilities.fs_paths` similar
     - Aplică `path.starts_with(declared)` PE versiunile normalizate
   - Asigură că `..` resolves înainte de `starts_with` check
   - Documentează în code comment WHY: prevent `FsRead("/data/agent/../../etc/passwd")` bypass

2. **Audit log** în `orchestrator/src/capability.rs`:
   - Toate `enforce` calls emit tracing event structured:
     ```rust
     tracing::info!(
         target: "ybos.audit",
         agent = %manifest.name,
         op = ?op,
         outcome = "allow",
         "Capability check"
     );
     // sau pe deny:
     tracing::warn!(
         target: "ybos.audit",
         agent = %manifest.name,
         op = ?op,
         outcome = "deny",
         reason = %reason,
         "Capability check denied"
     );
     ```
   - `manifest.name` displayed (% format), `op` formated debug (? format)
   - Niciun secret nu apare în log (`Operation::FsRead(path)` arată path-ul declarat — OK pentru audit, NU privacy leak)

3. **Test path normalization** în `orchestrator/tests/end_to_end.rs` (sau `orchestrator/tests/capability_hardening.rs` separate):
   - Agent declară `fs_paths = ["/data/agent/"]`
   - `enforce(manifest, FsRead("/data/agent/data.txt"))` → Ok
   - `enforce(manifest, FsRead("/data/agent/../../etc/passwd"))` → Err (DENIED) ✅
   - `enforce(manifest, FsRead("/data/agent/./sub/../file.txt"))` → Ok (resolves to `/data/agent/file.txt`)
   - Cu declared `fs_paths = ["/data/agent/../sub/"]` → normalize → declared devine `/data/sub/`

4. **Test audit log capture** (via `tracing-test` crate ca dev-dep):
   - Add `tracing-test = "0.2"` în `[dev-dependencies]` orchestrator
   - Test cu `#[traced_test]` decorator
   - După `enforce()` allow → assert `logs_contain("outcome=\"allow\"")` (sau echivalent în formatul real tracing-test)
   - După `enforce()` deny → assert `logs_contain("outcome=\"deny\"")` și conține `agent=...`

#### B. Carry-overs Y6 cleanup

1. **`memory/src/sqlite_vec_store.rs`**:
   - Înlocuiește `Uuid::from_slice(&id_bytes).unwrap()` cu `.map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Blob, Box::new(e)))?` SAU `.map_err(|e| MemoryError::Storage(format!("invalid UUID in DB: {}", e)))?` (alegerea Jules pentru ce e mai compatibil cu closure de la query_map)
   - Înlocuiește `serde_json::from_str(&metadata_str).unwrap()` similar (fallback la `serde_json::Value::Null` la eroare, SAU propagează ca Storage error)
   - **Comment clarification** pe `score: 1.0 - distance`: explică în code comment că sqlite-vec returnează L2 distance, iar transformarea `1 - distance` produce un score "higher is better" care aproximează cosine similarity DOAR pentru embedding-uri unit-normalized (cum sunt BGE outputs). Pentru embeddings ne-normalizate, scor-ul poate fi negativ sau >1. Documentat.

2. **`memory/src/mock_store.rs`**:
   - Înlocuiește toate `.unwrap()` pe RwLock cu `.expect("MockVectorStore: lock poisoned")` (descriptive messages, similar pattern Y3 follow-up)

3. **`memory/src/mock_embedder.rs`**:
   - Schimbă `model_name: "mock-bge-small"` în `model_name: "mock-deterministic-sha256"` (mai exact ce este)

4. **`memory/src/fastembed_embedder.rs`**:
   - Elimină param `model_name: Option<String>` din `FastEmbedEmbedder::load(...)` — currently silently ignored
   - Simplifică semnătura: `pub fn load(cache_dir: Option<PathBuf>) -> Result<Self, MemoryError>`
   - Update doc comment: "Currently locked to BAAI/bge-small-en-v1.5. Future versions may expose model selection."
   - Update call sites (orchestrator/main.rs, memory/tests/fastembed_smoke.rs) — sau dacă nu există apeluri cu model_name explicit (probabil n-au), nu e modificare în consumatori

5. **`orchestrator/src/agents/hello.rs`** — optimize memory flow:
   - În `invoke` cu `text_to_remember.is_some()`: stochează embedding-ul în variabilă, reuse pentru query
   ```rust
   let embedding = ctx.embedder.embed(text).await?;
   ctx.memory.insert(VectorItem {
       embedding: embedding.clone(), // sau direct fără clone dacă possible
       text: text.clone(),
       metadata: json!({"agent": self.manifest.name}),
   }).await?;
   // ... enforce read ...
   // Reuse embedding pentru query (nu re-embed):
   let matches = ctx.memory.query_top_k(
       VectorQuery { embedding },
       1,
   ).await?;
   ```

### Acceptance criteria Y7

- [ ] `path-clean` dep adăugat în orchestrator/Cargo.toml
- [ ] `capability::enforce` normalizes `FsRead`/`FsWrite` paths și fs_paths declared via `path_clean::clean`
- [ ] `..` bypass test: declared `/data/agent/`, requested `/data/agent/../../etc/passwd` → DENY
- [ ] Audit log: toate enforce() calls emit tracing event cu target `ybos.audit`, fields `agent`, `op`, `outcome`, optional `reason`
- [ ] `tracing-test` dep adăugat în [dev-dependencies] orchestrator
- [ ] Test audit log capture: assert logs after allow + after deny
- [ ] Y6 carry-overs:
  - [ ] `SqliteVecStore` unwraps în query path eliminate (propagate as MemoryError)
  - [ ] `SqliteVecStore` score doc comment clarifică L2 vs cosine
  - [ ] `MockVectorStore` `.unwrap()` → `.expect()` messages descriptive
  - [ ] `MockEmbedder.model_name` cosmetic update
  - [ ] `FastEmbedEmbedder::load` simplified signature (drop model_name param)
  - [ ] `HelloAgent` memory flow embed-uiește o singură dată (reuse pentru query)
- [ ] Zero modificări în `l0/src/main.rs`, `l0/src/identity/**` (L0 SACRED preserved)
- [ ] Zero modificări în `docs/`, `YBOSClaude.md`, `README.md` root, `reference/`, `platform/`, `Cross.toml`
- [ ] Zero modificări în `l0/src/grpc/**`, `l0/src/{lib,hw,bus,reflex}/**`
- [ ] Zero modificări în `proto/**`
- [ ] Zero modificări în `inference/**` (NU touch Y4 deliverables; Y6 deja a abordat token format)
- [ ] Zero modificări în `memory/src/{lib,trait_def,types}.rs` (Y6 deliverables; doar `mock_store.rs`, `sqlite_vec_store.rs`, `mock_embedder.rs`, `fastembed_embedder.rs` modificate)
- [ ] `cargo test --workspace --features ybos-l0/dev_test_init` verde
- [ ] All 7 existing CI jobs verzi (no new jobs needed)

### Ce NU intra în Y7

- News Digest agent (primul seed agent) — fază următoare
- Calendar agent — fază următoare
- User-Context Memory subsystem — fază separată
- UI vizualizare capabilities — depinde de UI framework, fază UI dedicată
- Privacy Firewall Layer 2 (eBPF redactor) — kernel-level, blocked pe Linux dev env
- Privacy Firewall Layer 3 (LLM judge) — fază separată
- Agent Builder Framework — fază separată
- Capability enforce auto-applied pe toate ops (aspect-oriented) — Rust nu suportă cleanly; rămâne by-convention
- Context pool optimization LocalLlama — deferat
- Metadata pre-filtering în VectorQuery — deferat

---

## Y8+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Semne de întrebare doar unde **chiar afectează faza activă (Y7)**.

- **Agent seed: News Digest** — primul agent end-to-end cu LLM + Vector store. Folosește Y4 + Y6 + Y7 (capability enforcement hardenized).
- **Agent seed: Calendar** — Google Calendar OAuth + LLM tool calling.
- **User-Context Memory subsystem** — storage + sync, va folosi memory crate Y6 ca backend; capability `data.user_prefs` deja declarată în Y3.
- **Privacy firewall Layer 2 (eBPF redactor)** — kernel-level, blocked pe Linux dev env.
- **Privacy firewall Layer 3 (LLM judge)** — folosește Inference stack.
- **Agent seed: Trip Planner**.
- **Agent seed: Market Intel**.
- **Agent seed: Learning Curator**.
- **Agent Builder Framework** — template + LLM-assisted configurator (folosește Inference + AgentContext + Memory).
- **Laptop Companion (Tauri)** — pairing QR/NFC + session crypto + task offload.
- **UI native YBOS mobile** — launcher, onboarding wizard, agent dashboards, capability visualization (audit log viewer).
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
