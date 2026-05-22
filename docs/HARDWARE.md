# YBOS Target Hardware

> Sesiunea decision: 2026-05-21 (clarificare sesiunea 2: device-ul exact NU e batut în cuie)
> Recomandare Lead Dev: **Pixel 7 second-hand (~450€)** ca exemplu solid — dar **alegerea finală e flexibilă**, depinde de disponibilitate și preț la momentul achiziției.

---

## Regula generală

**Codul YBOS NU presupune un model anume.** Device-specific code stă izolat în `l0/src/hw/` și `platform/`. Orice device ARM64 cu NPU dedicat + bootloader unlockable + AOSP/LineageOS/GrapheneOS community support poate deveni target test. Modelul exact e o alegere de **moment de cumpărare**, nu o constrângere arhitecturală.

---

## Minimum specs (brand-agnostic)

### Mandatorii
- ✅ **Bootloader unlockable** — verifică pe XDA / GrapheneOS forum înainte cumpărare
- ✅ **AOSP / LineageOS / GrapheneOS community support** activ
- ✅ **ARM64 octa-core**, modern (Cortex-A78+, Snapdragon 8 Gen 1+, Tensor G2+, Dimensity 9000+)
- ✅ **RAM minimum 8GB**, 12GB+ recomandat pentru dev și model 8B
- ✅ **NPU / AI accelerator dedicat**:
  - Google EdgeTPU (în Tensor G2/G3/G4)
  - Qualcomm Hexagon NPU (Snapdragon 8 Gen 1+)
  - MediaTek APU (Dimensity 9000+)
- ✅ **Stocare 128GB+ UFS 3.1+** (pentru model files + Android Runtime + YBOS)
- ✅ **USB-C** cu fastboot/ADB access
- ✅ **Wi-Fi 6**, **Bluetooth 5.1+**
- ✅ **Senzor amprentă** (under-display sau rear) pentru biometric onboarding test
- ✅ **NFC** — necesar pentru cross-device pairing (Y14 "simbioza")

### Nice-to-have
- 5G modem (4G mandatoriu)
- OLED display (battery savings during dev sessions)
- Stereo speakers (voice IO testing)
- Wireless charging
- IP67/68 (development resilience)

---

## Concrete options (sorted by recommendation, NOT fixed choice)

> Orice device de mai jos e acceptabil. Alegerea finală depinde de disponibilitate stoc, preț și ofertă second-hand. **Nu refacem arhitectura dacă ajunge un OnePlus în loc de Pixel.**

### Pixel 7 — recomandare top (exemplu)
- **Preț**: ~400-500€ second-hand (verificat eBay, Vinted, Backmarket)
- **Avantaj**:
  - Best AOSP support out-there
  - Tensor G2 cu EdgeTPU dedicat
  - GrapheneOS oficial suportat
  - Comunitate XDA enormă
  - 8GB RAM
- **Dezavantaj**:
  - Modem cam slab vs concurența (Samsung, Snapdragon)
  - Doar 5 ani updates Google (vs 7 la Pixel 8)

### Pixel 8 — dacă bugetul permite
- **Preț**: ~600-750€ second-hand / refurbished
- **Avantaj**: Tensor G3 mai capabil pe NPU, 7 ani updates Google, 8GB RAM (12GB pentru Pro)
- **Dezavantaj**: cost

### OnePlus 11 — alternativă Snapdragon
- **Preț**: ~450-550€ second-hand
- **Avantaj**: Snapdragon 8 Gen 2 (Hexagon NPU foarte bun), 16GB RAM variant disponibil, bootloader unlock easy
- **Dezavantaj**: mai puțin polish ROM-uri custom, comunitate sub Pixel

### Fairphone 5 — opțiune etică
- **Preț**: ~500€ nou
- **Avantaj**: modular, repairable, AOSP support oficial, etică production
- **Dezavantaj**: NPU mai slab (Snapdragon QCM6490 e mid-range)

### Sony Xperia 1 V — premium AOSP-friendly
- **Preț**: ~700€ second-hand
- **Avantaj**: hardware premium, Sony tradition AOSP-friendly, camera pro
- **Dezavantaj**: comunitate mai mică

### Nothing Phone 2 / Asus Zenfone — alte alternative valide
- Verifică bootloader unlock status + AOSP community înainte de cumpărare.

---

## AVOID — incompatibile sau riscante

### ❌ Samsung Galaxy (orice model)
- Knox bootloader unlock BRICK-uiește camera, biometric, payment chip
- Tradeoff inacceptabil pentru dev YBOS

### ❌ Huawei / Honor
- Fără Google services (problema pentru Google Calendar agent)
- Bootloader locked din 2018+

### ❌ Xiaomi / Redmi / POCO
- Politică bootloader inconsistentă (regional, "Mi Account wait period")
- Risc bricked permanent

### ❌ iPhone (orice generație)
- Closed boot chain
- Imposibil să flash custom OS

### ❌ Telefoane Linux dedicate (PinePhone, Librem 5)
- Hardware prea slab (NPU absent, CPU lent)
- LLM on-device n-ar funcționa bine

---

## Cumpărare checklist (când achiziționezi device)

Înainte de cumpărare verifică:

- [ ] Modelul exact (variantă regională — US, EU, JP au diferențe bootloader)
- [ ] **OEM unlock allowed** — caută pe XDA + GrapheneOS device list
- [ ] **AOSP/LineageOS build official** disponibil pentru modelul ales
- [ ] Bateria sănătoasă (dacă second-hand) — capacitate >85% original
- [ ] Toate butoanele funcționale (volume, power crucial pentru fastboot mode)
- [ ] USB-C port funcțional (nu doar pentru charging — pentru ADB)
- [ ] FCC ID corect (pentru variantă regională Europe)
- [ ] **Nu** SIM-locked la un carrier specific (afectează modem unlock)

Buget total pregătire dev:
- Phone: 400-700€ (Pixel 7 indicativ; range total flexibil)
- USB-C dock + cable bun (data transfer): 30€
- Cititor SD pentru backup (opțional): 10€
- Pad antistatic (când deschizi telefonul): 10€
- **Total**: ~450-750€

---

## Implicații cod (regulă consecventă cu YBOSClaude.md §8.7)

Nu hardcoda în cod generic (orchestrator, agenți, firewall, UI):
- ❌ `if device == "gs101"` sau `if soc == "tensor-g2"`
- ❌ `const NPU_PATH: &str = "/sys/devices/.../edgetpu0"`
- ❌ Paths specifice doar la Pixel kernel

În schimb:
- ✅ Trait `HwAccelerator` cu impls multiple (`TensorEdgeTpu`, `HexagonNpu`, `MediatekApu`, `CpuFallback`)
- ✅ Runtime detection prin `/sys/class/devfreq/` + capability probing
- ✅ Toate device-specific paths se rezolvă în `l0/src/hw/<vendor>.rs`

---

## Companion hardware (Y2+)

Pentru AOSP build environment:
- **PC build**: Ubuntu LTS, 32GB+ RAM, 500GB+ NVMe SSD, octa-core CPU
- **Sau**: cloud VM (Azure / AWS / Hetzner) cu echivalent
- **Storage** pentru AOSP source + builds: ~200GB minimum

Decizia local vs cloud build se ia după ce ajungem la Y2.

---

## Future: device YBOS-branded

Dacă proiectul prinde tracțiune (v1.0+), opțiuni:
- **Cea mai realistă**: partnership cu OEM existent (Fairphone, /e/OS Murena) pentru hardware co-branded
- **Mediu termen**: custom firmware pe device popular (Pixel 7 / 8 cu YBOS preinstalat)
- **Pe termen lung**: hardware custom proiectat — necesită capital VC

Nu e decizie pentru acum. MVP demo pe orice ARM64 + NPU + AOSP-friendly device disponibil pentru testare.
