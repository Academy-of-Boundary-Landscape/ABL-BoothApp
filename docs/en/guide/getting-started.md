---
title: 5-Minute Quick Start
description: Set up ordering and bookkeeping at a convention with no internet, using one host device plus a tablet or phone.
---

# 5-Minute Quick Start

BoothKernel's core idea: **don't depend on the venue's network**. One hotspot is enough for several devices to work together over a local network.
Below we follow the real on-site flow and build a working booth setup from scratch.

:::tip Upgrading from v1.1?
v1.2 uses a new ledger. Old events and orders are not migrated (you can export them to Excel). Please read [Upgrading from v1.1 to v1.2](./upgrade-v1.2) first.
:::

:::tip BoothKernel has three roles
- **Admin console**: used before and after the event. Enter products, create events, set up bundles, review orders and stats, read the settlement sheet
- **Customer order page**: placed in front of the booth. Shows products, lets customers order themselves, displays your payment QR codes
- **Vendor view**: in the booth owner's hand. New-order alerts, confirming payment, refunds, gifts/scrap, closing
:::

The app's interface is Chinese only. In this guide, UI labels are given in English with the original Chinese in 「」 so you can match them to the screen.

---

## What you need

### Minimal setup (recommended)
- Device A: a tablet with the **BoothKernel app** installed. Admin console before and after the event, Customer order page during the event
- Device B: a phone with a browser. Scan a QR code to connect and use it as the Vendor view to take orders

### High-traffic setup
- Host: a computer (under the table / behind the booth)
- Order pages: 1–2 tablets (in front of the booth as an electronic menu)
- Vendor view: 1–3 phones (the owner and circle members taking orders at the same time)

---

## Step 1: Install and basic setup

<div style="text-align: center;">
  <img src="/images/help/p1.png" alt="Step 1" style="max-width: 200px;">
</div>

1. Download and install BoothKernel from [GitHub Releases](https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases) (`.exe` for Windows, `.apk` for Android)
2. Open the app, tap **Enter the admin console to start setup** (「**进入管理后台开始配置**」) and enter the admin password (default `admin123`)
3. The left sidebar has: **Events / Product library / Circles / Settings / Tutorial** (「展会 / 商品库 / 社团 / 设置 / 使用教程」)

:::info Do this first
Go to **Settings → Security** (「**设置 → 安全设置**」) and change the admin password and the default vendor password. The default passwords are written in this public guide,
so anyone on the same Wi-Fi could log in with them. Until you change them, the admin console keeps showing a reminder at the top.
:::

---

## Step 2: Fill the Product library

Open **Product library** (「商品库」) from the sidebar and add products one by one under **Add a new product to the library** (「添加新商品到仓库」):

- **Product code** (「商品编号」, required, unique; short codes like A01, B02 work best), **Product name** (「商品名称」), **Default price** (「默认价格」)
- Optional: **Category** (「商品分类」, the order page groups products by it on the left), **Tags** (「标签」, filter by character or series), **Preview image** (「商品预览图」)
- Optional: **Commercial barcode** (「商业条码」). Commercial books and official merch with a JAN / ISBN can have it filled in, so you can [scan to add to cart](./barcode-scan) when ordering
- Optional: **Circle** (「所属社团」). Leave it empty and it belongs to "Your circle". For goods you sell on consignment for another circle, first create that circle on the **Circles** page, then pick it here. Settlement will split the accounts automatically

:::tip The Product library is a reusable template
Spend one day entering all of your circle's items, and after that each event only needs you to pick what goes on sale.
Once done, you can export a `.boothpack` at the bottom of the Product library and send it to friends helping at your booth.
:::

---

## Step 3: Create an event and list products

<div style="text-align: center;">
  <img src="/images/help/p2.png" alt="Step 3" style="max-width: 200px;">
</div>

1. Sidebar **Events** → **New event** (「**新建展会**」): fill in name, date, location. You can set a **vendor password** for this event (leave empty to use the global default vendor password)
2. Upload your **WeChat Pay QR code** and **Alipay QR code** (optional, one is fine). After a customer orders, the order page shows them
3. Tap the event card to open the **Event workbench**. At the top is a status bar **Preparing → In progress → Settled** (「筹备 → 进行中 → 已结算」); below it the tabs are grouped into **Pre-event / On-site / Closing** (「展前 / 现场 / 收摊」)
4. In **Pre-event · Products** (「展前 · 商品」), list what you're selling this time:
   - **Pick from Product library** (「**从商品库选**」): select several products, then fill in the sale price (defaults to the library price) and stock (empty means 0; you can **Restock** later) in the **To be listed** (「待上架」) table
   - **Import from previous event** (「**从上一场导入**」): if this isn't your first event, tick products and bundles from the last one and bring them over in one go
5. For bundle deals (pick any N, combo price), go to **Pre-event · Bundles** (「展前 · 套装」). See [Bundles & Deals](./lots)
6. When ready, tap **Start event** (「**开始展会**」) on the status bar. The event becomes **In progress**, and only then can the order page and Vendor view find it

<p style="text-align: center;"><img src="/images/v1.2/admin-workbench-products.png" alt="Event workbench: Pre-event · Products" style="width: 100%; border-radius: 8px; border: 1px solid var(--vp-c-divider);"></p>


---

## Step 4: Networking (offline LAN)

<div style="text-align: center;">
  <img src="/images/help/p3.png" alt="Step 4" style="max-width: 200px;">
</div>

Devices sync over the local network. The most reliable way is a **personal hotspot**:

1. Turn on the hotspot on any phone (mobile data not needed)
2. **Connect every device to that hotspot**
3. Keep BoothKernel open on the host (the device with the app installed)
4. On the host, go to **Settings → LAN connection** (「**设置 → 局域网连接**」) and tap **Get LAN QR codes** (「**获取局域网二维码**」). Three QR codes appear:
   **Customer entry**, **Vendor entry**, **Admin entry** (「顾客入口 / 摊主入口 / 管理员入口」)
5. On the other devices, scan the matching code with the **system camera** (not WeChat / Alipay, which may block it)
6. The first time, you'll see a "Your connection is not private" warning. Tap **Advanced → Proceed**. Each device only needs this once (why: [LAN HTTPS (Chinese)](/guide/lan-https))

:::warning Don't connect devices to venue Wi-Fi or campus networks
Venue Wi-Fi is congested and drops often; big networks like campus Wi-Fi usually enable client isolation, so devices can't reach each other at all.
On first launch, the Windows host asks about the firewall. Allow **Private networks**.
:::

---

## Step 5: Set up the devices

<div style="text-align: center;">
  <img src="/images/help/p4.png" alt="Step 5" style="max-width: 200px;">
</div>

The browser version works almost the same as the app, so it doesn't matter much which device is the host. Assign them however suits your booth.

### Order tablet
Purpose: sits in front of the booth, shows products, lets customers order themselves, shows your payment QR codes.

1. Open it with the **Customer entry** QR code (or tap **Customer view** (「顾客端」) at the bottom of the sidebar in the host app)
2. Tap the event in progress to open the order page
3. After 60 seconds with no interaction it returns to the "Welcome" (「欢迎光临」) attract screen and clears the cart

<p style="text-align: center;"><img src="/images/customer.png" alt="Order tablet: bundles applied automatically in the cart" style="width: 100%; border-radius: 8px; border: 1px solid var(--vp-c-divider);"></p>


Customers can also find products with **📷 AI photo recognition** (「📷 拍照识别」, set up [AI photo recognition (Chinese)](/guide/vision-search) first) or **▦ Scan** (「▦ 扫码」, see [Barcode scanning](./barcode-scan)).

### Vendor phone
Purpose: new-order alerts, confirming payment, packing orders.

1. Open it with the **Vendor entry** QR code and choose the event
2. Enter the vendor password (this event's own password if you set one, otherwise the default vendor password `vendor123`)
3. There are three tabs at the bottom: **Orders / Stock / Closing** (「订单 / 库存 / 收摊」)

<p style="text-align: center;"><img src="/images/v1.2/vendor-orders-phone.png" alt="Vendor view: Orders" style="max-width: 300px; width: 100%; border-radius: 8px; border: 1px solid var(--vp-c-divider);"></p>


The phone plays a sound when a new order comes in. Turn up the media volume and tap anywhere on the page once (browsers only play sound after the page has been touched).

:::tip Typical on-site flow
1. Customer picks items on the tablet → **Checkout** (「去结算」) → **Place order** (「确认下单」) → the tablet shows your payment QR code
2. Your phone rings → pack the items → once you hear the payment notification, tap **Finish packing** (「**完成配货**」) → choose the payment channel and tap **Confirm payment** (「**确认收款**」)
3. To order on a customer's behalf, tap **Take order** (「**去点单**」) in the Vendor view, and **Back to vendor view** (「**回摊主端**」) when done
:::

---

## Step 6: Closing and settlement

<div style="text-align: center;">
  <img src="/images/help/p5.png" alt="Step 6" style="max-width: 200px;">
</div>

When the event ends, switch to the **Closing** tab (「**收摊**」) in the **Vendor view** and follow the four-step wizard: **Check orders → Stocktake → Take back → Settle**.
After that the event is frozen, and the Closing tab turns into this event's **Settlement sheet**. Details of each step: [Closing & Settlement](./closing).

Then, in the admin console's Event workbench:

- **On-site · Stats** (「现场 · 统计」): sales curve and per-product sales. **Download Excel report** (「下载 Excel 报告」) gives you the **sales summary**, which answers "what did we sell"
- **Closing · Settlement** (「收摊 · 结算」): the settlement sheet. **Export Excel** (「导出 Excel」) tells you how much each circle gets and how much you owe whom, which answers "how do the accounts work out"

**For consignment payouts, use the settlement sheet, not the sales summary.** The difference is explained in [Export & Review](./export).

---

## FAQ: common on-site questions

### Q1: Do I need internet?
No. BoothKernel is designed to **work on an offline LAN**; the hotspot is the network. Only **Check for updates** and the first download of the AI recognition model need internet.

### Q2: How many devices can connect at once?
Mostly depends on the hotspot device and on-site interference. 1 host + 2 tablets + 3 phones is no problem.

### Q3: What happens if a device disconnects?
While disconnected, it won't get the latest orders and stock. It reconnects automatically once the connection is back; just refresh the page. All data lives on the host, so nothing is lost.

### Q4: How do I back up and export data?
- Product library: export a `.boothpack` at the bottom of the **Product library** page
- Event data: export the sales summary and settlement sheet (Excel). Note that Excel files can't be imported back into the app
