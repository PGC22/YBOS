# YBOS Roadmap

> Detaliat doar pentru Y0 (în progres) și Y1 (next). Restul fazelor sunt enumerate succint, cu semne de întrebare doar acolo unde decizia afectează arhitectura/implementarea din Y1.
>
> **Fără estimări de timp.** Ordinea fazelor + dependențele contează; timpul real e irelevant până când produsul devine vandabil.

---

## Y0 — Bootstrap ✅ În progres

- Repo YBOS creat (public, github.com/PGC22/YBOS)
- Structură directoare + docs scrise
- l0/ portat din prototipul inițial (Cargo.toml rebrand `ybos-l0`)
- YBOSClaude.md = source of truth context
- Arhitectură detailed (inclusiv laptop companion + user-context memory + task offload)

**Acceptance**: `git clone` + cititul `YBOSClaude.md` dă context complet oricărui Claude/dev.

---

## Y1 — L0 generalizare ⭐ NEXT

> Singura fază cu detaliu complet acum. Scope-ul e definit ca să livreze identity-ul generalizat + hook-uri arhitecturale pentru tot ce vine după.

### Scope

1. **Generalizare `l0/src/identity/` din owner hardcoded la enrollment dinamic**
   - `Identity` struct generic (nume, UUID generated, biometric_template_public, created_at)
   - Eliminare referințe text la owner/prototip din log messages + identitate
   - Layout paths nou: `${YBOS_DATA}/identity/...` în loc de paths de prototip
   - `sacred.rs` lista actualizată la layout YBOS

2. **Onboarding flow scaffold (Rust, single-device)**
   - State machine: Welcome → Name → PIN → Biometric (opt) → YubiKey (opt) → KeyGen → BIP39 display → Sealed
   - Argon2id pentru envelope A (PIN + biometric_template + device_fingerprint)
   - Stub pentru envelope B (TEE seal) — interfața + plan documentat, implementare reală în fază AOSP
   - Stub pentru envelope C (YubiKey HMAC) — interfața + plan
   - BIP39 mnemonic generation + display logic (afișat o singură dată, marker în `bip39.lock`)
   - Identity blob signed HMAC cu K, scris în `identity_core.bin`

3. **API pentru session token issuance (HOOK, NU implementare completă)**
   - Funcție `issue_session_token(scope, expiry, peer_fingerprint) -> SessionToken`
   - HKDF derivation din K-master cu salt aleator + epoca timpului
   - Storage in-memory pentru lista sesiuni active (persisted dacă necesar pentru reboot recovery)
   - API pentru `revoke_session(session_id)` + `revoke_all()`
   - **NU implementăm QR/NFC pairing flow aici** — doar API-ul intern. Pairing-ul e o fază viitoare cu laptop companion.

4. **Tripwire boot integrity (păstrat din prototip, adaptat)**
   - Hash check pe L0 SACRED files
   - Hash check pe lista L0_SACRED însăși (anti-tamper)
   - Boot blocat dacă mismatch

5. **Tests + smoke**
   - `cargo test` verde pentru identity enrollment, BIP39, HMAC, session token issuance API
   - Smoke test pe Linux dev: rulează onboarding scaffold end-to-end fără device real, generează identity, verifică integrity la "reboot" simulat

### Acceptance criteria Y1

- [ ] `cargo build && cargo test` în l0/ verde
- [ ] Identity enrollment cu PIN funcționează pe Linux dev (simulat fără TEE/YubiKey)
- [ ] BIP39 mnemonic generat, afișat, marker `bip39.lock` scris
- [ ] Session token API testabil unitar (issue + revoke)
- [ ] Tripwire detectează modificare a oricărui L0 SACRED file
- [ ] Zero referințe hardcoded la owner/prototip în code/docs
- [ ] Plan TEE binding documentat în `docs/ARCHITECTURE.md` §2.1 (StrongBox/Hexagon API research, fără implementare)

### Ce NU intrare în Y1

- Implementare TEE reală (vine în fază AOSP build când avem device real)
- QR/NFC pairing flow (vine în fază laptop companion)
- Multi-device identity restore (vine post-MVP)
- L1 orchestrator integration (fază separată)

---

## Y2+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Detaliu va fi adăugat pe măsură ce ne apropiem de fiecare fază. Semne de întrebare doar unde **chiar afectează Y1**.

- **AOSP build environment** — setup build host, sync AOSP, flash device test (model TBD per achiziție). Afectează Y1? **NU** — Y1 rulează pe Linux dev.
- **L1 orchestrator skeleton** — `ybos-orchestrator` crate, capability enforcement, agent registry. ❓ Hook pentru Agent Builder runtime registration: design-uim API-ul L1 în acea fază astfel încât Y1 session_token API să se integreze curat.
- **LLM inference layer** — llama.cpp + mlc-llm pe NPU. Afectează Y1? **NU**.
- **Agent seed: Calendar** — primul agent end-to-end demo. Afectează Y1? **NU**.
- **Agent seed: News Digest**. **NU** afectează Y1.
- **Privacy firewall Layer 1 (capabilities)**. **NU** afectează Y1.
- **Privacy firewall Layer 2 (eBPF redactor)**. **NU** afectează Y1.
- **Privacy firewall Layer 3 (LLM judge)**. **NU** afectează Y1.
- **Agent seed: Trip Planner**. **NU** afectează Y1.
- **Agent seed: Market Intel**. **NU** afectează Y1.
- **Agent seed: Learning Curator**. **NU** afectează Y1.
- **Agent Builder Framework** — template + LLM-assisted configurator + UI flow. **NU** afectează Y1 direct (afectează L1 design).
- **User-Context Memory subsystem** — storage + sync + capability `data.user_prefs`. **NU** afectează Y1.
- **Laptop Companion (Tauri)** — pairing QR/NFC + session crypto + task offload + cache sync. ❓ **Y1 trebuie să livreze API session token issuance** ca această fază să nu refactoreze identity-ul. Punct critic — vezi Y1 scope §3.
- **UI native YBOS mobile** — launcher, onboarding wizard UI, agent dashboards, agent builder UI. **NU** afectează Y1.
- **Cross-device extins** (multi-phone, tabletă) — post-MVP. **NU** afectează Y1.
- **Cloud burst activation** — v0.2+. **NU** afectează Y1.
- **VM Mode (Tier 1) laptop** — Linux VM minim, GPU passthrough, SEV-SNP/TDX integration. Research item, post-MVP. **NU** afectează Y1.
- **Split inference layer-by-layer** ❓ research item (vezi ARCHITECTURE.md §4.5). Independent, când/dacă apare hardware potrivit. **NU** afectează Y1.

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
