---
title: Hardware
---

# Hardware

In short: BoothKernel doesn't need much performance, but it needs to **stay online**, so choosing the host matters.

## Can I run the host on a phone?

Technically yes, but it's **strongly discouraged**.

The reasons are practical:

- Locking the screen or backgrounding the app can easily kill it  
- Heat causes throttling or even disconnections  
- You'll also be using it to take payments and chat, so it's easy to tap the wrong thing

**Use a laptop or tablet as the host.**

With a phone as host, locking the screen or switching to the WeChat / Alipay payment page may pause BoothKernel's background service, and customer orders will get stuck.
If you must, keep the screen always on, don't let it lock, and allow BoothKernel to run in the background in the battery settings.

## What are the hardware requirements?

Very low:

- Any Windows computer that runs the latest Chrome/Edge
- Or an Android 8.0+ tablet/phone

Either runs smoothly. An old device makes a great "customer ordering station", too.

## Does the screen need to stay on?

- Keep the host's screen on, or set it to **never sleep** in the power settings
- Set customer-facing tablets to **stay awake** so the screen doesn't go dark while someone is ordering

## Do I need a barcode scanner?

No. The Customer order page can scan barcodes with the camera. If you have a USB / Bluetooth barcode scanner (keyboard mode), just plug it in; it's faster.
On Windows, switch to an English input method before using a scanner. See [Barcode scanning](/en/guide/barcode-scan).

## The app crashes right after opening on Windows?

Since v1.2 the installer bundles the VC++ runtime, and if the AI recognition runtime fails to load, only AI photo recognition is disabled; the app still opens.
If it still crashes, go to **Settings → About & updates → Troubleshooting** (「设置 → 关于与更新 → 故障排查」), copy the log path, and send `booth.log` to the user group or a GitHub Issue.
