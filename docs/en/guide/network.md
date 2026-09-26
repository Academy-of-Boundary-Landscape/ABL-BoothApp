---
title: Networking & Connection
description: How one hotspot lets the host, order tablet and vendor phones work together over a LAN, and how to troubleshoot when they can't connect.
---

# Networking & Connection

## Why a LAN

Convention networks are notoriously unreliable: tens of thousands of people in one hall, congested 4G / 5G, venue Wi-Fi that keeps dropping.
Cloud-based checkout mini-apps tend to freeze exactly when you're busiest.

BoothKernel does the opposite: **all data and services live on one of your own devices (the host)**, and the other devices connect to it directly over the local network.
A LAN doesn't need internet; a phone hotspot is enough, uses no data, and doesn't care if the venue network goes down.
More on the reasoning in [Offline & payments (Chinese)](/guide/why-offline).

## One host, many terminals

| Device | What it does | How to open |
| --- | --- | --- |
| **Host** | Runs the BoothKernel app, stores all data | Just open the app |
| Order tablet | Customer self-ordering, shows payment QR codes | Scan the **Customer entry** (「顾客入口」) QR code |
| Vendor phone | Order alerts, confirming payment, closing | Scan the **Vendor entry** (「摊主入口」) QR code |
| Other computers / tablets | Remote access to the admin console | Scan the **Admin entry** (「管理员入口」) QR code |

The host can be a Windows computer or an Android tablet. Terminals only need a browser, no app install.

:::warning Try not to use a phone as the host
Screen lock, switching to a payment app, or an incoming call can all make the system pause BoothKernel's background service, and customer orders will hang.
If you do use a phone as the host, keep the screen always on, disable auto-lock, and allow BoothKernel to run in the background in the battery settings.
:::

## Setup steps

1. Turn on a **personal hotspot** on a phone (mobile data not needed)
2. Connect the host, order tablet and vendor phones **all to that hotspot**
3. On the host, open BoothKernel, go to **Settings → LAN connection** (「**设置 → 局域网连接**」) and tap **Get LAN QR codes** (「**获取局域网二维码**」)
4. On the other devices, scan the matching QR code (Customer / Vendor / Admin entry) with the **system camera**, or tap **Click to copy link** (「点击复制链接」) under a QR code and send it over
5. The first time, the browser says "Your connection is not private". Tap **Advanced → Proceed**. Each device only needs this once; why: [LAN HTTPS (Chinese)](/guide/lan-https)

Customers don't scan anything themselves: they just use the tablet you put on the booth.

:::tip Rehearse the night before
At home, connect the hotspot, host, tablet and phones once, and accept the certificate on every device.
At the venue, the host's IP changes on a new network so you'll need to get the QR codes again, but you'll already know the drill.
:::

## When it won't connect

Check in this order:

1. **Same hotspot?** The most common cause is a device automatically rejoining the venue Wi-Fi or your home Wi-Fi.
2. **Windows firewall**: on first launch the host asks; tick **Private networks** and allow. If you missed it, go to **Windows Defender Firewall → Allow an app through firewall** and tick BoothKernel. The port that needs to be open is **5141**; 5140 is only used locally, ignore it.
3. **IP changed**: restarting the hotspot or switching networks invalidates old QR codes. Go back to the host and **Get LAN QR codes** again.
4. **Venue / campus Wi-Fi**: these networks usually enable "AP isolation", so devices can't reach each other. Use a phone hotspot instead.
5. **Scanned with WeChat / Alipay?** Their built-in browsers may block self-signed certificates. Scan with the system camera.

More in [FAQ · Network](/en/faq/network).
