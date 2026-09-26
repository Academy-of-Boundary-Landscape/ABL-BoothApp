---
title: Network connection
---

# Network connection

These are the most common questions, and also the easiest to fix: **99% of the time it's "not on the same hotspot" or "blocked by the firewall".**

:::tip Check these in order (it really works)
1. **Same Wi-Fi / hotspot**: the host (computer/tablet) and the phones must be on the same hotspot  
2. **Windows Firewall**: is BoothKernel (「摊盒/Booth-Kernel」) allowed through?  
3. **Hotspot restarted and the IP changed**: after restarting the hotspot, go to **Settings → LAN connection** (「设置 → 局域网连接」) on the host and **get the LAN QR code again**
:::

## How do other devices connect to the host? There's no "scan to connect" in the app?

- That's right. The architecture is:
    - **Host**: runs the BoothKernel app and handles the business logic and data storage
    - **Client**: any device on the LAN opens the web pages served by the host in a browser
- So **the host shows the code and other devices scan it**: on the host, go to **Settings → LAN connection** (「设置 → 局域网连接」) and tap **Get LAN QR code** (「获取局域网二维码」),
  then use the tablet's / phone's **built-in camera app** to scan the **Customer entry** (「顾客入口」) or **Vendor entry** (「摊主入口」) code.
- The **▦ Scan** (「▦ 扫码」) button on the Customer order page scans **product barcodes** to add items to the cart. It has nothing to do with connecting; see [Barcode scanning](/en/guide/barcode-scan).

## After scanning, the phone says "connection timed out" or the page won't open?

This is the most common problem. Check in order:

1. **Hotspot**: make sure the host (computer/tablet) and the phone that scanned are on the same Wi-Fi/hotspot  
2. **Firewall**: on a Windows host, check that the firewall allows BoothKernel through, or temporarily turn the firewall off to test  
3. **IP changed**: if you restarted the hotspot, the IP address may have changed. **Get LAN QR code** (「获取局域网二维码」) again on the host
4. **Certificate warning not clicked through**: the first visit shows "Your connection is not private". Tap "Advanced → Proceed"; see [LAN HTTPS (Chinese)](/guide/lan-https)

:::warning From experience
At conventions a lot of people instinctively join the venue Wi-Fi, but it's often flaky and may block devices from talking to each other. **A hotspot LAN is the most reliable option.**
:::

## Does it work with no internet / bad signal?

**Absolutely.** BoothKernel is designed offline-first.  
Turn on a **mobile hotspot** on one device to form a LAN and connect the other devices to it:

- **No internet connection needed**
- **No mobile data used**

## I'm on campus / public Wi-Fi and devices can't connect?

Public networks usually have **AP isolation** turned on (devices can't reach each other).  
The fix is simple: **use a hotspot to build the network**. It's the safest setup at a convention.

## If the network drops mid-event, do I lose data?

**No.** As long as the host (the app) stays open, the data is there.  
Once the network is back, refresh the page on the phone and it picks up where it left off.
