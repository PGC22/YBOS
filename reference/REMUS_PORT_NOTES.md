# RemusOS3 → YBOS Port Notes

> Sesiunea: 2026-05-21
> Decision-maker: George (PGC22)
> Lead Dev: Claude (Opus 4.7)

Acest document explică ce am luat din [RemusOS3](https://github.com/PGC22/RemusOS3) la crearea YBOS, ce am lăsat, și de ce.

---

## Context

**RemusOS3** este OS-ul AI personal al lui George, construit pe Linux kernel, target ThinkPad T460, single-user (George hardcoded ca owner). Are 3-layer brain (L0/L1/L2), L0 portat în Rust până la S6.4 (gRPC services done).

**YBOS** e pivot-ul spre produs vandabil: mobile-first, multi-user, AOSP base, executive assistant cu multi-agent + privacy firewall.

George a decis explicit (2026-05-21) ce să luăm și ce să lăsăm.

---

## Ce am portat (✅)

### 1. `l0/` — întregul crate Rust
**Acțiune**: copy complet din `RemusOS3/l0/` în `YBOS/l0/`, schimbat doar Cargo.toml (rebrand `remus-l0` → `ybos-l0`).

**Conținut portat:**
- `Cargo.toml`, `build.rs`, `proto/l0.proto`
- `src/main.rs`, `src/bus/`, `src/grpc/`, `src/hw/`, `src/identity/`, `src/reflex/`
- `README.md`

**Status sprint-uri în Remus la moment port:**
- ✅ S6.0 Scaffold
- ✅ S6.1 Identity + boot integrity (Rust port din Python)
- ✅ S6.2 HAL telemetrie statică
- ✅ S6.3 MQTT broker rumqttd + publisher rumqttc
- ✅ S6.4 gRPC server (tonic) cu IdentityService + TelemetryService + ReflexService stub

**Adaptări planificate în Y1 (sprint următor)**:
- Generalizare `identity/` din George-hardcoded la enrollment dinamic
- Update paths în `paths.rs` pentru layout YBOS Android
- Adaptare `sacred.rs` lista L0_SACRED pentru fișiere YBOS (nu Remus)
- Evaluare MQTT vs Binder pentru bus L0→L1 (Android-native ar fi Binder)

### 2. Conceptul L0 SACRED
**Acțiune**: documentat în `docs/L0_SACRED.md`, adaptat pentru:
- Multi-user (lista include fișiere per-user generate la onboarding, nu George-specific)
- Android storage protection (SELinux + fs-verity, nu doar `chattr +i`)
- TEE binding (StrongBox / Hexagon, nu doar TPM discrete)

### 3. Conceptul 3-envelope crypto
**Acțiune**: documentat în `docs/ARCHITECTURE.md` §2.1 și `docs/L0_SACRED.md`.

Conceptul Remus:
- Envelope A: Argon2id(PIN + voice + fingerprint)
- Envelope B: TPM seal (PCR-bound)
- Envelope C: YubiKey HMAC-SHA1

Adaptare YBOS:
- Envelope A: Argon2id(PIN + biometric_template + device_fingerprint) — biometric e fingerprint/face/voice prin Android keystore
- Envelope B: StrongBox (Pixel) / Hexagon TEE (Snapdragon) seal
- Envelope C: YubiKey 5C NFC sau Yubikey 5C USB-C — opt-in (Remus avea mandatoriu)

### 4. BIP39 paper backup
**Acțiune**: păstrat ca strategie. 24 cuvinte afișate o singură dată la onboarding, user scrie pe hârtie.

### 5. Identity blob signed HMAC
**Acțiune**: păstrat. `identity_core.bin` semnat HMAC cu K. Verificare la boot.

### 6. Tripwire boot integrity
**Acțiune**: păstrat. Hash check pe L0 SACRED files + hash check pe lista L0_SACRED însăși.

### 7. Sprint workflow numeration (S6.x)
**Acțiune**: păstrat ca convenție. Y1 = generalizare identity, Y2 = AOSP build, etc.

---

## Ce am lăsat (❌)

### 1. Tot codul Python
**Fișiere lăsate**: `core/*.py` (5.2k linii), `web_interface.py` (2065 linii Flask UI), `interface.py` (740 linii TUI), `self_improvement/`, `skills/`, etc.

**De ce**: YBOS e Rust-only pentru runtime. Python rămâne în dev tools internal max.

**Înlocuiri Rust planificate:**
- `core/orchestrator.py` → `orchestrator/` (Rust crate nou, L1)
- `core/brain.py` → distribuit în L1 + L2 ca agenți specializați
- `core/memory_vector.py` (ChromaDB) → sqlite-vss sau qdrant embedded
- `core/identity.py` (Python) → `l0/src/identity/` (deja portat în Rust S6.1)
- `core/security.py` → integrat în l0 + orchestrator
- `core/self_updater.py` → `OTA` mecanism AOSP standard (A/B partitions)
- `self_improvement/proposer.py` → "agent skill discovery" subsystem post-MVP

### 2. Web Flask UI (`web_interface.py`)
**De ce**: replaced cu native Android UI (Jetpack Compose sau Slint).

### 3. TUI Python (`interface.py`)
**De ce**: N/A pe telefon. Eventual companion Linux laptop va avea TUI propriu post-MVP, dar nu portez Python-ul.

### 4. Deploy pipelines
**Lăsate**: `deploy/nixos/`, `deploy/buildroot/`, `deploy/build_iso.sh`.

**De ce**: înlocuit cu AOSP build system + Cargo workspace. Distros Linux pe laptop vin în Y14+ ca twin, cu strategie de packaging separată.

### 5. "Sentient" stuff (mood, dreams, journal)
**Lăsate**: codul EmotionalToneEngine, journal nocturn, mood persistent.

**De ce**: YBOS e executive assistant, nu sentient companion. Mood nu adaugă valoare la calendarul sau market intel. Decizie: amânat eventual pentru v2+ ca feature opt-in.

### 6. T460-specific
**Lăsate**: BIOS-uri Lenovo, fingerprint reader Validity, ACPI tables specifice.

**De ce**: target mobile, hardware diferit.

### 7. George-hardcoded ownership
**Lăsate**: `config/identity_core.txt` cu identitate George, `config/admin_key.txt`, roluri PRIMARY/SATELLITE/LIVE legate de "ce device George e pe".

**De ce**: YBOS = onboarding dinamic. Identity per device per user.

**Adaptare planificată Y1**: rescrierea `l0/src/identity/` pentru:
- `Identity` struct generic (nu hardcoded `George`)
- Enrollment flow în onboarding
- Multi-device cu shared identity (cross-device "simbioza" Y14)

### 8. NodeJS embedded
**Lăsat**: `nodeJS/` (binaries) — Remus folosea pentru ceva tool dev.

**De ce**: N/A YBOS.

### 9. Roluri PRIMARY/SATELLITE/LIVE
**Lăsate** ca enum specific. Cross-device în YBOS e mai mature (CRDT-uri, capability negotiation) — vezi `docs/ARCHITECTURE.md` §7.

---

## Ce e de adaptat (⚠️) — sprint-uri viitoare

### Y1 — Generalizare identity
Cod portat ca-i acum, dar conține referințe George-specific care vor fi rescrise:
- `l0/src/identity/blob.rs` — currently format Remus, va fi generalizat
- `l0/src/identity/paths.rs` — paths Linux/NixOS, va fi Android-friendly
- `l0/src/identity/sacred.rs` — lista L0_SACRED actuală e Remus, va fi YBOS
- `l0/src/main.rs` — referințe text "Remus" în log messages

### S6.6 (în terminologie Remus) — L1 client
Codul gRPC services + MQTT din l0/ așteaptă un L1 client. Remus avea plan Python `core/l0_client.py`. YBOS îl va avea direct ca `orchestrator/` Rust crate (Y3).

---

## Diff de cod (numeric)

| Aspect | RemusOS3 (la moment port) | YBOS (initial) |
|---|---|---|
| Linii Python | ~5200 | 0 (zero) |
| Linii Rust (l0) | ~3000 (estimat S6.4) | ~3000 (portat ca-i) |
| Linii config / docs | ~3500 | ~2000 (focused, fără history) |
| Total | ~12000 | ~5000 |

Diff = ~7000 linii rezultate din pivot. Mostly Python eliminat + sentient stuff lăsat.

---

## Lessons learned din Remus, aplicate în YBOS

### Din `CLAUDE.md` §9.1 (decision-making)
- George e arhitect, Claude propune. Aplicat în YBOS prin separare clară roluri (vezi `YBOSClaude.md` §4.8).

### Din `CLAUDE.md` §9.2 (terminologie clarificată)
- "OS", "kernel", "platform" au sensuri diferite — clarificate în YBOSClaude.md de la început.

### Din L0 SACRED implementation
- Refuz sintactic, nu cerere de autorizare. Portat 1:1 în YBOS.

### Din 3-envelope crypto
- Conceptul de "K nu trăiește pe disk în clar" este filosofia centrală. Adaptat pentru mobile TEE.

### Din experiența port Python → Rust (S6.1-S6.4)
- Paritate Python ↔ Rust live demonstrată pe HMAC verify. Documentată ca pattern recomandat pentru port-uri viitoare.

---

## Update plan pentru acest document

Update după fiecare fază mare YBOS:
- Y1 (generalizare identity) — adăugare diff cod identity port
- Y2 (AOSP build) — adăugare nou environment
- Y13 (UI native) — adăugare design decizii UI vs orice schiță Remus avea
