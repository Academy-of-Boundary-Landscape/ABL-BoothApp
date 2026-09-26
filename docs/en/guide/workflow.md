---
title: Workflow
description: From pre-event prep to post-event review — how to use BoothKernel efficiently
---

# Recommended Workflow

BoothKernel isn't just a bookkeeping app; behind it is a standard workflow that we recommend.

:::warning ⚠️ Core principle: money safety
**BoothKernel is a tool for ordering and stock management. It does not handle money directly.**
Our payment logic is "show the booth owner's personal WeChat Pay / Alipay QR code". We can't and won't touch your money flow.
This means an "order completed" in the app only means **a record was made**, not that the money actually reached your account.
**"I owe XX" on the settlement sheet is the amount the ledger says you should transfer, not something already transferred; "actual received" is what you counted and entered yourself. The system has no way to verify any payment.**
*   Only confirm in the app **after your phone shows** the WeChat / Alipay / cash payment notification.
*   We recommend turning on WeChat / Alipay "voice announcement" to help confirm.
:::

## Pre-event

Before heading to the convention, do this setup comfortably at home, not on-site.

### 1. Enter products and stock
*   **Product library**: enter every item's code, name, price and image in the **Product library** (「商品库」). For goods you sell on consignment for another circle, set **Circle** (「所属社团」) to that circle (create it first on the **Circles** page, 「社团」), and settlement splits the accounts automatically.
*   **Listing and stock**: create an event, open the Event workbench, and in **Pre-event · Products** (「展前 · 商品」) use **Pick from Product library** (「从商品库选」) or **Import from previous event** (「从上一场导入」). Fill in the stock you're bringing (e.g. new book x100, acrylic stand x50). If something runs low on-site, you can **Restock** (「补货」) any time.
*   **Payment QR codes**:
    *   Upload your **WeChat Pay / Alipay QR code images** when creating / editing the event.
    *   After a customer orders, the tablet shows the QR code directly, so you don't have to point at a sticker on the table.

:::tip About entering items
On the **Product library** page you can export item data (code, name, price, image) to a `.boothpack` file.
If a partner is running the booth for you, send them this file to import directly, instead of entering everything again.
:::


### 2. How to handle "sets"
:::tip 💡 Enter sets according to whether you pre-pack them
If you sell sets that contain several items (e.g. new-book set = 1 book + 1 charm), pick one of the two methods below according to **how you pack**, so stock stays accurate:
:::

*   **Method A: pre-packed (recommended)**
    *   *Scenario*: before the event you've already sealed the book and charm into a bag.
    *   *How*: create a product in the app called **"New-book set"**, with stock equal to the number actually packed (e.g. 50).
    *   *Logic*: sell it as an independent product; each sale deducts one.
*   **Method B: assembled on the spot + bundle deal**
    *   *Scenario*: you didn't pre-pack; when a customer buys the set you grab a book and a charm on the spot.
    *   *How*: enter "New book" and "Charm" as separate products, then in the Event workbench's **Pre-event · Bundles** (「展前 · 套装」) set up a **Fixed combo** (「固定组合」) bundle: candidates are these two, pick 2 items, fill in the bundle price. See [Bundles & Deals](./lots).
    *   *Logic*: the customer taps "New book" and "Charm" separately on the order page, and the cart automatically applies the bundle and shows how much they saved. Stock for each item is still deducted precisely.
    *   *Note*: **all candidates in a bundle must belong to the same Owner.** Consignment goods from another circle can't go into the same bundle as your own books — that would mean giving a discount on their behalf.

### 3. Dry run
*   Get together all the devices you'll bring (phones, tablets).
*   **Offline test**: turn off the router, have a phone open a hotspot, make sure the tablet connects and can take orders.
*   **Battery anxiety**: fully charge every device and bring **power banks** (this matters a lot!).
*   **Rehearse the flow**: have a friend pretend to be a customer and walk through ordering, paying, recording the payment method and handing over goods, so you're comfortable with every step.

---

## On-site

What matters most on-site is **speed** and **accuracy**.

### 1. Opening quickly
1.  **Hotspot**: turn on a Wi-Fi hotspot on one phone; connect the host, tablet and vendor phone to it.
2.  **Start the host**: open the BoothKernel app on the host, go to **Settings → LAN connection** (「设置 → 局域网连接」) and tap **Get LAN QR codes** (「获取局域网二维码」).
3.  **Connect devices**: the tablet scans **Customer entry** (「顾客入口」), the vendor phone scans **Vendor entry** (「摊主入口」), and both open this event.
4.  **Place devices**: stand the tablet in the most visible spot on the booth as an "electronic menu".

:::tip
### Tip: getting customers to use self-ordering
Customers sometimes just ask you directly out of habit and ignore the tablet. A few tricks make self-ordering smoother:

1.  **Physical cues**:
    *   **Add a label**: stick an eye-catching note on the tablet frame or stand, e.g. **"👈 Tap to browse / order here"**.
    *   **Placement**: put the tablet where customers' eyes naturally land (usually front-right of the booth), not hidden behind the goods.

2.  **What to say**:
    *   When a customer comes close: "Hi, **feel free to swipe through the tablet**, it has big pictures and prices."
    *   When asked "how much is this": "If you tap it on the tablet it adds up the total for you, give it a try."

3.  **Idle display**:
    *   When nobody is ordering, keep the tablet at the **top of the product list** or on your hottest new book cover. Don't leave it on the checkout page or a black screen.
:::

### 2. Standard order flow

Make it muscle memory — follow this 4-step loop:

#### Step 1: Customer orders on the tablet
*   Point customers to browse on the tablet.
*   The customer picks items and taps **Checkout → Place order** (「去结算」→「确认下单」). If a bundle deal applies, the cart applies it automatically and shows the savings.
*   The tablet shows **"Please scan to pay ¥85"** (「请扫码支付 ¥85」) with your payment QR code and the order number.

#### Step 2: Payment and packing (in parallel)
This is where the time savings come from!
*   **You (the vendor)**: see the "new order" alert on your phone, turn around and grab the items from the box right away.
*   **Customer**: at the same time, takes out their phone and scans the code on the tablet to pay.

#### Step 3: Confirm payment
*   **Listen / look**: wait for your phone's "Alipay received" sound, or glance at the customer's payment success screen.
*   **Tap**: on this order in the Vendor view, tap **Finish packing** (「完成配货」). The **Confirm payment** (「确认收款」) dialog opens:
    *   Choose the **Payment channel** (「收款渠道」: WeChat / Alipay / Cash / Other); at closing, accounts are reconciled per channel;
    *   If the customer doesn't want a bundle, you can split it here; for rounding down or a price cut, just edit **Amount received** (「**实收**」);
    *   Tap **Confirm payment ¥xx** (「确认收款 ¥xx」).
*   **Note**: this step only **records the entry**; it does not mean the app verified that the money arrived.

#### Step 4: Hand over and finish
*   Hand the packed items to the customer.
*   Stock was already deducted when the customer placed the order, so this order is done. On to the next customer.

:::tip Ordering for customers / scan to add
If a customer doesn't want to order themselves, tap **Take order** (「**去点单**」) in the Vendor view to open the order page and order for them, then tap **Back to vendor view** (「**回摊主端**」) when done.
If products have barcodes, **▦ Scan** (「▦ 扫码」) on the order page lets you scan continuously to add items, or you can just use a barcode scanner. See [Barcode scanning](./barcode-scan).
:::


### 3. Checking what's left

The **Stock** tab (「**库存**」) in the Vendor view shows the on-site stock of each item, calculated from all orders, refunds, gifts and scrap so far.

For goods that are no longer on the booth but weren't sold (given away, damaged), record them via **Record gift/scrap** (「**登记赠送/报废**」) in the Stock tab, or the closing stocktake won't match.

:::tip
A good habit: before closing, count the actual stock by hand and compare it with the app.
If there's a difference, find out why (missed orders, mistakes, etc.) so you can improve your process next time.
:::

### 4. Refunds

When a customer comes back to return something, find the order in **Vendor view → Completed** (「已完成」) and tap **Refund** (「退货」):

*   The dialog lists the order **line by line** (the same product may be split into several lines because of bundle membership). You decide which line and how many to refund; the system doesn't guess.
*   For each line, choose where the returned goods go: **Back to on-site stock** (「回现场仓」, can be sold again) or **Write off as loss** (「进损耗」, already damaged).
*   **The refund channel defaults to the payment channel.** If they paid cash, refund in cash, or the payment recount at closing won't match. You can pick a different channel, but then both channels' accounts change separately.
*   The refund amount defaults to the total actually paid for the selected lines. If you refund less than was paid, the difference stays on the Owner's account (you're refunding the part the customer didn't take); **you can't refund more than the customer paid** — to give money away, use a settlement adjustment.

Fully refunded lines are marked **Fully refunded** (「已退完」) to prevent double refunds. **A submitted refund can't be undone**; if you refunded by mistake, fix it with a reverse order or a settlement adjustment.

How gifts, scrap and refunds affect settlement: [Closing & Settlement](./closing).

---

## Closing

When the event is over and you're packing up, switch to the **Closing** tab (「收摊」) in the **Vendor view** and follow the four-step wizard:

1.  **Check orders**: complete or cancel every unconfirmed order one by one first, or you can't continue.
2.  **Stocktake** (can be skipped): count how many of each item are actually left and fill it in; mismatches with the books are recorded as stocktake discrepancies. For events without a stocktake, the settlement sheet notes "No stocktake; remaining quantities are calculated from the books".
3.  **Take back**: confirm you're taking all remaining goods with you.
4.  **Settle**: tap **End event** (「结束展会」). The ledger is frozen, and the Closing tab turns into this event's settlement sheet in place.

After freezing, only three things can still be filled in later at home: **Advances, Settlement adjustments, Payment recount**. Details of each step and how to read the settlement sheet: [Closing & Settlement](./closing).

:::warning Note: deleting an event is not protected by freezing
Deleting a settled event from the event list **permanently deletes** it together with all its orders and ledger entries, **with no way to recover**. Don't delete events you want to keep for reconciliation.
:::

---

## Post-event review

After closing, your data is your most valuable asset.

### 1. Quick reconciliation
On the way out or back at the hotel:
*   Open the admin console and go to this event's **Closing · Settlement** (「收摊 · 结算」).
*   In **Payment recount** (「收摊清点」), fill in the actual amount for each payment channel, using WeChat / Alipay's "today's bill" and the cash in hand.
*   The system calculates each channel's **book amount** and **difference**. A few yuan off is common; for large gaps, export the settlement sheet and go through the **Ledger entries** and **Order details** sheets line by line.

### 2. Export reports
The two exports answer two different questions; pick what you need:
*   **Sales summary** (Event workbench → On-site · Stats → **Download Excel report**, 「现场 · 统计 → 下载 Excel 报告」): answers "**what did we sell**" — sales quantity and revenue per product. Good for posting results in your group or post-event review.
*   **Settlement sheet** (Event → Closing · Settlement → **Export Excel**, 「收摊 · 结算 → 导出 Excel」; also exportable from the Closing tab in the Vendor view): answers "**how do the accounts work out**" — how much each Owner gets, how much you owe whom, and how each payment channel's book and actual amounts differ. **Consignment payouts use this one, not the sales summary.**
*   Both write amounts as "yuan, two decimal places", so they can be imported straight into your circle's finance system.

### 3. Data backup
*   Event data only exists on the host. After each event, export a settlement sheet and sales summary to keep, in case the device is lost.
*   Back up the Product library by exporting a `.boothpack` at the bottom of the **Product library** page.

---

## Extra tips

*   **iPad theft**: leaving the tablet out is convenient, but with heavy crowds use an **anti-theft stand/lock**, or bring the tablet in from time to time.
*   **Cash**: even in the age of mobile payments, bring about ¥200 in small change (¥10 / ¥5) just in case.
*   **Mindset**: technology is a tool; you're in charge. If things get really chaotic and the app won't connect, **switch to pen and paper right away**. Don't spend selling time fixing bugs.
