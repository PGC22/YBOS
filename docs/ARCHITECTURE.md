# YBOS Architecture (detailed)

> Versiune: 0.1
> Data: 2026-05-21
> Sursa de adevăr deciziilor: `YBOSClaude.md` §4

---

## 1. Big picture

```
┌─────────────────────────────────────────────────────────────────┐
│                       YBOS Device (Pixel)                       │
│                                                                 │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │              User Interface (UI native YBOS)            │   │
│   │       Onboarding wizard │ Launcher │ Agent dashboards   │   │
│   └────────────────────────────┬────────────────────────────┘   │
│                                │                                │
│                                │ Binder + gRPC                  │
│                                ▼                                │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  L2 — Cognitive Layer                                   │   │
│   │  Main LLM (llama 3B/8B quant, on Tensor NPU)            │   │
│   │  Sub-agents: Calendar | Trip | Learning | Market | News │   │
│   │  Privacy Guard (LLM judge for outbound payloads)        │   │
│   └────────────────────────────┬────────────────────────────┘   │
│                                │                                │
│                                ▼                                │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  L1 — Agentic Layer (ybos-orchestrator, Rust)           │   │
│   │  Routing │ Capability enforcement │ Policy firewall     │   │
│   │  Memory (vector DB) │ Agent lifecycle                   │   │
│   └────────────────────────────┬────────────────────────────┘   │
│                                │                                │
│                                │ gRPC + MQTT                    │
│                                ▼                                │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  L0 — Reflex Layer (ybos-l0, Rust daemon)               │   │
│   │  Identity (per-user) │ HW telemetry │ Reflex actions    │   │
│   │  Boot integrity │ L0 SACRED enforcement                 │   │
│   └────────────────────────────┬────────────────────────────┘   │
│                                │                                │
│   ──────────────── Kernel boundary ─────────────────────────    │
│                                │                                │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │  Linux Kernel (mainline from AOSP)                      │   │
│   │  + YBOS kernel modules (Rust): policy, eBPF firewall    │   │
│   │  + Android HAL bridges (modem, camera, sensors, GPU)    │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. L0 — Reflex Layer

### 2.1 Identitate per-user
- La onboarding: generate `K` (master key 256-bit, o singură dată)
- 3-envelope crypto:
  - **A**: Argon2id(PIN + biometric_template + device_fingerprint, salt, t=4, m=64MiB) → unwrap K
  - **B**: TEE seal (StrongBox pe Pixel, Hexagon TEE pe Snapdragon) → automat pe device de origine
  - **C**: YubiKey HMAC-SHA1 slot 2, opt-in NFC/USB-C → unwrap K când e prezent
- BIP39 24 cuvinte = paper backup, afișat o dată la onboarding, scris pe hârtie de user
- `identity_core.bin` = nucleul identitar (nume, UUID, public part al biometric template), semnat HMAC cu K

### 2.2 L0 SACRED
- Lista hardcoded în `l0/src/identity/sacred.rs`
- Refuz sintactic la orice scriere (nu cerere de autorizare)
- Hash check la boot — dacă lista a fost modificată, boot blocat
- Pe Android: SELinux policy `restrict_l0_sacred` + immutable bit (chattr +i equivalent via libfsverity)

### 2.3 HW telemetry
- `/sys/class/hwmon/`, `/sys/class/thermal/`, `/sys/class/power_supply/`, `/proc/stat`, ACPI
- Plus Android sensors: accelero, gyro, baro, ambient light, proximity
- Publicat pe MQTT topic `ybos/telemetry/*`

### 2.4 Reflex actions (S6.5)
- cpufreq governor write (`/sys/devices/system/cpu/.../scaling_governor`)
- Display brightness
- CPU throttle / unthrottle (battery low)
- Suspend / wake

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
io.notifications = true
```
L1 refuză orice operație ne-declarată.

### 3.3 Privacy firewall
- **Layer 1**: capabilities (Y7 MVP)
- **Layer 2**: eBPF redactor — PII strip pe net syscalls (Y8 MVP)
- **Layer 3**: LLM judge — sub-agent local mic (sub-1B params), decide allow/redact/block/ask-user pe payload outbound (Y9 MVP)

### 3.4 Memory
- Vector DB embedded (sqlite-vss sau qdrant) per agent
- Index temporal + categorial
- TTL configurabil per data type

---

## 4. L2 — Cognitive Layer

### 4.1 Main LLM
- llama 3B quantized (Q4_K_M) sau 8B dacă RAM permite
- Backend: llama.cpp (CPU baseline) + mlc-llm (Tensor NPU acceleration)
- Context window: 8K tokens MVP, extensibil

### 4.2 Sub-agents
Fiecare are own LLM context (instance separată) + memory + tools. Cei mici (Privacy Guard, classifiers) pot rula pe modele <1B distillate.

### 4.3 Cloud burst (v0.2+)
Trait Rust:
```rust
trait Inference {
    async fn complete(&self, prompt: &str, max_tokens: usize) -> Result<String>;
}
struct LocalLlama { /* ... */ }
struct RemoteAPI { endpoint: String, api_key: SecretString, /* ... */ }
```
User aprobă per categorie în settings. Default OFF la MVP.

---

## 5. App compatibility

### 5.1 Android Runtime preserved
- ART păstrat din AOSP
- Google Play Services opt-in (MicroG default pentru privacy, Google opt-in)
- Apps Android rulează în "Apps" tab (separate de UI YBOS native)

### 5.2 YBOS-native apps
- Scrise în Rust + UI framework (Slint sau Jetpack Compose cu Rust binding)
- Inter-process via Binder

---

## 6. Build & deploy

### 6.1 AOSP build environment
- Branch: android-14-release sau android-15 când stabil
- Custom manifests în `platform/manifests/`
- Build target: Pixel 7 (gs101) initial

### 6.2 Cargo workspace
- Root `Cargo.toml` cu workspace members: l0, orchestrator, agents/*, firewall, ui
- Shared dependencies în `[workspace.dependencies]`

### 6.3 OTA updates
- Atomic A/B partitions (AOSP standard)
- Self-rollback la boot failure
- Updates semnate cu cheia YBOS (TBD: ceremony de signing)

---

## 7. Cross-device "simbioza" (Y14)

### 7.1 Discovery
- mDNS / DNS-SD pe rețea locală
- Bluetooth LE advertising pentru proximity
- NFC tap pentru pairing inițial

### 7.2 Identity exchange
- Mutual TLS cu cert-uri derivate din K (cross-device fără leak)
- Capability negotiation: ce poate face fiecare device

### 7.3 State sync
- CRDT-uri pentru calendar, notes (eventual consistency)
- Last-write-wins pentru config simple
- Per-agent sync policy

---

## 8. Decizii de re-evaluat în viitor (parking lot)

- **MQTT vs Binder pentru telemetrie L0→L1** — la S6.6 generalizat
- **UI framework** — Slint vs Jetpack Compose binding — TBD când ajungem la UI
- **LLM model size** — 3B vs 8B trade-off RAM/quality
- **Vector DB choice** — sqlite-vss vs qdrant embedded — benchmark needed
- **License** — Apache 2.0 vs MIT vs Proprietary — TBD George decision business model
- **Nume YBOS final + branding** — TBD post-MVP technical bring-up
