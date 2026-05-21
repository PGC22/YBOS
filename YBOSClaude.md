# YBOS — Claude Code Instructions

> **Creat**: 2026-05-21 — sesiunea de planificare arhitectură cu George (PGC22)
> **Lead Dev (planning + final review)**: Claude (Opus 4.7)
> **Code generators**: Codex, Jules, alte tool-uri AI (George decide per task)
> **Final review**: George + Claude + Codex (eventual un dev uman)
> **Origine**: pivot din RemusOS3 — un OS sentient personal pentru George — către **YBOS**, un OS AI vandabil, multi-user, mobile-first.

---

## 1. VIZIUNE YBOS

**YBOS este un AI Executive Assistant OS** care:

- Rulează pe **telefoane primar**, laptopuri secundar (simbioză cross-device prin telemetrie + sync)
- Trimite un **agent LLM principal** ca orchestrator care **delegă** către sub-agenți specializați (calendar, business trip, learning, market intel, news)
- E **vandabil** — multi-user, fiecare device primește identitate la onboarding (NU vine cu owner hardcoded ca Remus)
- Are **privacy ca prim feature** — LLM-ul main agent este și paznic al datelor (firewall în 3 layere)
- Este **scris în Rust** pentru tot userland-ul nou + module kernel proprii
- E **aproape de hardware** (sub-ms reflex) prin daemon L0 Rust + kernel modules Rust + eBPF programs
- Permite **Android apps native** prin baza AOSP (Google Play apps merg din ziua 1)
- Suportă **cloud burst** opt-in pentru LLM mari (design-uit ca trait din zi 1, dezactivat la MVP)

**Diferentiator vs Apple Intelligence / Gemini / Bixby:** noi nu trimitem date în cloud fără consimțământ explicit per-task. LLM-ul principal e și paznic, nu doar asistent.

---

## 2. ARHITECTURA OS-ULUI

### 2.1 Baza: AOSP custom (decizie 2026-05-21)

**Kernel**: Linux mainline (cel din AOSP), cu module Rust YBOS-specific adăugate gradual.
**Userland**: TOT Rust pentru servicii noi YBOS. Android system services păstrate doar cât e necesar pentru runtime apps.
**App layer**: Android Runtime (ART) păstrat → Google Play apps merg nativ. Apps YBOS-native scrise în Rust (Jetpack Compose binding sau Slint).
**Build**: AOSP build system + Cargo workspace pentru toate crate-urile Rust.

**De ce nu Rust kernel from scratch**: am amortiza 3-5 ani înainte de demo. AOSP ne dă day-1 hardware support + Android apps. Rust kernel pur rămâne **side project personal George**, nu road map YBOS.

**Migrare graduală către Rust în kernel**: Linux 6.x suportă cod Rust nativ. În MVP avem kernel modules Rust pentru policy/firewall. În v0.5+ înlocuim HAL-uri specifice cu Rust. La v2+ contribuim upstream Rust-for-Linux. La v3+ (5-10 ani) putem evalua fork Redox dacă are sens.

### 2.2 3-layer brain (păstrat din Remus, generalizat)

| Layer | Nume | Rol | Implementare |
|---|---|---|---|
| **L0** | Reflex Layer | Identitate per-user, telemetrie HW, reflex sub-ms | Rust daemon (`ybos-l0`), portat din Remus |
| **L1** | Agentic Layer | Orchestrator multi-agent, routing, privacy firewall | Rust service nou (`ybos-orchestrator`) |
| **L2** | Cognitive Layer | LLM principal + sub-agenți specializați | llama.cpp / mlc-llm pe NPU (Tensor / Hexagon) |

**Comunicare între layere**: Binder (Android-native) pentru integrare cu system services + gRPC pentru servicii noi. MQTT broker embedded pentru telemetrie L0→L1 (evaluat pentru înlocuire cu Binder la S6.6).

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

---

## 3. STACK TEHNIC FIXAT

| Componentă | Alegere | Motiv |
|---|---|---|
| OS base | AOSP 14/15 custom | Android compat + drivere mobile + kernel optimized mobile |
| Kernel | Linux mainline (cel din AOSP) + module Rust | Pragmatic, Rust upstream maturizează în paralel |
| Userland nou | Rust 1.75+ | Cerință George, safety + performance |
| L0 daemon | Rust (portat din Remus l0/) | Deja există scaffold S6.0-S6.4 |
| L1 orchestrator | Rust nou (`ybos-orchestrator`) | Bridge între L0 și agenți |
| L2 inference | llama.cpp + mlc-llm | Acceleration pe Tensor / Hexagon NPU |
| Vector DB | sqlite-vss sau qdrant embedded | Memorie semantică agenți, locale |
| Privacy firewall | eBPF Rust (aya-rs) + LSM hooks | 3 layere: capabilities + redactor + LLM judge |
| Comunicare cross-layer | Binder + gRPC (tonic) | Binder pentru Android-native, gRPC pentru cross-platform |
| Telemetrie hw → L1 | MQTT (rumqttd embedded) sau Binder | TBD S6.6 |
| UI native YBOS | Jetpack Compose (Kotlin/Rust binding) sau Slint | TBD când ajungem la UI |
| Build | AOSP build + Cargo workspace | Standard |
| App compat | Android Runtime (ART) păstrat | Google Play merge nativ |
| Cloud LLM (opt-in) | Trait `Inference` cu impl `RemoteAPI` (stub MVP) | Anthropic / OpenAI / Together / self-hosted |

---

## 4. DECIZII AGREATE (sesiune 2026-05-21)

Toate confirmate explicit de George. Nu se renegociază în sesiuni viitoare fără discuție nouă.

### 4.1 OS base: AOSP custom + Rust userland
- Rust kernel from scratch rămâne **side project personal George**, NU road map YBOS.
- Migrare graduală Rust în kernel via Rust-for-Linux upstream + module proprii.

### 4.2 Device test primar: Pixel 7+ (sau echivalent ARM64 cu NPU dedicat + bootloader unlockable)
- Vezi `docs/HARDWARE.md` pentru lista completă specs minime.
- George cumpără device-ul în următoarele săptămâni.

### 4.3 MVP scope: 5 agenți
1. **Calendar** — local + Google Calendar sync (cu permission user)
2. **Trip Planner** — flights/hotels APIs + summary + booking handoff
3. **Learning Curator** — user dă share din TikTok/IG/YouTube Shorts → picker categorii (Programming, Teambuilding, Idei, +custom) → background: Whisper transcribe + LLM extract structure (pași, resurse, sumar) → vector DB tagged → UI tip ZEST cu cards
4. **Market Intel** — agent reusable, instanțiabil per-piață (energie, tech, etc.), scrape + LLM summary
5. **News Digest** — surse whitelisted (WSJ, Reuters, Al-Jazeera, CNN, FT, etc.), morning brief

**Capabilities per agent**: fiecare declară ce domenii poate accesa, ce date poate citi. L1 firewall enforce.

### 4.4 Privacy firewall: 3 layere full la MVP
- **Layer 1**: capability-based, fiecare agent declară ce poate accesa
- **Layer 2**: eBPF redactor (Rust + aya-rs) pe net syscalls, strip PII
- **Layer 3**: LLM judge (sub-agent local mic) decide allow/redact/block/ask-user pe payload înainte de orice cloud send

### 4.5 Cloud burst: trait stub la MVP, activat la v0.2+
- Trait `Inference` cu impl `LocalLlama` (default) + `RemoteAPI` (stub)
- User aprobă per categorie în settings ("research = cloud OK", "calendar = niciodată")

### 4.6 Timeline MVP: 8-10 luni
- 6 luni baseline + 2 luni Learning Curator + 2 luni firewall full
- Accept stretch dacă apar blocaje hardware-specifice

### 4.7 Onboarding flow generalizat
- L0 NU mai vine hardcoded cu owner. Se naște la primul boot.
- 3-envelope crypto din Remus păstrat. TPM → StrongBox / Hexagon TEE. YubiKey → opt-in NFC/USB-C.

### 4.8 Roluri în proiect
- **George**: arhitect, final decision maker, validează direcția
- **Claude (Opus 4.7)**: Lead Dev, planning + final review + arhitectură detailed
- **Codex, Jules, etc.**: code generators per task
- **Dev uman (eventual)**: review periodic

---

## 5. STRUCTURĂ REPOSITORY YBOS

```
YBOS/
├── README.md                     # Public-facing, scurt
├── YBOSClaude.md                 # ACEST FIȘIER — sursa de adevăr pentru Claude
├── LICENSE                       # TBD cu George (Apache-2.0 / MIT / Proprietary)
├── .gitignore
├── docs/
│   ├── ARCHITECTURE.md           # Decizii arhitecturale detaliate
│   ├── ROADMAP.md                # Faze MVP → v2
│   ├── HARDWARE.md               # Specs minime device test
│   └── L0_SACRED.md              # Lista fișiere sacre L0
├── l0/                           # Rust daemon kernel-adjacent (portat din Remus)
│   ├── Cargo.toml                # package name: ybos-l0
│   ├── build.rs
│   ├── proto/
│   │   └── l0.proto              # gRPC services
│   ├── src/
│   │   ├── main.rs
│   │   ├── identity/             # Identity per-user (de generalizat din Remus)
│   │   ├── hw/                   # HAL telemetrie
│   │   ├── bus/                  # MQTT broker + publisher
│   │   ├── grpc/                 # gRPC services
│   │   └── reflex/               # Reflex actions (S6.5)
│   └── README.md
├── orchestrator/                 # L1 — Rust nou, TBD
├── agents/                       # Definiții agenți MVP (calendar, trip, learning, market, news)
├── firewall/                     # Privacy firewall 3 layere
├── ui/                           # UI native YBOS (TBD design)
├── platform/                     # AOSP customizations, build scripts
└── reference/
    └── REMUS_PORT_NOTES.md       # Ce am portat din RemusOS3, ce nu, de ce
```

---

## 6. CE AM PORTAT / CE AM LĂSAT DIN RemusOS3

### Portat (✅ incluse în YBOS initial commit)
- `l0/` — întregul crate Rust (Cargo.toml rebrand `ybos-l0`, restul cod păstrat pentru moment)
  - Identity + boot integrity (S6.1)
  - HAL telemetrie (S6.2)
  - MQTT broker + publisher (S6.3)
  - gRPC services (S6.4)
- Conceptul **L0_SACRED** — documentat în `docs/L0_SACRED.md`
- Conceptul **3-envelope crypto** (Argon2 + TEE + opt YubiKey) — documentat în ARCHITECTURE.md
- Conceptul **BIP39 paper backup** — same
- Conceptul **per-user identity blob** semnat HMAC — same (de generalizat din George-hardcoded la enrollment dinamic)

### NU portat (❌ rămas în RemusOS3)
- Tot codul Python (`core/*.py`, `web_interface.py`, `interface.py`) — înlocuit cu Rust
- ChromaDB memory — înlocuit cu sqlite-vss / qdrant
- Flask web UI — înlocuit cu native Android UI
- Deploy NixOS / Buildroot — înlocuit cu AOSP build
- "Sentient" stuff (mood, dreams, journal) — nu MVP, eventual v2+ ca feature opt-in
- T460-specific (BIOS, fingerprint reader Validity) — N/A pe mobil
- George-hardcoded ownership — înlocuit cu onboarding wizard
- Roluri PRIMARY/SATELLITE/LIVE — reproiectate pentru cross-device "simbioza"

### De adaptat în sprint-urile următoare (⚠️ comentate-n cod)
- `l0/src/identity/` — de generalizat din George-only la enrollment dinamic la onboarding
- `l0/src/identity/sacred.rs` — paths actualizate la layout YBOS
- `l0/src/bus/` — evaluat de înlocuit cu Binder pe Android (S6.6 generalizat)

---

## 7. ROADMAP YBOS (faze)

Vezi `docs/ROADMAP.md` pentru detalii. Sumar:

| Fază | Nume | Effort | Status |
|---|---|---|---|
| Y0 | Repo bootstrap, structură, docs, port l0/ ca-i | 1 zi | ✅ În progres (acum) |
| Y1 | L0 generalizare: identity per-user, onboarding flow scaffold | 3-4 săpt | TBD |
| Y2 | AOSP build environment + Pixel device flashing first boot | 4-6 săpt | TBD |
| Y3 | L1 orchestrator skeleton + Binder integration | 4 săpt | TBD |
| Y4 | LLM inference layer (llama.cpp/mlc-llm pe NPU) | 4 săpt | TBD |
| Y5 | Agent 1: Calendar (end-to-end demo) | 3 săpt | TBD |
| Y6 | Agent 2: News Digest | 2 săpt | TBD |
| Y7 | Privacy firewall Layer 1 (capabilities) | 2 săpt | TBD |
| Y8 | Privacy firewall Layer 2 (eBPF redactor) | 4-6 săpt | TBD |
| Y9 | Privacy firewall Layer 3 (LLM judge) | 3 săpt | TBD |
| Y10 | Agent 3: Trip Planner | 3 săpt | TBD |
| Y11 | Agent 4: Market Intel | 3 săpt | TBD |
| Y12 | Agent 5: Learning Curator (share intent + decompose) | 6-8 săpt | TBD |
| Y13 | UI native YBOS launcher + onboarding wizard | 4-6 săpt | TBD |
| Y14 | Cross-device "simbioza" (laptop <-> phone sync) | 4 săpt | TBD |
| Y15 | Cloud burst trait activation (opt-in per categorie) | 2 săpt | v0.2 |

**MVP demo target**: Faza Y0 → Y13. Estimated 8-10 luni cu paralelizare prin generatoare AI cod.

---

## 8. REGULI CRITICE DE COD

### 8.1 Toate sursele noi sunt Rust
Fără Python în YBOS. În YBOS, Python e doar pentru tool-uri dev internal (build scripts, etc.), nu pentru runtime.

### 8.2 Paths via PathBuf, niciodată hardcoded strings
```rust
// GRESIT
fs::read_to_string("config/identity_core.bin")

// CORECT
use crate::identity::paths;
fs::read_to_string(paths::identity_blob())
```

### 8.3 L0 SACRED — refuz hard
Vezi `docs/L0_SACRED.md`. Niciun cod self-improvement, OTA update, agent skill discovery nu atinge L0 sacred files. Verifică `is_l0_sacred()` înainte de orice scriere.

### 8.4 No diacritics in print()/println!/eprintln!
Păstrat din Remus (lecție Windows cp1252 dev environment). Diacriticele OK în strings retur, fișiere UTF-8, docstrings.

### 8.5 Gitignored — niciodată commit fără confirmare
- Identitate per-user (blob, salt, BIP39 paper backup digital)
- Keys, certificates
- Build artifacts (target/, out/, .apk, .img)
- Cache local

### 8.6 Capability declarations obligatorii pentru agenți
Fiecare agent are `manifest.toml` cu capabilities (`net.domains`, `fs.paths`, `data.types`). L1 refuză orice operație ne-declarată.

---

## 9. COMPORTAMENT AȘTEPTAT DE LA CLAUDE

### 9.1 Lead Dev, dar George e arhitect
Eu propun direcția tehnică detaliată (Lead Dev role). George validează / redirectează / decide final. Când am dubii dacă o decizie e în zona mea sau a lui, întreb înainte.

### 9.2 Antipattern — NU iau decizii strategice singur
Păstrat din CLAUDE.md Remus §9.1. Decizii care CER întrebare explicită (lista nu exhaustivă):
- Toolchain, runtime, framework nou
- Locație fișier/director major
- Dependențe externe noi
- Pattern arhitectural (Binder vs gRPC, embedded broker vs extern, etc.)
- Interpretare ambiguă a răspunsului lui George ca decizie

### 9.3 Antipattern — nu derutez utilizatorul cu termen ambigu
Păstrat din CLAUDE.md Remus §9.2. Explic terminologia înainte de opțiuni. Nu presupun că termenii ("OS", "kernel", "platform", "scope") au același sens pentru George.

### 9.4 Comunicare în română
George preferă română. Diacritice OK în chat și docs. ASCII în print/log Rust (lecție dev environment).

### 9.5 Onestitate radicală
Dacă codul nu face ce promit documentele, spun. Dacă o opțiune are trade-off nefavorabil, spun. Dacă ceva e în afară zonei mele de expertiză, spun.

### 9.6 Execuție directă pe pași agreați
"Execută direct" se aplică DOAR la implementarea pașilor concreti deja agreați. NU la decizii arhitecturale.

### 9.7 Test după fiecare modificare
Cargo build + cargo test pe l0/. Cargo workspace pentru toate crate-urile.

### 9.8 Commit + push după fiecare fază completă
Fără commits intermediare needescă. Mesaj de commit clar, descriere "why".

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

# Push (după ce George creează repo gol pe GitHub)
git remote add origin <URL>
git push -u origin main
```

---

## 12. ATENȚIONĂRI

- **YBOS e proiect comercial în proiecție** — zero secrete/keys/PIN-uri în git. Niciodată.
- **L0 e sacred** — repetat. Refuz hard la modificare, niciun cod nu atinge L0 sacred files.
- **Rust kernel pur = side project George**, NU YBOS road map.
- **Decizii sesiunii 2026-05-21 sunt fixe** — nu se renegociază fără sesiune nouă.
- **YBOSClaude.md** trebuie invocat de George la început fiecare sesiune Claude. Eu nu pot ști contextul YBOS fără acest fișier încărcat.

---

## 13. DOCUMENTE DE CITIT ÎN ORDINE (pentru context complet)

1. `YBOSClaude.md` (acest fișier) — source of truth
2. `docs/ARCHITECTURE.md` — decizii arhitecturale detaliate
3. `docs/ROADMAP.md` — faze + timeline
4. `docs/HARDWARE.md` — specs device test
5. `docs/L0_SACRED.md` — securitate L0
6. `reference/REMUS_PORT_NOTES.md` — legătura cu RemusOS3
7. `l0/README.md` — starea Rust daemon

---

*Începutul sesiunii 2026-05-21. George și Claude au planificat împreună direcția YBOS. Acest document e contractul lor.*
