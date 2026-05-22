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

## Y7 — Privacy Firewall Layer 1 hardening + carry-overs cleanup ✅ Done (PR #8 merged)

- `path-clean` crate adăugat în orchestrator deps
- `capability::enforce` normalizes `FsRead`/`FsWrite` paths și fs_paths declared via `path_clean::clean`; `..` bypass blocat
- Audit log: toate enforce() calls emit tracing event structured cu target `ybos.audit`, fields `agent`/`op`/`outcome`/`reason`; allow = info, deny = warn
- Audit log tests via custom `Layer` în `#[cfg(test)] mod tests` din `capability.rs` (drop tracing-test 0.2 — unreliable cu target custom)
- Y6 carry-overs cleanup: SqliteVecStore unwraps eliminate + score doc clarification, MockVectorStore RwLock messages, MockEmbedder cosmetic, FastEmbedEmbedder load() simplified, HelloAgent memory embed-once

**Known carry-overs Y7**:
- `SqliteVecStore` Mutex `.unwrap()` la lock acquire (line 130, 191) — deferat
- Capability enforce by-convention (nu aspect-oriented) — Rust nu suportă cleanly, documentat
- Context pool LocalLlama — deferat
- UI vizualizare capabilities — Y... fază UI dedicată

---

## Y8 — News Digest seed agent + minimal HTTP client + RSS parser (hand-rolled) ⭐ NEXT

> Decizii agreate (2026-05-22 sesiune Y8):
> - **Primul seed agent** = News Digest (folosește Y4 inference + Y6 memory + Y7 capability). NU Calendar (necesită Google OAuth — fază viitoare).
> - **HTTP client** = `hyper-util` + `hyper-tls` direct (NU `reqwest` heavy deps; YBOS privacy/control-first philosophy). Trait `HttpClient` cu impl `HyperHttpClient` + `MockHttpClient`.
> - **XML parser** = hand-rolled de la zero (lexer + parser, NU `quick-xml` sau alt crate XML). RSS 2.0 + minim Atom 1.0 subset. Big scope, dar aligned cu YBOS minimal-deps principle.
> - **Real RSS feed smoke** = behind feature flag `real_rss`, descarcă din feed public stabil (e.g. BBC, Reuters). Mocks default.
> - **Carry-over Y7** inclus: `SqliteVecStore` Mutex `.unwrap()` → `.expect()` messages descriptive (small, low-risk fix).

> ⚠️ **Avertisment scope**: hand-rolled XML lexer + parser + RSS layer + hyper direct + News agent integration = PR potențial mare (~1000-1500 linii). Dacă Jules detectează că scope devine ne-livrabil într-un singur PR, va flag în PR description și vom splita Y8 în:
> - Y8.a = HTTP client + XML parser + RSS interpreter (infrastructure)
> - Y8.b = News agent integration (consumă Y8.a)

### Scope Y8

#### A. HTTP client (`orchestrator/src/http.rs` — general purpose, reusable de viitori agenți)

1. Trait `HttpClient`:
   ```rust
   #[async_trait]
   pub trait HttpClient: Send + Sync {
       async fn get(&self, url: &str) -> Result<HttpResponse, HttpError>;
   }

   pub struct HttpResponse {
       pub status: u16,
       pub headers: Vec<(String, String)>,
       pub body: Vec<u8>,
   }

   #[derive(Debug, thiserror::Error)]
   pub enum HttpError {
       InvalidUrl, Network, Tls, Status(u16), Body, Timeout
   }
   ```

2. `HyperHttpClient` impl via `hyper-util` + `hyper-tls`:
   - HttpsConnector via hyper-tls (rustls backend dacă disponibil în hyper-tls 0.6+)
   - Client builder cu reasonable defaults (15s timeout, 10MB max body)
   - Body collected în-memory (NU streaming pentru Y8; agenții lucrează cu documente întregi)
   - Redirect handling (max 5 redirects)
   - User-Agent: `"YBOS/0.1 (+https://github.com/PGC22/YBOS)"`

3. `MockHttpClient` (cfg-gated `#[cfg(test)]` sau în `src/`):
   - Constructor primește `Vec<(url_pattern, HttpResponse)>` canned
   - `get(url)` returnează primul match sau eroare

4. Adăugare în `AgentContext`:
   ```rust
   pub struct AgentContext {
       pub inference: Arc<dyn Inference>,
       pub memory: Arc<dyn VectorStore>,
       pub embedder: Arc<dyn Embedder>,
       pub http: Arc<dyn HttpClient>,  // NEW
   }
   ```
   - Trait Agent::invoke signature unchanged
   - Capability nouă: `Capabilities.net: bool` SAU folosim existing `net_domains: Vec<String>` și adăugăm enforcement la nivel de HttpClient wrapper (recommended — net_domains deja există)
   - Decizie: enforcement la nivel de agent — agent verifică `capability::enforce(.., Operation::NetConnect(domain))` înainte de fiecare HTTP call. Pattern by-convention, consistent cu LLM/Memory.

#### B. XML lexer + parser hand-rolled (`orchestrator/src/news/xml.rs`)

Big scope. Documentat clar.

1. **Lexer** (`Tokenizer`):
   - Input: `&str` sau `&[u8]` (recomandat &str cu UTF-8 validation upfront)
   - Tokens: `StartTag { name, attrs }`, `EndTag { name }`, `Text(String)`, `CData(String)`, `Comment`, `ProcessingInstruction`, `EntityRef(String)`, `Whitespace`, `Eof`
   - Handle entity references: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`, `&#nnn;`, `&#xHH;`
   - Handle CDATA sections `<![CDATA[...]]>`
   - Handle comments `<!-- ... -->`
   - Handle processing instructions `<?xml version="1.0"?>` (skip-able)
   - Handle whitespace properly (preserve significant whitespace, trim insignificant)
   - Errors: `XmlLexerError` enum (UnexpectedEof, InvalidEntity, MalformedTag, etc.)

2. **Parser** (`parse_document`):
   - Input: token stream
   - Output: `XmlNode` tree structure
   ```rust
   pub enum XmlNode {
       Element { name: String, attrs: HashMap<String, String>, children: Vec<XmlNode> },
       Text(String),
       CData(String),
   }
   ```
   - Tag matching (open vs close)
   - Nested elements
   - Attribute parsing (`key="value"` or `key='value'`)
   - Error: `XmlParseError`

3. **Tests pentru lexer + parser**:
   - Fixture-based: ~10 XML samples (well-formed RSS, well-formed Atom, malformed cases)
   - Unit test fiecare token type
   - Unit test entity decoding
   - Unit test CDATA
   - Round-trip: parse → serialize back → reparse → same tree

#### C. RSS interpretation layer (`orchestrator/src/news/rss.rs`)

1. Walks `XmlNode` tree to extract:
   ```rust
   pub struct RssChannel {
       pub title: String,
       pub link: String,
       pub description: String,
       pub items: Vec<RssItem>,
   }

   pub struct RssItem {
       pub title: String,
       pub link: String,
       pub description: String,
       pub pub_date: Option<String>,   // RFC 822 string; parsed Y8.c sau later
       pub guid: Option<String>,
   }
   ```

2. Support RSS 2.0 (channel > item structure)

3. **Minim Atom 1.0** support (optional, can defer): Atom uses `<feed><entry>` instead of `<channel><item>`. Document if not implemented.

4. Function `parse_rss(xml: &str) -> Result<RssChannel, RssError>`

#### D. NewsAgent (`orchestrator/src/agents/news.rs`)

1. `NewsAgent` struct:
   ```rust
   pub struct NewsAgent {
       manifest: Manifest,
       sources: Vec<String>,  // RSS URLs whitelisted
   }
   ```

2. Constructor `NewsAgent::new(name: &str, sources: Vec<String>) -> Self`:
   - Manifest declarations:
     - `net_domains`: derive from `sources` (extract hostname din each URL)
     - `llm: true`
     - `memory: ReadWrite`

3. AgentCall methods (via `call.method`):
   - `"fetch"`: pentru fiecare `source`, `capability::enforce(.., NetConnect(host))` → `ctx.http.get(url)` → parse RSS → embed fiecare `RssItem` → insert în memory cu metadata `{"source": url, "type": "news", "fetched_at": now}`
   - `"summarize"`: query top-K recent items din memory → format ca prompt → `ctx.inference.complete()` → return summary
   - `"query"`: payload e a text query → embed → `ctx.memory.query_top_k(.., 5)` → return matches

4. Capability enforce înainte de fiecare op (consistent cu HelloAgent pattern)

5. Documentat clar în comments că sources e whitelist hardcoded (production: user-configurable via UI / settings, deferred)

#### E. End-to-end tests (`orchestrator/tests/news_e2e.rs`)

1. **Mock test**:
   - Setup MockHttpClient cu canned RSS XML response
   - Setup MockInference + MockVectorStore + MockEmbedder
   - Register NewsAgent
   - Call `fetch` → assert items inserted în memory
   - Call `summarize` → assert response non-empty
   - Call `query("...")` → assert matches returned

2. **Real RSS smoke** (`#[cfg(feature = "real_rss")]`):
   - Use HyperHttpClient (real)
   - Fetch din feed public stabil (suggestion: `https://feeds.bbci.co.uk/news/world/rss.xml` sau `https://www.reuters.com/world/rss` — Jules alege unul reliable după inspecție live)
   - Parse + insert în SqliteVecStore (cu fastembed feature)
   - Assert at least 1 item parsed successfully

#### F. CI updates (`.github/workflows/ci.yml`)

- Existing jobs adaptați (workspace build acum include hyper deps — verifică că `Build & Test Workspace` rămâne sub 3 min)
- NEW job: `News Digest smoke (real RSS)` — `cargo test -p ybos-orchestrator --features real_rss,fastembed,sqlite_vec --test news_e2e -- --include-ignored` sau echivalent
  - Timeout 5 min
  - Continue-on-error: false (failure blocks merge; dar dacă feed-ul public e down, jobul reia)
- ShellCheck rămâne

#### G. Carry-over Y7

1. `memory/src/sqlite_vec_store.rs`: înlocuiește `conn.lock().unwrap()` (linia 130, 191) cu `.expect("SqliteVecStore: connection lock poisoned")`. Consistent cu pattern-ul Y3/Y6/Y7.

### Acceptance criteria Y8

- [ ] `orchestrator/src/http.rs` cu trait `HttpClient` + `HyperHttpClient` + `MockHttpClient`
- [ ] `orchestrator/src/news/xml.rs` cu lexer + parser hand-rolled, fixture-based tests
- [ ] `orchestrator/src/news/rss.rs` cu RSS 2.0 interpretation; Atom 1.0 best-effort sau documented gap
- [ ] `orchestrator/src/agents/news.rs` cu NewsAgent + 3 methods (fetch/summarize/query)
- [ ] `AgentContext` extins cu `http: Arc<dyn HttpClient>`
- [ ] `orchestrator/src/main.rs` instantiate `HyperHttpClient` în default context
- [ ] Mock e2e test pass (no network)
- [ ] Real RSS smoke test pass în CI cu feature `real_rss`
- [ ] Carry-over Y7: `SqliteVecStore` Mutex unwraps → expects
- [ ] Zero modificări în `l0/`, `proto/`, `inference/`, `memory/src/{lib,trait_def,types,mock_*,fastembed_*}.rs` (doar sqlite_vec_store.rs pentru carry-over)
- [ ] Zero modificări în `docs/`, `YBOSClaude.md`, `README.md` root, `reference/`, `platform/`, `Cross.toml`
- [ ] `Build & Test Workspace` runtime sub 3 min (hyper compile acceptable; dacă crește semnificativ, flag)
- [ ] All existing CI jobs verzi + 1 nou (real_rss)

### Ce NU intra în Y8

- Calendar agent (necesită Google OAuth — fază viitoare, mai complex)
- User-Context Memory subsystem
- Privacy Firewall Layer 2/3
- Cross-compile orchestrator pentru aarch64 (hyper + custom XML need cross-compile validation — deferred până device disponibil)
- Background daemon scheduling (RSS auto-fetch la interval) — NewsAgent e on-demand
- UI / push notifications pentru morning brief
- Multi-source dedup (RSS items care apar în multiple feeds) — Y8 doar inserează, dedup deferred
- Agent Builder Framework
- Real OAuth providers

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

## Y9+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Semne de întrebare doar unde **chiar afectează faza activă (Y8)**.

- **Agent seed: Calendar** — Google Calendar OAuth + LLM tool calling. Folosește Y8 HTTP client + Y4 inference + Y6 memory.
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
