# YBOS Architecture (detailed)

> Versiune: 0.2
> Data: 2026-05-21 (sesiunea 2 — laptop companion + user-context memory + task offload)
> Sursa de adevăr deciziilor: `YBOSClaude.md` §4

---

## 1. Big picture

```
┌──────────────────────────────────────────────────────────────────────┐
│                       YBOS Phone (sursa de adevar)                   │
│                                                                      │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │              User Interface (UI native YBOS)               │     │
│   │       Onboarding wizard │ Launcher │ Agent dashboards      │     │
│   └────────────────────────────┬───────────────────────────────┘     │
│                                │ Binder + gRPC                       │
│                                ▼                                     │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │  L2 — Cognitive Layer                                      │     │
│   │  Main LLM (3B/8B quant, on NPU)                            │     │
│   │  Sub-agents: Calendar│Trip│Learning│Market│News│Custom...  │     │
│   │  Privacy Guard (LLM judge for outbound payloads)           │     │
│   └────────────────────────────┬───────────────────────────────┘     │
│                                │                                     │
│                                ▼                                     │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │  L1 — Agentic Layer (ybos-orchestrator, Rust)              │     │
│   │  Routing │ Capability enforcement │ Privacy firewall       │     │
│   │  User-Context Memory │ Agent Builder │ Session manager     │     │
│   └────────────────────────────┬───────────────────────────────┘     │
│                                │ gRPC + MQTT                         │
│                                ▼                                     │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │  L0 — Reflex Layer (ybos-l0, Rust daemon)                  │     │
│   │  Identity (per-user) │ HW telemetry │ Reflex actions       │     │
│   │  Boot integrity │ L0 SACRED enforcement                    │     │
│   │  Session Token Issuance API (hook pentru pairing laptop)   │     │
│   └────────────────────────────┬───────────────────────────────┘     │
│                                │                                     │
│   ──────────────── Kernel boundary ─────────────────────────         │
│                                │                                     │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │  Linux Kernel (mainline from AOSP)                         │     │
│   │  + YBOS kernel modules (Rust): policy, eBPF firewall       │     │
│   │  + Android HAL bridges (modem, camera, sensors, GPU/NPU)   │     │
│   └────────────────────────────────────────────────────────────┘     │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
                                  │
                                  │ mTLS over Wi-Fi LAN (session-scoped)
                                  │ AES-256-GCM payloads, HKDF session_key
                                  │
                                  ▼
┌──────────────────────────────────────────────────────────────────────┐
│              YBOS Laptop Companion (Tauri app, ephemeral)            │
│                                                                      │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │  Tauri UI (WebView + Rust core)                            │     │
│   │  Conversation │ Agent dashboards │ Settings                │     │
│   └────────────────────────────┬───────────────────────────────┘     │
│                                │                                     │
│                                ▼                                     │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │  Session Client (Rust)                                     │     │
│   │  mTLS conn │ Task receiver │ Result encryptor              │     │
│   │  Encrypted user-context cache (session-scoped)             │     │
│   │  Zeroize on logout                                         │     │
│   └────────────────────────────┬───────────────────────────────┘     │
│                                │                                     │
│                                ▼                                     │
│   ┌────────────────────────────────────────────────────────────┐     │
│   │  LLM Inference (local, GPU-accelerated)                    │     │
│   │  llama.cpp + Vulkan/Metal/DirectML/CUDA                    │     │
│   │  Runs offloaded tasks decrypted in-process                 │     │
│   └────────────────────────────────────────────────────────────┘     │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 2. L0 — Reflex Layer

### 2.1 Identitate per-user
- La onboarding: generate `K` (master key 256-bit, o singură dată)
- 3-envelope crypto:
  - **A**: Argon2id(PIN + biometric_template + device_fingerprint, salt, t=4, m=64MiB) → unwrap K
  - **B**: TEE seal (StrongBox pe Pixel, Hexagon TEE pe Snapdragon, equivalent pe Mediatek/Apple Silicon) → automat pe device de origine
  - **C**: YubiKey HMAC-SHA1 slot 2, opt-in NFC/USB-C → unwrap K când e prezent
- BIP39 24 cuvinte = paper backup, afișat o dată la onboarding, scris pe hârtie de user
- `identity_core.bin` = nucleul identitar (nume, UUID, public part al biometric template), semnat HMAC cu K

### 2.2 L0 SACRED
- Lista hardcoded în `l0/src/identity/sacred.rs`
- Refuz sintactic la orice scriere (nu cerere de autorizare)
- Hash check la boot — dacă lista a fost modificată, boot blocat
- Pe Android: SELinux policy `restrict_l0_sacred` + immutable bit (fs-verity)

### 2.3 HW telemetry
- `/sys/class/hwmon/`, `/sys/class/thermal/`, `/sys/class/power_supply/`, `/proc/stat`, ACPI
- Plus Android sensors: accelero, gyro, baro, ambient light, proximity
- Publicat pe MQTT topic `ybos/telemetry/*`

### 2.4 Reflex actions
- cpufreq governor write (`/sys/devices/system/cpu/.../scaling_governor`)
- Display brightness
- CPU throttle / unthrottle (battery low)
- Suspend / wake

### 2.5 Session Token Issuance API (hook pentru pairing laptop, Y1 design)
- Funcție expusă de L0: `issue_session_token(scope, expiry, peer_fingerprint) -> SessionToken`
- Token derivat din K-master via HKDF cu salt aleator + epoca timpului
- Salt-ul nu se persistă — sesiunea moare cu device-ul, cu memoria, sau cu apel explicit `revoke_session(session_id)`
- Y1 implementează doar interfața API + storage pentru lista sesiuni active. Pairing-ul efectiv (QR/NFC flow) vine în faza laptop companion.

---

## 3. L1 — Agentic Layer (ybos-orchestrator)

### 3.1 Routing
- User input → main LLM → intent classification → route to agent
- Multi-agent collaboration (one agent calls another via L1 mediator)

### 3.2 Capability enforcement
Fiecare agent declară în `manifest.toml`:
```toml
[capabilities]
net.domains = ["calendar.google.com", "apis.google.com"]
fs.paths = ["${YBOS_DATA}/calendar/"]
data.types = ["calendar_event", "contact"]
data.user_prefs = "read"  # sau "read_write" pentru cei care invata preferinte
io.notifications = true
```
L1 refuză orice operație ne-declarată. Aplicat identic pentru seed agents și custom agents.

### 3.3 Privacy firewall (3 layere)
- **Layer 1**: capabilities (declared în manifest)
- **Layer 2**: eBPF redactor — PII strip pe net syscalls
- **Layer 3**: LLM judge — sub-agent local mic (sub-1B params), decide allow/redact/block/ask-user pe payload outbound

### 3.4 Memory (per-agent vector DB)
- Vector DB embedded (sqlite-vss sau qdrant) per agent
- Index temporal + categorial
- TTL configurabil per data type
- Disjunct de user-context memory (vezi §3.6)

### 3.5 Agent Builder Framework
- Template generic în `agents/_template/` (Cargo crate skeleton + manifest.toml + tool registration stubs)
- CLI dev: `ybos agent new <slug>`
- LLM-assisted configurator: user descrie agent în natural language → main agent generează:
  - manifest.toml draft cu capabilities propuse
  - skeleton Rust cu tool stubs
  - sugestie tool calls disponibile
- User aprobă/editează capabilities înainte de instanțiere
- Restart orchestrator (sau hot-reload dacă registry permite) → agent live
- Capability enforcement + firewall aplicat identic ca pentru seed

### 3.6 User-Context Memory (subsistem dedicat)
Layer separat de vector DB-urile per-agent. Scop: păstrare detalii recurente / preferințe învățate, accesibile mai multor agenți.

**Storage**:
- sqlite cu schema structurată (categorii: travel_prefs, calendar_recurrence, contacts_context, personal_patterns, custom_tags)
- Embeddings pentru fuzzy lookup ("ce-am preferat ultima oară la Berlin?")
- Encryption at rest cu cheia derivată din K-master

**Acces**:
- Capability `data.user_prefs` (read | read_write) declarată în manifest agent
- Agents fără această capability NU pot citi/scrie nimic în user-context
- Main LLM are by default `read_write` (e orchestratorul)
- API: `user_context.lookup(category, query)`, `user_context.append(category, fact, confidence)`, `user_context.feedback(fact_id, accepted: bool)`

**Learning loop**:
- Agents propun fact-uri noi: "Am observat că de obicei vrei alertă 1h înainte de meetings externe. Salvez asta?"
- User confirmă/respinge (primele câteva ori); după, agent salvează silent cu confidence high
- Periodic main agent rulează "consolidation": merge fact-uri duplicate, marchează stale, propune review pentru cele cu confidence low

**Privacy**:
- Niciodată în cloud fără consimțământ explicit per categorie
- Layer 3 firewall verifică cereri cloud care includ user-context
- Export user explicit ("Exportă tot ce știi despre mine" → JSON criptat cu K)
- Erase: "Uită X" → main agent traversează user-context + memory-urile per-agent → confirm + șterge

### 3.7 Session Manager (orchestrare laptop companion)
- Lista sesiuni active (laptop X, laptop Y)
- Per sesiune: session_id, peer_fingerprint, capability scope, expiry, last_seen
- API: `start_session(qr_payload)`, `revoke_session(session_id)`, `revoke_all()`
- Task offload decision logic: main LLM hotărăște per-task dacă rulează local NPU sau ofloadează la laptop activ
- Heartbeat pentru detect laptop disconnect → expiry session

---

## 4. L2 — Cognitive Layer

### 4.1 Main LLM
- llama 3B quantized (Q4_K_M) sau 8B dacă RAM permite
- Backend: llama.cpp (CPU baseline) + mlc-llm (NPU acceleration)
- Context window: 8K tokens MVP, extensibil

### 4.2 Sub-agents
Fiecare are own LLM context (instance separată) + memory + tools. Cei mici (Privacy Guard, classifiers) pot rula pe modele <1B distillate.

### 4.3 Task offload la laptop companion
- Main LLM decide per-task: "ăsta îl pot rula local pe NPU" sau "ăsta e prea greu, ofloadez la laptop"
- Criterii: dimensiune model necesar (e.g. cere context window >8K, sau model 13B+), latency budget, baterie telefon
- Trimite task complet (prompt + context + system instructions) criptat cu session_key
- Laptopul decriptează în RAM procesului, rulează LLM cu GPU local, criptează rezultat, retur
- Laptopul NU păstrează nimic post-task (cu excepția cache user-context care e session-scoped, vezi §6.3)

### 4.4 Cloud burst (v0.2+)
Trait Rust:
```rust
trait Inference {
    async fn complete(&self, prompt: &str, max_tokens: usize) -> Result<String>;
}
struct LocalLlama { /* ... */ }
struct RemoteAPI { endpoint: String, api_key: SecretString, /* ... */ }
```
User aprobă per categorie în settings. Default OFF la MVP.

### 4.5 Split inference layer-by-layer (research item, NU MVP) ❓
**Idee preliminară abandonată ca model primar** datorită latency-ului round-trip telefon↔laptop pe layer (~1-1.5s per token pentru 8B model).

Rămâne ca **direcție de research** dacă în viitor apare:
- Conexiune ultra-low-latency telefon↔laptop (Wi-Fi 7 ultra-low-latency mode, eSIM cellular direct, USB-C tether)
- Hardware nou care permite cryptographic split inference cu garanții (FHE accelerated, secure multiparty)

Beneficiu teoretic: laptopul **nu vede niciodată plaintext**, doar activations criptate-greu-de-invertit. Privacy story 100% chiar pe Tier 2 (App Mode).

George face research independent pe această direcție. Documentăm aici ca semn de întrebare deschis.

---

## 5. App compatibility

### 5.1 Android Runtime preserved (pe telefon)
- ART păstrat din AOSP
- Google Play Services opt-in (MicroG default pentru privacy, Google opt-in)
- Apps Android rulează în "Apps" tab (separate de UI YBOS native)

### 5.2 YBOS-native apps (pe telefon)
- Scrise în Rust + UI framework (Slint sau Jetpack Compose cu Rust binding)
- Inter-process via Binder

---

## 6. Laptop Companion (detailed)

### 6.1 Pairing (session start)
- User pe laptop deschide YBOS Companion → "Conectează la telefon"
- Pe telefon: Settings → Sesiuni → "Adaugă laptop" → afișează QR code OR activează NFC tap
- QR payload: `ybos://session?id=...&pub=...&fp=...&exp=...` (cu signed challenge)
- Sau NFC tap: telefonul scrie pe NFC payload identic
- Laptopul citește payload → derivă session_key prin HKDF + face mTLS handshake cu telefonul
- Telefonul confirmă pe UI: "Laptop 'Lenovo X1 Carbon' conectat. Capabilities: full"
- User pe telefon poate adjusta capabilities sesiunii (e.g. "no calendar write din laptop")

### 6.2 Crypto sesiune
- session_key = HKDF(K-master, salt = random_per_session, info = "ybos-session-v1")
- Salt-ul nu se persistă — moare cu sesiunea
- Toate payload-urile telefon↔laptop: AES-256-GCM cu nonce random per mesaj
- mTLS între daemonul telefonului și clientul laptop (certs derivate per-sesiune din session_key)
- Heartbeat la 30s → după 2 min fără heartbeat, telefonul revocă sesiunea automat

### 6.3 Cache user-context pe laptop (session-scoped)
- La pairing, telefonul trimite snapshot user-context relevant (criptat cu session_key)
- Laptopul îl ține în memorie + opțional encrypted temp file (dacă e prea mare pentru RAM)
- Modificări pe laptop → push înapoi imediat la telefon (write-through)
- La logout / expiry / revoke:
  - Zeroize RAM (folosind `zeroize` crate)
  - Secure delete temp files (shred pe Linux, SDelete pe Windows, secure-erase pe Mac)
  - session_key dispare → cache devine non-decryptable chiar dacă remanesta în memorie/disk

### 6.4 Task offload protocol
```
Phone → Laptop:  { task_id, prompt, system, max_tokens, capabilities_scope } (AES-256-GCM)
Laptop:          decrypt → load model (cache) → llama.cpp inference → encrypt response
Laptop → Phone:  { task_id, response, tokens_used, latency } (AES-256-GCM)
```
Plaintext exists în RAM laptop pe durata inference. Disclaimer la pairing acceptat de user.

### 6.5 Tier 1 (VM Mode) vs Tier 2 (App Mode)

| Aspect | Tier 1 (VM Mode) | Tier 2 (App Mode, default) |
|---|---|---|
| Implementare | Linux VM minim (KVM/Hyper-V/Hypervisor.framework) | Tauri app nativ |
| OS gazdă vede plaintext | NU (cu SEV-SNP/TDX, memoria criptată) | DA (memory laptop e accesibilă OS-ului) |
| GPU access | PCIe passthrough sau paravirtual | Direct (Vulkan/Metal/DirectML/CUDA) |
| User effort | Mare (setup VM, hardware compatibil) | Mic (instalează app) |
| Effort dev | Mare (reutilizăm 80% Linux distro twin) | Mediu (Tauri standard) |
| Disclaimer | Nu necesar (privacy garantată) | Da (T&C explicit la pairing) |

MVP livrează **Tier 2 funcțional**. Tier 1 = research/build după ce Tier 2 e stabil.

### 6.6 Disclaimers și T&C (Tier 2)
La pairing laptop, popup explicit:
> "Pe acest laptop, YBOS rulează ca aplicație. Sistemul de operare (Windows/macOS/Linux) poate teoretic accesa memoria aplicației pe durata sesiunii — inclusiv prompt-urile și răspunsurile LLM. La logout, toate datele sunt șterse criptografic. Pentru izolare puternică, folosește Tier 1 (VM Mode) — vezi documentația. Continui?"

Plus T&C standard (data handling, retention, breach notification) în docs separate.

---

## 7. Build & deploy

### 7.1 AOSP build environment (telefon)
- Branch: android-14-release sau android-15 când stabil
- Custom manifests în `platform/manifests/`
- Build target: device achiziționat (TBD)

### 7.2 Cargo workspace
- Root `Cargo.toml` cu workspace members: l0, orchestrator, agents/*, user_context, firewall, companion, ui
- Shared dependencies în `[workspace.dependencies]`

### 7.3 Companion app distribution
- Tauri produce binare native per platformă: `.exe` (Win), `.dmg`/`.app` (Mac), `.AppImage`/`.deb`/`.rpm` (Linux)
- Auto-update via Tauri updater (signed)

### 7.4 OTA updates (telefon)
- Atomic A/B partitions (AOSP standard)
- Self-rollback la boot failure
- Updates semnate cu cheia YBOS

---

## 8. Cross-device "simbioza" (extins)

În modelul session-based, "simbioza" telefon↔laptop e nativă (vezi §6). Pentru cross-phone (multi-device per user — telefon + tabletă, sau telefon înlocuit), TBD:

- Identity restore din BIP39 paper backup pe device nou
- Migrare K-master cross-device prin envelope re-key (TBD design)
- CRDT-uri pentru calendar / notes (eventual consistency)

Nu MVP.

---

## 9. Decizii deschise (parking lot — semn de întrebare)

- ❓ **MQTT vs Binder pentru telemetrie L0→L1** — TBD când implementăm L1
- ❓ **UI framework mobile** — Slint vs Jetpack Compose binding — TBD când ajungem la UI
- ❓ **LLM model size primary** — 3B vs 8B trade-off RAM/quality
- ❓ **Vector DB choice** — sqlite-vss vs qdrant embedded — benchmark needed
- ❓ **Split inference layer-by-layer** (vezi §4.5) — research George, posibil viitor
- ❓ **VM Mode hardware support** — SEV-SNP/TDX disponibilitate consumer hw — research când ajungem la Tier 1
- ❓ **License** — Apache 2.0 vs MIT vs Proprietary — TBD George decision business model
- ❓ **Nume YBOS final + branding** — TBD post-MVP technical bring-up
