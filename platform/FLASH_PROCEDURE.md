# YBOS Flash Procedure (Y2.b Prep)

This document describes the steps required to flash YBOS onto a physical ARM64 device.
**Note**: These steps are generic for ARM64 and will be finalized in Y2.b after a specific device is acquired.

## Prerequisites

- Physical ARM64 device with unlockable bootloader.
- Linux build host with `fastboot` and `adb` installed.
- YBOS build artifacts (images produced by AOSP build).

## 1. Unlock Bootloader

> **WARNING**: Unlocking the bootloader wipes all user data.

### Generic ARM64
1. Enable **Developer Options** (tap Build Number 7 times).
2. Enable **OEM Unlocking** and **USB Debugging**.
3. Reboot to bootloader: `adb reboot bootloader`.
4. Unlock: `fastboot flashing unlock`.
5. Confirm on device screen.

### OEM Specifics
- **Pixel**: `fastboot flashing unlock` [verificat când achiziționăm Pixel 7/8]
- **OnePlus**: `fastboot oem unlock` [verificat când achiziționăm OnePlus 11]
- **Fairphone**: Requires unlock code from manufacturer website [verificat când achiziționăm Fairphone 5]

## 2. Prepare for Flashing

Ensure you have the following images in your build output directory (`out/target/product/<device>/`):
- `boot.img`
- `dtbo.img`
- `system.img`
- `vendor.img`
- `vbmeta.img`

## 3. Flash YBOS

1. Reboot to bootloader: `adb reboot bootloader`.
2. Flash images:
   ```bash
   fastboot flash boot boot.img
   fastboot flash dtbo dtbo.img
   fastboot flash --disable-verity --disable-verification vbmeta vbmeta.img
   fastboot flash system system.img
   fastboot flash vendor vendor.img
   ```
3. Wipe data (first time only): `fastboot -w`.
4. Reboot: `fastboot reboot`.

## 4. Smoke Test

Once the device boots:

1. Verify system properties:
   ```bash
   adb shell getprop ro.product.brand  # Should return 'YBOS'
   adb shell getprop ro.product.model  # Should return 'YBOS-DEV'
   ```

2. Verify `ybos-l0` daemon:
   ```bash
   adb shell ps -A | grep ybos-l0
   ```

3. Check logs:
   ```bash
   adb logcat | grep ybos-l0
   ```

Code implemented with help from AI Agents Claude, Codex, Jules.
