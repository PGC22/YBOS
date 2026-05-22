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

## Y2 — Build environment + cross-compile + AOSP customization scaffolding ⭐ NEXT

> **Constrângere cunoscută**: device-ul fizic nu e încă achiziționat. Y2 livrează TOT ce se poate face *fără device fizic*. Verificarea finală end-to-end (Y2.b: flash + boot) e blocată pe achiziție device și o execută George manual când ajunge.

### Scope Y2 (device-agnostic prep)

1. **AOSP build host preparation — scripts + documentation, NOT executed în CI**
   - `platform/build_host/setup_ubuntu.sh` — script Bash pentru Ubuntu 22.04 LTS care instalează prerequisites (JDK 11/17, repo tool, build-essential, ccache, git-lfs, etc.) conform AOSP official docs
   - `platform/build_host/README.md` — documentație clară: minim 32GB RAM / 200GB disk / octa-core CPU, recomandare cloud VM (Hetzner CCX33 / AWS c6i.4xlarge / Azure equivalent) pentru build session
   - Script idempotent: rulat de 2x nu strică nimic
   - NU se rulează în CI YBOS (timpii / resursele sunt prohibitive); doar verificare lexicală (shellcheck în CI dacă pus)

2. **AOSP source sync workflow**
   - `platform/manifests/ybos-aosp.xml` — manifest custom care extinde android-14.0.0_r1 (sau ultima stabilă) cu remote-uri proprii YBOS pentru orice fork de proiecte AOSP (gol la început, dar structura pregătită)
   - `platform/sync_aosp.sh` — script `repo init -u ... -m ybos-aosp.xml && repo sync -c -j$(nproc)`
   - Documentație: paths recomandate (`~/aosp-ybos/`), disk usage estimat (~150GB după sync), cum se actualizează (`repo sync` incremental)

3. **Cross-compile ybos-l0 pentru aarch64-linux-android**
   - Update `l0/Cargo.toml` cu profile-uri și target config (`[target.aarch64-linux-android]` linker / ar / etc.)
   - `l0/.cargo/config.toml` (NEW) — linker setup folosind NDK toolchain (cale parametrizată via env var `ANDROID_NDK_HOME`)
   - `l0/build_android.sh` — script wrapper care:
     - Verifică `ANDROID_NDK_HOME` setat
     - Rulează `rustup target add aarch64-linux-android` dacă lipsește
     - `cargo build --release --target aarch64-linux-android`
     - Output binary path printat la final
   - Update `l0/README.md` cu secțiune "Cross-compile aarch64" + how-to
   - CI YBOS adaugă job `cross_compile_android` care rulează build-ul (necesită NDK în CI image — alternativ, dacă NDK nu e disponibil în GitHub Actions out-of-box, folosim `cross` crate ca fallback)
   - Test minimal: cargo build target aarch64-linux-android trece, binary produs e ELF aarch64 (verificat cu `file`)

4. **AOSP customization scaffolding (DEVICE-AGNOSTIC)**
   - `platform/aosp_overlay/` — director cu fișiere care vor fi suprascrise peste source AOSP după sync:
     - `device/ybos/common/BoardConfigCommon.mk` — placeholder cu comentariu "device-specific board config in Y2.b"
     - `device/ybos/common/system.prop` — set `ro.product.brand=YBOS`, `ro.product.manufacturer=YBOS`, `ro.product.model=YBOS-DEV`
     - `device/ybos/common/init.ybos.rc` — init script care:
       - Creează `/data/ybos/` cu owner/group și SELinux context
       - Pornește serviciul `ybos-l0` ca user `ybos` (definit nou) cu capabilities limitate
       - Restart-on-crash policy
     - `device/ybos/common/sepolicy/ybos_l0.te` — SELinux policy fragment pentru ybos-l0 daemon (allow hwmon read, ACPI read, MQTT bind localhost, gRPC bind localhost; deny rest)
   - `platform/aosp_overlay/apply_overlay.sh` — script care copie overlay-ul peste AOSP source tree (cu backup + restore option)
   - DOCUMENTAȚIE clară: aceste fișiere sunt scaffolds; finalizarea lor (board-specific paths, kernel config) e Y2.b post-device.

5. **Flash procedure documentation (Y2.b prep)**
   - `platform/FLASH_PROCEDURE.md` — checklist + comenzi pentru când vine device-ul:
     - Unlock bootloader (comenzi generic ARM64 + variante per OEM: Pixel, OnePlus, Fairphone)
     - Build YBOS image (`m otapackage` sau echivalent)
     - Flash via fastboot
     - Smoke test boot: `adb shell` + verificare ybos-l0 daemon rulează
   - Documentația e generic-ARM64; secțiunile per-OEM marcate clar cu "[verificat când achiziționăm modelul X]"

6. **Tests + CI**
   - `cargo build --target aarch64-linux-android` rulează în CI (job separat)
   - `shellcheck` pe toate scriptele bash din `platform/`
   - `cargo build && cargo test` pe l0/ rămân verzi (zero regression)
   - Nu testăm AOSP build în CI (timpii / resurse imposibile)

### Acceptance criteria Y2

- [ ] `platform/build_host/setup_ubuntu.sh` există, e idempotent, shellcheck-clean
- [ ] `platform/sync_aosp.sh` + manifest `ybos-aosp.xml` există + README
- [ ] `cargo build --release --target aarch64-linux-android` reușește local (verifică George manual)
- [ ] `l0/build_android.sh` funcțional, documentat
- [ ] `platform/aosp_overlay/` cu toate fișierele scaffolds + apply script
- [ ] `platform/FLASH_PROCEDURE.md` complet pentru cazul generic ARM64
- [ ] Zero modificări în `l0/src/` care nu țin de cross-compile config
- [ ] Zero modificări în docs/* sau YBOSClaude.md
- [ ] CI pass (cargo build + cargo test + cross-compile job + shellcheck)

### Ce NU intra în Y2

- Flash pe device fizic (Y2.b — blocked pe achiziție device)
- Boot verification end-to-end (Y2.b)
- Device-specific kernel config / drivers
- HAL bindings device-specific
- Decizia OS-ului host (Ubuntu vs alta) — fixat pe Ubuntu 22.04 LTS (cea mai universal suportată pentru AOSP build)
- L1 orchestrator integration (fază separată)

---

## Y2.b — Flash + boot verification (BLOCKED on device acquisition)

> Execută George manual când ajunge device-ul. Folosește scaffolds + documentația din Y2.

- Selectare device-specific BoardConfig (Pixel / OnePlus / etc.)
- Kernel config adaptat
- Build complet AOSP YBOS image pentru device-ul achiziționat
- Flash + boot
- Verificare ybos-l0 daemon rulează, telemetria curge

---

## Y3+ — Faze enumerate (detaliu TBD când ajungem)

Doar headline-uri. Semne de întrebare doar unde **chiar afectează faza activă (Y2)**.

- **L1 orchestrator skeleton** — `ybos-orchestrator` crate, capability enforcement, agent registry. ❓ Hook pentru Agent Builder runtime registration: design-uim API-ul L1 în acea fază astfel încât session_token API din Y1 să se integreze curat.
- **LLM inference layer** — llama.cpp + mlc-llm pe NPU. Afectează Y2? **NU**.
- **Agent seed: Calendar** — primul agent end-to-end demo. **NU** afectează Y2.
- **Agent seed: News Digest**. **NU** afectează Y2.
- **Privacy firewall Layer 1 (capabilities)**. **NU** afectează Y2.
- **Privacy firewall Layer 2 (eBPF redactor)**. ❓ eBPF necesită kernel features specifice — Y2 doc-only flag pentru BoardConfig (CONFIG_BPF, CONFIG_BPF_SYSCALL, etc.) ca să nu uităm la Y2.b.
- **Privacy firewall Layer 3 (LLM judge)**. **NU** afectează Y2.
- **Agent seed: Trip Planner**. **NU** afectează Y2.
- **Agent seed: Market Intel**. **NU** afectează Y2.
- **Agent seed: Learning Curator**. **NU** afectează Y2.
- **Agent Builder Framework** — template + LLM-assisted configurator + UI flow. **NU** afectează Y2.
- **User-Context Memory subsystem** — storage + sync + capability `data.user_prefs`. **NU** afectează Y2.
- **Laptop Companion (Tauri)** — pairing QR/NFC + session crypto + task offload + cache sync. **NU** afectează Y2.
- **UI native YBOS mobile** — launcher, onboarding wizard UI, agent dashboards. **NU** afectează Y2.
- **Cross-device extins** (multi-phone, tabletă) — post-MVP. **NU** afectează Y2.
- **Cloud burst activation** — v0.2+. **NU** afectează Y2.
- **VM Mode (Tier 1) laptop** — Linux VM minim, GPU passthrough, SEV-SNP/TDX integration. Research item, post-MVP. **NU** afectează Y2.
- **Split inference layer-by-layer** ❓ research item (vezi ARCHITECTURE.md §4.5). Independent. **NU** afectează Y2.

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
