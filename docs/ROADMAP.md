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

## Y3 — L1 orchestrator skeleton + L0 SessionService gRPC ⭐ NEXT

> Decizii agreate (2026-05-22 sesiune Y3):
> - **Process model agenți**: hybrid — trait `Agent` + trait `AgentRuntime` cu impl `InProcess` în Y3, design-uit pentru `Subprocess` impl viitor (Android Binder / gRPC) fără refactor major.
> - **Cargo workspace**: da, convertesc root în workspace cu members `[l0, orchestrator]`.
> - **L1 → L0 session wire-up**: real (NU doar placeholder). L0 expune SessionService gRPC nouă, L1 are client real care apelează `IssueToken/RevokeSession/ListActive`.

### Scope Y3

#### A. L0 — expune session token API via gRPC (wrapper peste identity::session existing)

1. **Proto definition**
   - `l0/proto/l0.proto`: adaugă service `SessionService` cu RPC:
     - `IssueToken(IssueTokenRequest) → IssueTokenResponse` (scope + expiry_secs + peer_fingerprint → session_id, key_bytes, expires_at)
     - `RevokeSession(RevokeSessionRequest) → RevokeSessionResponse`
     - `RevokeAll(RevokeAllRequest) → RevokeAllResponse`
     - `ListActive(ListActiveRequest) → ListActiveResponse` (returnează `repeated SessionInfo`)
     - `InitializeForTest(InitializeForTestRequest) → InitializeForTestResponse` — DEV-ONLY RPC pentru testare end-to-end fără onboarding flow real; gated în production cu feature flag `dev_test_init` sau env check.

2. **Implementation**
   - `l0/src/grpc/session_service.rs` (NEW) — implementează tonic service trait, deleagă la `identity::session::{issue_session_token, revoke_session, revoke_all, list_active}` existing.
   - `l0/src/grpc/mod.rs` — register `SessionService` alongside existing `IdentityService`/`TelemetryService`/`ReflexService` în `serve()`.
   - **NU modifică `l0/src/main.rs`** (L0 SACRED) — toată wiring se face în `grpc::serve()`.
   - **NU modifică `l0/src/identity/*`** (Y1 modules, L0 SACRED enforcement) — session_service.rs e doar wrapper peste module-level API public din Y1.

3. **Tests**
   - Unit tests pentru convert layer (proto ↔ Rust types)
   - Integration test: spawn SessionService în background tokio task → client tonic apelează IssueToken → primește token valid → ListActive returnează 1 sesiune → RevokeSession → ListActive returnează 0
   - Pentru InitializeForTest: cu master_key fix `[0u8; 32]` în test (NU în production)

#### B. Cargo workspace conversion

1. **Root**
   - `Cargo.toml` (NEW, root): `[workspace] members = ["l0", "orchestrator"] resolver = "2"` + `[workspace.dependencies]` cu deps shared (tokio, tracing, anyhow, thiserror, serde, prost, tonic, hex, sha2 — versiuni unice).
2. **l0**
   - `l0/Cargo.toml` — adapt minor pentru workspace inheritance (e.g. `tokio.workspace = true` unde aplicabil); restul intact. Verifică `cargo test -p ybos-l0` still green.
3. **orchestrator** — vezi C.
4. **CI**
   - `.github/workflows/ci.yml` — schimbă `cargo test` în `cargo test --workspace`, similar pentru cross-compile job dacă orchestrator targets aarch64.

#### C. orchestrator/ crate (L1 skeleton)

Layout:
```
orchestrator/
├── Cargo.toml             # package = ybos-orchestrator
├── build.rs               # tonic-build pentru proto
├── proto/
│   └── orchestrator.proto # L1's own API (RegisterAgent, ListAgents, InvokeAgent, ...)
└── src/
    ├── lib.rs             # public re-exports
    ├── main.rs            # binary daemon mode (optional, minimal)
    ├── agent.rs           # trait Agent + Manifest struct
    ├── runtime.rs         # trait AgentRuntime + impl InProcessRuntime + placeholder SubprocessRuntime trait (no impl)
    ├── manifest.rs        # parser TOML pentru manifest.toml
    ├── capability.rs      # Layer 1 enforcement (declarative check)
    ├── registry.rs        # AgentRegistry: static + runtime registration
    ├── l0_client.rs       # gRPC client pentru L0 SessionService + IdentityService
    └── agents/
        └── hello.rs       # demo agent in-process pentru smoke test
```

Funcționalitate minimă Y3:
- `Agent` trait cu metode: `manifest() → &Manifest`, `invoke(call: AgentCall) → Result<AgentResponse>`
- `Manifest` struct cu: `name: String, version: String, capabilities: Capabilities` unde `Capabilities` declară `net.domains`, `fs.paths`, `data.types`, `data.user_prefs` (read|read_write|none)
- `AgentRuntime` trait cu `spawn(manifest) → RuntimeHandle`, `invoke(handle, call) → Response`. Impl `InProcessRuntime` care păstrează agenții ca `Arc<dyn Agent>` într-un HashMap.
- `AgentRegistry` cu: `register_static(agent: Arc<dyn Agent>)`, `register_runtime(manifest_toml: &str, factory: Box<dyn Fn() -> Arc<dyn Agent>>)`, `list() → Vec<&Manifest>`, `get(name) → Option<Arc<dyn Agent>>`
- `capability::enforce(manifest, intended_op: Operation) → Result<()>` — verifică dacă op-ul cerut e declarat în manifest. Operațiuni: `NetConnect(domain)`, `FsRead(path)`, `FsWrite(path)`, `UserContextRead`, `UserContextWrite`.
- `l0_client::L0Client` cu metodă `issue_session_token(scope, expiry, peer_fingerprint) → SessionToken` care apelează gRPC SessionService.
- `hello::HelloAgent` — agent demo cu manifest minimal (no capabilities), răspunde la invoke cu "hello from <name>".

#### D. End-to-end demo (smoke test acceptance)

Test integration care:
1. Pornește l0 daemon în task tokio
2. Apelează `SessionService.InitializeForTest` cu master_key fix
3. Pornește orchestrator → instanțiază `L0Client`
4. Apelează `l0_client.issue_session_token(scope, expiry, peer_fp)` → primește token valid
5. Registrează `HelloAgent` static + un al doilea agent printr-un manifest.toml string runtime-registered
6. `registry.list()` returnează 2 agenți
7. `runtime.invoke("hello", AgentCall{...})` returnează response așteptat
8. Test capability enforcement: invoke cu op nedeclarat → `Err(CapabilityDenied)`

### Acceptance criteria Y3

- [ ] `Cargo.toml` root cu workspace funcțional (`cargo build --workspace` verde)
- [ ] `cargo test --workspace` verde (Y1 48 tests + orchestrator new tests + session_service new tests)
- [ ] Zero modificări în `l0/src/identity/*` și `l0/src/main.rs` (L0 SACRED preserved)
- [ ] Zero modificări în `docs/`, `YBOSClaude.md`, `README.md` root, `reference/`
- [ ] `l0/proto/l0.proto` extins cu SessionService (4 RPC + 1 dev-only)
- [ ] `l0/src/grpc/session_service.rs` implementat + înregistrat în `grpc::serve()`
- [ ] `orchestrator/` crate creat conform layout-ului
- [ ] Demo end-to-end test pass: orchestrator obține session token real de la L0, registrează agenți (static + runtime), capability enforcement blochează op nedeclarat
- [ ] CI: `Build & Test l0`, `Cross-compile l0 for Android` (workspace-aware), `ShellCheck` toate verzi. Adaugă nou job `Build & Test orchestrator`.

### Ce NU intra în Y3

- Privacy firewall Layer 2 (eBPF redactor) — fază separată
- Privacy firewall Layer 3 (LLM judge) — fază separată
- LLM inference integration — Y4
- Persistent memory per-agent (vector DB) — fază separată
- User-Context Memory subsystem — fază separată
- Agent Builder LLM-assisted configurator full — Y12.5 (Y3 lasă doar hook în registry pentru runtime registration)
- Real laptop pairing flow (QR/NFC scan + mTLS conn) — fază separată
- Process isolation pentru agenți (SubprocessRuntime impl real) — design-uit ca trait în Y3, impl ulterior
- Replace Argon2id-XOR envelope A cu AEAD vetted — known carry-over Y1, fază separată
- Cross-compile orchestrator pentru aarch64-android — verificare doar pe l0 în CI Y3; orchestrator cross-compile când avem nevoie

---

## Y4+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Semne de întrebare doar unde **chiar afectează faza activă (Y3)**.

- **LLM inference layer** — llama.cpp + mlc-llm pe NPU. ❓ Inference trait design: Y3 orchestrator nu o consumă direct, dar registry/runtime ar trebui să poată invoca agenți care apoi cer LLM (deferat la Y4).
- **Agent seed: Calendar** — primul agent end-to-end demo cu LLM tools.
- **Agent seed: News Digest**.
- **Privacy firewall Layer 1 (capabilities)** — Y3 livrează schelet (`capability::enforce`); Y7 hardenizează enforcement pe toate operațiile + audit log + UI.
- **Privacy firewall Layer 2 (eBPF redactor)**.
- **Privacy firewall Layer 3 (LLM judge)**.
- **Agent seed: Trip Planner**.
- **Agent seed: Market Intel**.
- **Agent seed: Learning Curator**.
- **Agent Builder Framework** — template `agents/_template/` + LLM-assisted configurator + UI flow. Y3 lasă hook în registry; Y12.5 livrează workflow complet.
- **User-Context Memory subsystem** — storage + sync + capability `data.user_prefs`. Y3 declară doar capability în Manifest, NU implementează storage.
- **Laptop Companion (Tauri)** — pairing QR/NFC + session crypto + task offload + cache sync. Y3 livrează server-side (L0 SessionService) + client-side (orchestrator L0Client); Tauri app + protocol mTLS rămân fază separată.
- **UI native YBOS mobile** — launcher, onboarding wizard UI, agent dashboards.
- **Cross-device extins** (multi-phone, tabletă) — post-MVP.
- **Cloud burst activation** — v0.2+.
- **VM Mode (Tier 1) laptop** — Linux VM minim, GPU passthrough, SEV-SNP/TDX integration. Research, post-MVP.
- **Split inference layer-by-layer** ❓ research item (vezi ARCHITECTURE.md §4.5). Independent.
- **SubprocessRuntime impl real** — pentru process isolation agenți. Y3 design-uit ca trait, impl când avem nevoie reală (probabil aproape de Privacy Firewall hardening).

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
