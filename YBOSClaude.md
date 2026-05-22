# YBOS — Claude Code Instructions

> Creat: 2026-05-21
> Update: 2026-05-21
> Lead Dev planning + final review: Claude (Opus 4.7)
> Code generators: Codex, Jules, alte tool-uri AI
> Final review: project architect + Claude + Codex
> Origine: pivot dintr-un prototip personal către un OS AI vandabil,
> multi-user, mobile-first cu laptop companion.

## 1. Viziune YBOS

YBOS este un AI Executive Assistant OS care:

- Rulează primar pe telefon, sursa unică de adevăr pentru identitate, K-master,
  memorie persistentă și modele locale.
- Are laptop companion app session-based, fără identitate permanentă pe laptop.
- Are agent LLM principal care orchestrează sub-agenți specializați și permite
  user-ului să creeze agenți noi.
- Este vandabil și multi-user: fiecare device primește identitate la onboarding.
- Pune privacy ca prim feature: LLM-ul principal este și paznic al datelor.
- Ține user-context memory persistent pentru preferințe recurente.
- Folosește Rust pentru userland nou și module proprii.
- Este aproape de hardware prin daemon L0 Rust + kernel modules Rust + eBPF.
- Permite Android apps native prin baza AOSP.
- Suportă cloud burst opt-in prin trait, dezactivat la MVP.

## 2. Arhitectura

### 2.1 Baza

- OS base: AOSP custom.
- Kernel: Linux mainline din AOSP, cu module Rust YBOS-specific adăugate gradual.
- Userland nou: Rust.
- App layer: Android Runtime păstrat pentru compatibilitate.
- Build: AOSP build system + Cargo workspace.

### 2.2 3-layer brain

| Layer | Nume | Rol | Implementare |
|---|---|---|---|
| L0 | Reflex Layer | Identitate per-user, telemetrie HW, reflex sub-ms | Rust daemon `ybos-l0` |
| L1 | Agentic Layer | Orchestrator multi-agent, routing, privacy firewall, user-context memory | Rust service `ybos-orchestrator` |
| L2 | Cognitive Layer | LLM principal + sub-agenți specializați | llama.cpp / mlc-llm |

Comunicare între layere: Binder pentru integrare Android-native + gRPC pentru
servicii noi. MQTT embedded rămâne transport local de dezvoltare până la
decizia L1/Binder.

### 2.3 Onboarding wizard

Prima pornire după instalare:

```text
Welcome
Name
PIN
Biometric opt-in
Hardware key opt-in
KeyGen
BIP39 display one time
Sealed
```

Reguli post-onboarding:

- L0_SACRED files devin read-only prin policy de platformă.
- Modificare identity-critical necesită envelope valid.
- L0 SACRED files primesc refuz sintactic pentru orice write automat.
- Pierderea tuturor envelope-urilor + paper backup înseamnă wipe necesar.

### 2.4 Laptop Companion

Telefonul este sursa de adevăr. Laptopul este terminal temporar cu resurse extra.
Pairing-ul real prin QR/NFC vine într-o fază viitoare. Y1 livrează doar hook-ul
L0 pentru session token issuance.

### 2.5 User-Context Memory

Layer separat în L1, local pe telefon, accesibil agenților prin capability
`data.user_prefs`. Laptopul primește doar cache temporar criptat pe durata
sesiunii.

## 3. Stack tehnic fixat

| Componentă | Alegere |
|---|---|
| OS base | AOSP custom |
| Kernel | Linux mainline din AOSP + module Rust |
| Userland nou | Rust 1.75+ |
| L0 daemon | `ybos-l0` |
| L1 orchestrator | `ybos-orchestrator` |
| L2 inference | llama.cpp + mlc-llm |
| Vector DB | sqlite-vss sau qdrant embedded |
| User-context store | sqlite + embeddings |
| Privacy firewall | eBPF Rust + LSM hooks |
| Cross-layer comms | Binder + gRPC |
| Laptop companion | Tauri |
| Session crypto laptop | AES-256-GCM + HKDF din K-master |

## 4. Decizii agreate

- Rust kernel from scratch nu este roadmap YBOS.
- Device test: ARM64 cu NPU dedicat + bootloader unlockable; modelul exact este flexibil.
- Codul generic nu hardcodează device-uri concrete; device-specific code stă izolat în HAL/platform.
- MVP include 5 agenți seed + framework de creare agenți custom.
- Privacy firewall are 3 layere la MVP: capabilities, eBPF redactor, LLM judge.
- Cloud burst este trait stub la MVP, activat ulterior doar opt-in.
- Laptop companion este session-based.
- User-context memory este subsistem dedicat.
- Onboarding generalizat: L0 nu vine cu owner hardcoded.
- Project architect decide direcția finală; Claude face planning/review; Codex/Jules implementează task-uri.
- Docs nu includ estimări de timp.

## 5. Structură repository

```text
YBOS/
├── README.md
├── YBOSClaude.md
├── docs/
│   ├── ARCHITECTURE.md
│   ├── ROADMAP.md
│   ├── HARDWARE.md
│   └── L0_SACRED.md
├── l0/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── proto/l0.proto
│   └── src/
│       ├── main.rs
│       ├── identity/
│       ├── hw/
│       ├── bus/
│       ├── grpc/
│       └── reflex/
├── orchestrator/
├── agents/
├── user_context/
├── firewall/
├── companion/
├── ui/
├── platform/
└── reference/
    └── PORT_NOTES.md
```

## 6. Y1 Scope

Y1 implementează doar:

- Generalizare `l0/src/identity/` la enrollment dinamic.
- Onboarding scaffold single-device pe Linux dev.
- Envelope A Argon2id; envelope B/C doar trait + plan documentat.
- BIP39 24 cuvinte, afișat o singură dată, cu `bip39.lock`.
- `identity_core.bin` semnat HMAC-SHA256 cu K-master.
- Session token issuance API hook + in-memory active sessions.
- Tripwire boot integrity adaptat la layout YBOS.
- Unit tests + smoke test.

Y1 nu implementează:

- TEE real.
- QR/NFC pairing flow.
- Laptop client.
- Multi-device restore.
- L1 orchestrator integration.

## 7. Reguli critice de cod

- Runtime nou în Rust; Python doar pentru tool-uri dev, nu runtime.
- Paths via `PathBuf` și `crate::identity::paths`.
- L0 SACRED refuză hard orice write automat.
- Log messages Rust trebuie să fie ASCII.
- Hardware abstraction prin HAL traits; fără constante device-specific în cod generic.
- Nu se comit identity blobs, keys, session tokens sau mnemonics.
- Capability declarations sunt obligatorii pentru agenți.
- Session keys se zeroizează la logout/revoke.

## 8. Workflow

- Citește `YBOSClaude.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`,
  `docs/L0_SACRED.md`, `reference/PORT_NOTES.md`, `l0/README.md`, apoi `l0/src/`.
- Pentru `l0/`: rulează `cargo build` și `cargo test`.
- Commit messages includ:

```text
Code implemented with help from AI Agents Claude, Codex, Jules.
```

Fără `Co-Authored-By`, email-uri sau link-uri în commit message.
