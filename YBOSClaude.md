# YBOS — Claude Code Instructions

> **Creat**: 2026-05-21 (sesiunea 1 — arhitectură initial)
> **Update**: 2026-05-21 (sesiunea 2 — clarificări hardware, agent builder, laptop companion, user-context memory)
> **Lead Dev (planning + final review)**: Claude (Opus 4.7)
> **Code generators**: Codex, Jules, alte tool-uri AI (George decide per task)
> **Final review**: George + Claude + Codex (eventual un dev uman)
> **Origine**: pivot din RemusOS3 — un OS sentient personal pentru George — către **YBOS**, un OS AI vandabil, multi-user, mobile-first cu laptop companion.

---

## 1. VIZIUNE YBOS

**YBOS este un AI Executive Assistant OS** care:

- Rulează **primar pe telefon** (sursa unică de adevăr: identitate, K-master, memorie persistentă, modele LLM)
- Are **laptop companion app** (Tauri cross-platform) — session-based ca WhatsApp Web, fără identitate permanentă pe laptop
- Trimite un **agent LLM principal** ca orchestrator care **delegă** către sub-agenți specializați **și permite user-ului să creeze agenți noi pentru orice task**
- E **vandabil** — multi-user, fiecare device primește identitate la onboarding (NU vine cu owner hardcoded ca Remus)
- Are **privacy ca prim feature** — LLM-ul main agent este și paznic al datelor (firewall în 3 layere)
- Ține **memorie user-context persistent** (preferințe zboruri, recurrențe calendar, context personal recurent)
- Este **scris în Rust** pentru tot userland-ul nou + module kernel proprii
- E **aproape de hardware** (sub-ms reflex) prin daemon L0 Rust + kernel modules Rust + eBPF programs
- Permite **Android apps native** prin baza AOSP (Google Play apps merg din ziua 1)
- Suportă **cloud burst** opt-in pentru LLM mari (design-uit ca trait din zi 1, dezactivat la MVP)

**Diferentiator vs Apple Intelligence / Gemini / Bixby:**
- Nu trimitem date în cloud fără consimțământ explicit per-task. LLM-ul principal e și paznic, nu doar asistent.
- User-ul își poate crea proprii agenți, nu e limitat la cei pre-built.
- Laptop companion fără account-uri remote (totul stă pe telefonul tău, laptopul e doar terminal cu resurse extra).

---

## 2. ARHITECTURA OS-ULUI

### 2.1 Baza: AOSP custom (decizie 2026-05-21)

**Kernel**: Linux mainline (cel din AOSP), cu module Rust YBOS-specific adăugate gradual.
**Userland**: TOT Rust pentru servicii noi YBOS. Android system services păstrate doar cât e necesar pentru runtime apps.
**App layer**: Android Runtime (ART) păstrat → Google Play apps merg nativ. Apps YBOS-native scrise în Rust (Jetpack Compose binding sau Slint).
**Build**: AOSP build system + Cargo workspace pentru toate crate-urile Rust.

**De ce nu Rust kernel from scratch**: ne-ar costa multă muncă înainte de demo. AOSP ne dă day-1 hardware support + Android apps. Rust kernel pur rămâne **side project personal George**, nu road map YBOS.

**Migrare graduală către Rust în kernel**: Linux 6.x suportă cod Rust nativ. În MVP avem kernel modules Rust pentru policy/firewall. Mai târziu înlocuim HAL-uri specifice cu Rust. Pe termen lung contribuim upstream Rust-for-Linux.

### 2.2 3-layer brain (păstrat din Remus, generalizat)

| Layer | Nume | Rol | Implementare |
|---|---|---|---|
| **L0** | Reflex Layer | Identitate per-user, telemetrie HW, reflex sub-ms | Rust daemon (`ybos-l0`), portat din Remus |
| **L1** | Agentic Layer | Orchestrator multi-agent, routing, privacy firewall, user-context memory | Rust service nou (`ybos-orchestrator`) |
| **L2** | Cognitive Layer | LLM principal + sub-agenți specializați | llama.cpp / mlc-llm pe NPU (Tensor / Hexagon / Mediatek — runtime detect) |

**Comunicare între layere**: Binder (Android-native) pentru integrare cu system services + gRPC pentru servicii noi. MQTT broker embedded pentru telemetrie L0→L1 (evaluare înlocuire cu Binder la fază viitoare).

### 2.3 Onboarding wizard (cum se naște L0 personalizat)

```
Prima pornire după instalare:
→ Welcome screen
→ "Salut. Care e numele tău?"        ← input free-text
→ "Alege un PIN de 6+ cifre"           ← input PIN (Argon2id hash)
→ "Folosești amprenta / fața?"          ← opt-in biometric (dacă hw permite)
→ "Ai un YubiKey/security key?"        ← opt-in extra envelope
→ [GENERARE master key K (256-bit, o singură dată)]
→ [Envelope A: Argon2id(PIN + bio + device_fingerprint) → K]
→ [Envelope B: TEE seal pe StrongBox / Hexagon TEE → K]
→ [Envelope C: opt YubiKey HMAC challenge → K]
→ [BIP39 mnemonic 24 cuvinte afișat o SINGURă dată → user scrie pe hârtie]
→ [Scriere identity_core.bin semnat HMAC cu K]
→ [L0 sealed — nu se mai atinge decât cu autorizare envelope A/B/C]
→ "Bun venit, [nume]. Sunt asistentul tău. Cu ce începem?"
```

**Reguli imuabile post-onboarding:**
- L0_SACRED files devin read-only (chattr +i echivalent pe Android: SELinux policy + immutable bit)
- Modificare L0 → necesită oricare envelope A/B/C
- L0 SACRED files → refuz hard, niciun cod (self-improvement, OTA update) nu le atinge
- Pierderea TUTUROR envelope-uri + paper backup = device wipe necesar

### 2.4 Laptop Companion (model session-based, paradigma WhatsApp Web)

**Principiul**: telefonul e sursa unică de adevăr. Laptopul e **terminal temporar cu resurse extra (RAM/GPU)** care extinde capacitățile YBOS pe durata sesiunii. NU are identitate proprie, NU stochează nimic permanent.

```
User vrea YBOS pe laptop X (orice laptop, oriunde):
1. Descarcă "YBOS Companion" (Tauri app, ~10MB, Win/Mac/Linux)
2. Deschide app → "Scan QR / NFC tap"
3. Pe telefon: Settings → "Conectează laptop nou" → afișează QR cu session token
4. Scan QR (sau NFC tap) → laptop primește: session_key (ephemeral, derivat din K) + capability scope + expiry
5. Conexiune mTLS telefon ↔ laptop stabilita pe Wi-Fi local
6. Telefonul trimite tasks/prompts criptate cu session_key
7. Laptopul decriptează în memorie procesului, rulează LLM cu GPU local, returnează rezultat criptat
8. Cache user-context (preferințe, recurrențe) sincronizat temporar pe laptop, criptat, doar pe durata sesiunii
9. Logout / expiry / "Logout laptop X" din telefon → session_key dispare, cache șters, laptop e curat
```

**Două moduri de rulare laptop**:

| Mod | Cum | Privacy | Effort dev |
|---|---|---|---|
| **App Mode** (Tier 2, default) | Tauri app nativ peste Windows/Mac/Linux | OS-ul gazdă poate teoretic citi memoria procesului — disclaimer explicit la pairing, "T&C" acceptate | Mediu |
| **VM Mode** (Tier 1, power users) | Linux VM minim peste OS gazdă (Hyper-V/KVM/Hypervisor.framework), GPU passthrough | OS gazdă vede doar VM neagră; cu hardware compatibil SEV-SNP/TDX, memoria criptată | Mare (reutilizăm 80% Linux distro twin) |

**Task offload model** (decizie 2026-05-21 sesiunea 2):
- Telefonul decide per-task: "ăsta îl rulez local NPU mobil" sau "ăsta îl ofloadez pe laptop (e prea greu)"
- Task offload = trimit task întreg către laptop, decriptare în RAM procesului laptop, inference cu GPU, criptare răspuns, retur
- Laptopul vede temporar plaintext pe durata calculului. Acceptat ca trade-off, disclaim-uit la onboarding.
- **NU split inference layer-by-layer** (idee preliminară abandonată — round-trip telefon↔laptop pe layer = latency inacceptabilă, ~1-1.5s per token). Rămâne ca semn de întrebare research, vezi ARCHITECTURE.md.

**Securitate sesiune**:
- session_key efemer derivat din K-master prin HKDF cu salt aleator per sesiune
- Encryption: AES-256-GCM pentru toate payload-urile telefon↔laptop
- Cache user-context laptop: encrypted la rest cu session_key, în memorie sau în temp dir cu auto-delete
- La logout/expiry: secure delete (zeroize în RAM, shred pe disk dacă a existat), session_key dispare → cache devine non-decryptable
- Revocare instant: "Sesiuni active" în Settings telefon → Logout individual sau "Logout all"

### 2.5 User-Context Memory (subsistem dedicat)

**Scop**: păstrare detalii recurente / preferințe învățate despre user, accesibile tuturor agenților (seed sau custom).

**Exemple stocate**:
- Preferințe travel: Lufthansa la Berlin, seat aisle, hotel <2km centru, business class peste 4h flight
- Recurrențe calendar: alerte 15min pentru ședințe interne, 1h pentru externe, mama = luni 14:00
- Context personal: "mama" = Maria Popescu (telefon X, ziua Y), "biroul" = adresa Z
- Pattern-uri repetate: "în general comand pizza vineri", "rulez maraton de focus luni dimineața"

**Arhitectură**:
- Layer separat în L1 orchestrator (NU în vector DB-urile per-agent)
- Storage local pe telefon, vector + structured (sqlite cu coloane tipizate + embedding pentru fuzzy lookup)
- Capability `data.user_prefs` (read sau write) declarat în manifest.toml — fără asta, agentul nu vede memoria
- Main LLM consultă la decision time: "user a mai zburat la Berlin → vrei aceleași preferințe?"
- Agenții scriu observații (cu confirmare user pentru primele câteva): "Am observat că de obicei vrei alertă 1h înainte de meetings externe. Salvez asta?"

**Sincronizare laptop**:
- Pe durata sesiunii, snapshot user-context criptat cu session_key, copiat pe laptop pentru viteza UI
- Modificări pe laptop → push înapoi la telefon (sursa de adevăr) → la logout, cache laptop șters
- Telefonul e mereu authoritative; laptopul nu poate diverge

**Privacy**:
- Niciodată în cloud fără consimțământ explicit per categorie
- Layer 3 firewall (LLM judge) verifică cereri cloud care includ user-context
- Export user explicit ("Exportă tot ce știi despre mine" → JSON criptat cu K) — drept de portabilitate

---

## 3. STACK TEHNIC FIXAT

| Componentă | Alegere | Motiv |
|---|---|---|
| OS base | AOSP 14/15 custom | Android compat + drivere mobile + kernel optimized mobile |
| Kernel | Linux mainline (cel din AOSP) + module Rust | Pragmatic, Rust upstream maturizează în paralel |
| Userland nou | Rust 1.75+ | Cerință George, safety + performance |
| L0 daemon | Rust (portat din Remus l0/) | Deja există scaffold S6.0-S6.4 |
| L1 orchestrator | Rust nou (`ybos-orchestrator`) | Bridge între L0 și agenți |
| L2 inference | llama.cpp + mlc-llm | Acceleration pe Tensor / Hexagon / Mediatek NPU |
| Vector DB | sqlite-vss sau qdrant embedded | Memorie semantică agenți, locale |
| User-context store | sqlite + embeddings | Structured + fuzzy lookup |
| Privacy firewall | eBPF Rust (aya-rs) + LSM hooks | 3 layere: capabilities + redactor + LLM judge |
| Comunicare cross-layer | Binder + gRPC (tonic) | Binder pentru Android-native, gRPC pentru cross-platform |
| Telemetrie hw → L1 | MQTT (rumqttd embedded) sau Binder | TBD |
| UI native YBOS (mobile) | Jetpack Compose (Kotlin/Rust binding) sau Slint | TBD când ajungem la UI |
| Laptop companion UI | Tauri (Rust + WebView) | Cross-platform, binary mic, Rust shared core |
| Session crypto laptop | AES-256-GCM + HKDF din K-master | Standard, ephemeral |
| Build | AOSP build + Cargo workspace | Standard |
| App compat | Android Runtime (ART) păstrat | Google Play merge nativ |
| Cloud LLM (opt-in) | Trait `Inference` cu impl `RemoteAPI` (stub MVP) | Anthropic / OpenAI / Together / self-hosted |

---

## 4. DECIZII AGREATE (sesiuni 2026-05-21)

Toate confirmate explicit de George. Nu se renegociază în sesiuni viitoare fără discuție nouă.

### 4.1 OS base: AOSP custom + Rust userland
- Rust kernel from scratch rămâne **side project personal George**, NU road map YBOS.
- Migrare graduală Rust în kernel via Rust-for-Linux upstream + module proprii.

### 4.2 Device test: ARM64 cu NPU dedicat + bootloader unlockable (model TBD)
- **Pixel 7 e exemplu de device compatibil, NU obligație.** Alegerea finală depinde de disponibilitate / preț în momentul achiziției.
- Lista alternative + specs minime: `docs/HARDWARE.md`.
- **Codul YBOS NU trebuie să facă asumpții hardcoded despre modelul exact** — abstractizare prin HAL trait, device-specific code izolat în module clar marcate.

### 4.3 MVP scope: 5 agenți seed + framework de creare agenți custom

**Cei 5 agenți seed (live la MVP demo):**
1. **Calendar** — local + Google Calendar sync (cu permission user)
2. **Trip Planner** — flights/hotels APIs + summary + booking handoff
3. **Learning Curator** — user dă share din TikTok/IG/YouTube Shorts → picker categorii → background Whisper + LLM decompose → cards UI
4. **Market Intel** — agent reusable, instanțiabil per-piață (energie, tech, etc.)
5. **News Digest** — surse whitelisted, morning brief

**Framework de creare agenți custom (parte din MVP, NU post-MVP):**
- User poate crea agenți noi pentru orice task fără a aștepta release nou.
- Mecanism: template + manifest.toml scaffold + LLM-assisted configurator (limbaj natural → skeleton + capabilities draft)
- UI: "+ Agent nou" în settings SAU conversațional cu main agent
- Privacy enforcement identic cu seed agents (capabilities obligatorii, firewall 3-layer aplicat, sandbox).

### 4.4 Privacy firewall: 3 layere full la MVP
- **Layer 1**: capability-based, fiecare agent declară ce poate accesa
- **Layer 2**: eBPF redactor (Rust + aya-rs) pe net syscalls, strip PII
- **Layer 3**: LLM judge (sub-agent local mic) decide allow/redact/block/ask-user pe payload înainte de orice cloud send

### 4.5 Cloud burst: trait stub la MVP, activat la v0.2+
- Trait `Inference` cu impl `LocalLlama` (default) + `RemoteAPI` (stub)
- User aprobă per categorie în settings.

### 4.6 Laptop Companion: session-based (WhatsApp Web paradigm)
- App Tauri cross-platform (Windows/Mac/Linux), Tier 2 default
- VM Mode (Tier 1) opțional pentru power users
- Task offload model: telefonul decide per-task, laptopul rulează LLM local, mediator de sesiune
- Pairing: QR scan SAU NFC tap (ambele suportate)
- Session ephemeral, zero state persistent post-logout
- Disclaimers explicit la onboarding pairing pentru limitări security Tier 2

### 4.7 User-Context Memory ca subsistem dedicat
- Layer separat în L1, accesibil agenților prin capability `data.user_prefs`
- Storage local pe telefon (sursa de adevăr); cache temporar criptat pe laptop pe durata sesiunii
- Privacy: niciodată în cloud fără consimțământ explicit

### 4.8 Onboarding flow generalizat
- L0 NU mai vine hardcoded cu owner. Se naște la primul boot.
- 3-envelope crypto din Remus păstrat. TPM → StrongBox / Hexagon TEE. YubiKey → opt-in NFC/USB-C.
- API pentru session token issuance pre-conceput (hook, NU implementare completă în Y1).

### 4.9 Roluri în proiect
- **George**: arhitect, final decision maker, validează direcția
- **Claude (Opus 4.7)**: Lead Dev, planning + final review + arhitectură detailed
- **Codex, Jules, etc.**: code generators per task
- **Dev uman (eventual)**: review periodic

### 4.10 Fără estimări de timp în docs
- Roadmap-ul NU mai include "săptămâni", "luni", "MVP target 8-10 luni".
- Doar ordinea fazelor + dependențe + ce afectează arhitectura. Timpul real e irelevant până când produsul devine vandabil.

---

## 5. STRUCTURĂ REPOSITORY YBOS

```
YBOS/
├── README.md                     # Public-facing, scurt
├── YBOSClaude.md                 # ACEST FIȘIER — sursa de adevăr pentru Claude
├── LICENSE                       # TBD cu George (Apache-2.0 / MIT / Proprietary)
├── .gitignore
├── docs/
│   ├── ARCHITECTURE.md           # Decizii arhitecturale detaliate (inclusiv laptop companion + user-context)
│   ├── ROADMAP.md                # Faze (fără timeline-uri); detaliat doar pentru Y1
│   ├── HARDWARE.md               # Specs minime device test (flexibil)
│   └── L0_SACRED.md              # Lista fișiere sacre L0
├── l0/                           # Rust daemon kernel-adjacent (portat din Remus)
│   ├── Cargo.toml                # package name: ybos-l0
│   ├── build.rs
│   ├── proto/
│   │   └── l0.proto              # gRPC services
│   ├── src/
│   │   ├── main.rs
│   │   ├── identity/             # Identity per-user (de generalizat în Y1)
│   │   ├── hw/                   # HAL telemetrie
│   │   ├── bus/                  # MQTT broker + publisher
│   │   ├── grpc/                 # gRPC services
│   │   └── reflex/               # Reflex actions
│   └── README.md
├── orchestrator/                 # L1 — Rust nou, TBD
├── agents/                       # Definiții agenți (seed + framework pentru custom)
│   ├── _template/                # Template pentru agenți custom
│   ├── calendar/                 # Seed
│   ├── trip/                     # Seed
│   ├── learning/                 # Seed
│   ├── market/                   # Seed
│   └── news/                     # Seed
├── user_context/                 # Subsistem user-context memory (storage + sync)
├── firewall/                     # Privacy firewall 3 layere
├── companion/                    # Laptop companion Tauri app (Win/Mac/Linux)
├── ui/                           # UI native YBOS mobile
├── platform/                     # AOSP customizations, build scripts
└── reference/
    └── REMUS_PORT_NOTES.md       # Ce am portat din RemusOS3, ce nu, de ce
```

---

## 6. CE AM PORTAT / CE AM LĂSAT DIN RemusOS3

### Portat (✅ incluse în YBOS initial commit)
- `l0/` — întregul crate Rust (Cargo.toml rebrand `ybos-l0`, restul cod păstrat pentru moment)
  - Identity + boot integrity
  - HAL telemetrie
  - MQTT broker + publisher
  - gRPC services
- Conceptul **L0_SACRED** — documentat în `docs/L0_SACRED.md`
- Conceptul **3-envelope crypto** (Argon2 + TEE + opt YubiKey) — documentat în ARCHITECTURE.md
- Conceptul **BIP39 paper backup** — same
- Conceptul **per-user identity blob** semnat HMAC — same (de generalizat în Y1)

### NU portat (❌ rămas în RemusOS3)
- Tot codul Python (`core/*.py`, `web_interface.py`, `interface.py`) — înlocuit cu Rust
- ChromaDB memory — înlocuit cu sqlite-vss / qdrant
- Flask web UI — înlocuit cu native Android UI + Tauri laptop companion
- Deploy NixOS / Buildroot — înlocuit cu AOSP build (VM Mode laptop poate refolosi Linux distro twin în viitor)
- "Sentient" stuff (mood, dreams, journal) — nu MVP, eventual v2+ ca feature opt-in
- T460-specific (BIOS, fingerprint reader Validity) — N/A pe mobil
- George-hardcoded ownership — înlocuit cu onboarding wizard
- Roluri PRIMARY/SATELLITE/LIVE — înlocuite cu session-based laptop companion

### De adaptat în Y1 (⚠️ generalizare)
- `l0/src/identity/` — de generalizat din George-only la enrollment dinamic la onboarding
- `l0/src/identity/sacred.rs` — paths actualizate la layout YBOS
- `l0/src/bus/` — evaluare înlocuire cu Binder pe Android (TBD fază viitoare)

---

## 7. ROADMAP YBOS (faze, fără timeline)

Detaliat: vezi `docs/ROADMAP.md`. Sumar succint:

| Fază | Nume | Status |
|---|---|---|
| Y0 | Repo bootstrap, structură, docs, port l0/ ca-i | ✅ Done |
| Y1 | L0 generalizare: identity per-user + API session token + onboarding scaffold | ✅ Done (PR #1) |
| Y2 | AOSP build env + cross-compile ybos-l0 aarch64 + AOSP overlay scaffolds (device-agnostic) | ✅ Done (PR #2) |
| Y2.b | Flash + boot verification | BLOCKED pe achiziție device |
| Y3 | L1 orchestrator skeleton (hybrid trait+runtime) + L0 SessionService gRPC + Cargo workspace | ✅ Done (PR #3 + PR #4 cleanup) |
| Y4 | LLM inference layer (skeleton + LocalLlama CPU via llama-cpp-2 + streaming, new `inference/` crate) | ✅ Done (PR #5) |
| Y4.b | NPU acceleration (mlc-llm) + cross-compile aarch64 | BLOCKED pe device |
| Y5 | Orchestrator ⇌ Inference integration (AgentContext + llm capability) + ybos-proto extract + Y4 fixes | ✅ Done (PR #6) |
| Y6 | Memory layer: VectorStore + Embedder traits, sqlite-vec store, fastembed embedder, orchestrator integration | ✅ Done (PR #7) |
| Y7 | **Privacy Firewall Layer 1 hardening (path normalization + audit log) + Y4/Y5/Y6 carry-overs cleanup** | **NEXT** |
| Y8+ | Agenți seed (News, Calendar), user-context memory subsystem, firewall Layer 2/3, agent builder framework, laptop companion (Tauri), UI native | TBD |

**Detaliile fazelor Y8+ sunt notate cu semne de întrebare în ROADMAP.md** doar acolo unde decizia afectează arhitectura sau implementarea din Y7. Restul = "TBD când ajungem".

---

## 8. REGULI CRITICE DE COD

### 8.1 Toate sursele noi sunt Rust
Fără Python în YBOS. Python doar pentru tool-uri dev internal (build scripts), nu pentru runtime.

### 8.2 Paths via PathBuf, niciodată hardcoded strings
```rust
// GRESIT
fs::read_to_string("config/identity_core.bin")

// CORECT
use crate::identity::paths;
fs::read_to_string(paths::identity_blob())
```

### 8.3 L0 SACRED — refuz hard
Vezi `docs/L0_SACRED.md`. Niciun cod self-improvement, OTA update, agent skill discovery nu atinge L0 sacred files.

### 8.4 No diacritics in print()/println!/eprintln!
Păstrat din Remus (lecție Windows cp1252 dev environment). Diacriticele OK în strings retur, fișiere UTF-8, docstrings.

### 8.5 Gitignored — niciodată commit fără confirmare
- Identitate per-user (blob, salt, BIP39 paper backup digital)
- Keys, certificates, session tokens
- Build artifacts (target/, out/, .apk, .img)
- Cache local

### 8.6 Capability declarations obligatorii pentru agenți
Fiecare agent (seed sau custom) are `manifest.toml` cu capabilities. L1 refuză orice operație ne-declarată. NU există shortcut pentru agenți custom.

### 8.7 Hardware abstraction — fără asumpții device-specifice în cod generic
Codul YBOS (orchestrator, agenți, firewall, UI) NU presupune Pixel 7 sau orice model anume. Device-specific code izolat în `l0/src/hw/` și `platform/`.

### 8.8 Session crypto — zeroize la logout, niciodată persist
Session keys, plaintext în memorie pe laptop, cache user-context laptop: TOATE trebuie zeroize-uite la logout/expiry. Folosește `zeroize` crate. Cache pe disk (dacă e necesar) → secure delete cu shred-equivalent.

---

## 9. COMPORTAMENT AȘTEPTAT DE LA CLAUDE

### 9.1 Lead Dev, dar George e arhitect
Eu propun direcția tehnică detaliată. George validează / redirectează / decide final. Când am dubii dacă o decizie e în zona mea sau a lui, întreb înainte.

### 9.2 Antipattern — NU iau decizii strategice singur
Decizii care CER întrebare explicită (lista nu exhaustivă):
- Toolchain, runtime, framework nou
- Locație fișier/director major
- Dependențe externe noi
- Pattern arhitectural (Binder vs gRPC, embedded broker vs extern, etc.)
- Interpretare ambiguă a răspunsului lui George ca decizie

### 9.3 Antipattern — nu derutez utilizatorul cu termen ambigu
Explic terminologia înainte de opțiuni. Nu presupun că termenii ("OS", "kernel", "platform", "scope") au același sens pentru George.

### 9.4 Comunicare în română
George preferă română. Diacritice OK în chat și docs. ASCII în print/log Rust.

### 9.5 Onestitate radicală
Dacă codul nu face ce promit documentele, spun. Dacă o opțiune are trade-off nefavorabil, spun. Dacă ceva e în afară zonei mele de expertiză, spun.

### 9.6 Execuție directă pe pași agreați
"Execută direct" se aplică DOAR la implementarea pașilor concreti deja agreați. NU la decizii arhitecturale.

### 9.7 Test după fiecare modificare
Cargo build + cargo test pe l0/. Cargo workspace pentru toate crate-urile.

### 9.8 Commit + push după fiecare fază completă
Fără commits intermediare needescă. Mesaj de commit clar, descriere "why".

### 9.9 Fără estimări de timp
Nu mai pun "săptămâni", "luni", "X effort" în docs sau plan-uri. Doar ordine de execuție + dependențe.

---

## 10. COMMIT ATTRIBUTION

```
Code implemented with help from AI Agents Claude, Codex, Jules (according to which generated which part).
```

Fără links, fără email-uri, fără `Co-Authored-By:`.

---

## 11. COMENZI UTILE

```bash
# Build l0 daemon
cd l0 && cargo build --release

# Test l0
cd l0 && cargo test

# Run l0 local (Linux dev)
cd l0 && cargo run

# Verifică L0 sacred files
grep -r "L0_SACRED" l0/src/

# Git status YBOS
git status
```

---

## 12. ATENȚIONĂRI

- **YBOS e proiect comercial în proiecție** — zero secrete/keys/PIN-uri în git. Niciodată.
- **L0 e sacred** — refuz hard la modificare, niciun cod nu atinge L0 sacred files.
- **Rust kernel pur = side project George**, NU YBOS road map.
- **Device test = ARM64 cu NPU**, modelul exact e flexibil (NU hardcodat Pixel 7 nicăieri în cod generic).
- **MVP include agent builder framework + user-context memory + laptop companion**, nu doar 5 agenți.
- **Telefonul = sursa de adevăr**. Laptop = terminal session-based, fără identitate proprie, zero state post-logout.
- **Fără estimări de timp** în docs.
- **Disclaimers obligatorii** la onboarding laptop pentru limitări security Tier 2 (App Mode).
- **YBOSClaude.md** trebuie invocat de George la început fiecare sesiune Claude.

---

## 13. DOCUMENTE DE CITIT ÎN ORDINE (pentru context complet)

1. `YBOSClaude.md` (acest fișier) — source of truth
2. `docs/ARCHITECTURE.md` — decizii arhitecturale detaliate (inclusiv laptop companion, user-context, task offload)
3. `docs/ROADMAP.md` — faze (Y1 detaliat, restul cu semne de întrebare)
4. `docs/HARDWARE.md` — specs device test (flexibil)
5. `docs/L0_SACRED.md` — securitate L0
6. `reference/REMUS_PORT_NOTES.md` — legătura cu RemusOS3
7. `l0/README.md` — starea Rust daemon

---

*Sesiunea 2026-05-21 (1 + 2). George și Claude au planificat împreună direcția YBOS. Acest document e contractul lor.*
