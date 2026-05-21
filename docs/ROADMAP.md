# YBOS Roadmap

> MVP target: **8-10 luni** de la start
> Demo target: Pixel 7 boot → onboarding → conversație cu main agent → 5 agenți activi → privacy firewall demonstrabil

---

## MVP Phases

### Y0 — Bootstrap (1 zi) ✅ În progres
- Repo YBOS creat (public, github.com/PGC22/YBOS)
- Structură directoare + docs scrise
- l0/ portat din RemusOS3 (Cargo.toml rebrand `ybos-l0`)
- YBOSClaude.md = source of truth context

**Acceptance**: `git clone` + cititul `YBOSClaude.md` dă context complet oricărui Claude/dev.

---

### Y1 — L0 generalizare (3-4 săpt)
- Generalizare `identity/` din George-hardcoded la enrollment dinamic
- Onboarding flow scaffold (Rust): wizard logic, key generation, 3-envelope crypto
- BIP39 mnemonic generation + display
- TEE binding plan (StrongBox API research)
- L0 SACRED enforcement adaptat pentru Android (SELinux policy draft)

**Acceptance**: `cargo test` în l0/ verde, identity enrollment cu PIN funcționează pe Linux dev, plan TEE scris.

---

### Y2 — AOSP build environment (4-6 săpt)
- Setup AOSP build host (Ubuntu LTS în VM/cloud, 200GB+ disk, 32GB+ RAM)
- Sync sursa AOSP 14/15
- Build target: Pixel 7 stock GSI mai întâi (verificare environment)
- Apoi: minimal AOSP modifications pentru YBOS hostname/branding
- Flash pe device test
- ybos-l0 cross-compiled aarch64, instalat ca system service

**Acceptance**: Pixel 7 bootează în "YBOS" customizat, ybos-l0 rulează ca daemon, telemetria curge.

---

### Y3 — L1 orchestrator skeleton (4 săpt)
- `ybos-orchestrator` Rust crate creat
- Binder service definition (AIDL → Rust binding)
- gRPC server pentru cross-Linux compat
- Capability enforcement layer 1
- Agent registry + manifest.toml parsing

**Acceptance**: orchestrator înregistrează un "hello-world" agent, refuză cereri ne-declarate, returnează raport capabilities.

---

### Y4 — LLM inference layer (4 săpt)
- llama.cpp + mlc-llm integration
- Modele test: llama 3B Q4_K_M, phi-3 mini
- NPU acceleration pe Tensor G2/G3 (mlc-llm pipeline)
- Memory: sqlite-vss embedded
- Streaming responses

**Acceptance**: prompt → response în <5s pe device cu modelul 3B, vector store funcțional, RAM usage <3GB.

---

### Y5 — Agent 1: Calendar (3 săpt)
- Calendar agent scaffold (Rust)
- Local calendar storage
- Google Calendar OAuth + sync (cu user consent flow)
- Tools: create/list/update/delete events
- LLM integration: "set up meeting cu X mâine la 10" → tool calls

**Acceptance**: user spune "programează ședință cu mama luni 14:00", agent creează event, sync cu Google Calendar funcțional.

---

### Y6 — Agent 2: News Digest (2 săpt)
- News agent scaffold
- Whitelist surse: WSJ, Reuters, Al-Jazeera, CNN, FT, BBC, AP (configurabil per user)
- RSS / API fetchers
- LLM summarization per categorie
- Morning brief notification

**Acceptance**: "ce e nou azi în energie?" → 5-bullet summary cu surse.

---

### Y7 — Privacy firewall Layer 1: capabilities (2 săpt)
- Enforce strict pe orchestrator
- UI vizualizare capabilities active
- Audit log: ce agent a accesat ce, când
- Block + notify pe încălcare

**Acceptance**: Calendar agent încearcă `wget google.com/something-nondeclared` → blocked + log.

---

### Y8 — Privacy firewall Layer 2: eBPF redactor (4-6 săpt)
- aya-rs setup pe Pixel kernel
- BPF program: hook `connect()`, `sendmsg()` syscalls
- PII patterns: email regex, phone E.164, locație precisă (GPS coords)
- Redactor: strip PII din payload, log eveniment
- Performance test: throughput >10MB/s acceptabil

**Acceptance**: agent trimite "ridică-mă de la 45.123,25.456" → outbound stripped la "ridică-mă de la <LOCATION>", agent vede răspuns ok, log arată redactare.

---

### Y9 — Privacy firewall Layer 3: LLM judge (3 săpt)
- Sub-model "Privacy Guard" — phi-3 mini sau distillation
- Judecă payload-uri outbound înainte de send
- Output: allow / redact / block / ask-user
- UI prompt user când "ask-user"

**Acceptance**: agent cere cloud burst pentru market data → LLM judge analizează prompt → "OK, fără PII detectat, allow" → trimite.

---

### Y10 — Agent 3: Trip Planner (3 săpt)
- Flights/hotels APIs (Amadeus / Skyscanner OAuth)
- Itinerary planner cu LLM
- Booking handoff (link user-driven, no auto-purchase MVP)
- Calendar integration pentru meeting trips

**Acceptance**: "vreau să zbor în Berlin săptămâna viitoare pentru 3 zile, business meeting cu X" → propunere itinerar + flight options.

---

### Y11 — Agent 4: Market Intel (3 săpt)
- Reusable agent template (instanțiat per piață: energie, tech, etc.)
- Surse: company filings, market data APIs (yfinance, polygon.io)
- Daily/weekly reports
- Vector memory per piață

**Acceptance**: "fă-mi un raport pe piața de energie europeană luna asta" → 2-3 page summary cu surse.

---

### Y12 — Agent 5: Learning Curator (6-8 săpt)
- Share intent: "Share to YBOS" din TikTok/IG/YouTube Shorts
- Picker UI categorii (Programming, Teambuilding, Idei, +custom)
- Background: download video → Whisper transcribe → LLM extract structure
- Output: card cu pași, resurse, sumar, action items
- UI tip ZEST: browse cards per categorie

**Acceptance**: user dă share unui reel "How to deploy with Docker", alege "Programming" → card apare cu pași listed.

---

### Y13 — UI native YBOS (4-6 săpt)
- Launcher (replace SystemUI default)
- Onboarding wizard UI (cu fluxul L0 din Y1)
- Agent dashboards
- Quick chat overlay (gesture sau button hw)
- Settings (capability management, cloud burst toggles)

**Acceptance**: Pixel 7 fresh flash → boot direct în onboarding wizard YBOS → user completează → ajunge în launcher → conversație cu main agent funcțională.

---

### Y14 — Cross-device "simbioza" (4 săpt, post-MVP)
- mDNS discovery
- Cert exchange (signed cu K-derived ephemeral key)
- CRDT calendar sync între phone + laptop
- "Continuum" feature: încep conversația pe telefon, continui pe laptop

**Acceptance**: user are YBOS pe Pixel + laptop test, fac sync, calendar e replicat.

---

### Y15 — Cloud burst activation (2 săpt, v0.2)
- RemoteAPI impl funcțional (Anthropic API initial)
- Per-category toggle în UI
- Privacy Guard verifică payload înainte
- Cost tracking + budget alerts

**Acceptance**: user activează "research = cloud OK" → market intel folosește Claude API → response calitate superioară on-device.

---

## Post-MVP (v0.3+)

- iOS app companion (read-only, view dashboards)
- Linux distro twin (laptop) cu același Rust core
- Multi-tenancy laptop (multi-user per device)
- Plugin SDK pentru agenți third-party (cu privacy review obligatoriu)
- Marketplace agenți
- B2B enterprise features (audit, MDM)

---

## Effort distribution (estimat)

| Categorie | Eff total | % din MVP |
|---|---|---|
| L0 + onboarding | 3-4 săpt | 8% |
| AOSP build + device | 4-6 săpt | 12% |
| L1 orchestrator + capabilities | 6 săpt | 13% |
| LLM inference + memory | 4 săpt | 8% |
| 5 agenți | 17-19 săpt | 38% |
| Privacy firewall (3 layere) | 9-13 săpt | 22% |
| UI native | 4-6 săpt | 12% |
| Buffer / unknowns | 4 săpt | ~10% |
| **TOTAL MVP** | **~42-52 săpt** | **8-10 luni** |

Asumare: paralelizare prin generatoare AI cod (Codex, Jules), George ca arhitect full-time, Claude review continuu.
