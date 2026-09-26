---
title: On-site operation
---

# On-site operation

This page covers the things you'll run into at the booth: payment QR codes, completing orders, sound alerts, and using several devices together.

## How do I set up payment QR codes? Can I use more than one channel?

Open the Event workbench and tap **Edit** (「编辑」) in the page header. You can upload **separate WeChat and Alipay** payment codes.

- Upload both: the customer's checkout page shows the WeChat / Alipay codes **side by side**, and customers scan whichever app they use
- Upload only one: the page shows **a single code, centered**
- No need to stitch images together yourself; two-code upload has been supported since v1.1

## Does the system complete the order automatically after the customer pays?

**No.**  
To protect privacy and avoid financial risk, BoothKernel does not connect to any payment API.

When you hear the "payment received" notification from Alipay/WeChat, open that order in the Vendor view, tap **Finish packing** (「完成配货」), pick the Payment channel (WeChat / Alipay / Cash / Other), then tap **Confirm payment** (「确认收款」).

Stock is already reserved when the customer places the order. **Confirm payment** records **the money side**: which channel, and the Amount received. It is bookkeeping only; the system doesn't (and can't) check whether the money actually arrived.

## Is there a sound when a new order comes in?

**Yes.** The Vendor view on your phone plays a sound when a new order arrives.

If you don't hear it, check first:

- Is the phone on **silent / Do Not Disturb**?
- Is the **media volume** too low?
- **Have you tapped the page at least once** since opening the Vendor view? Browsers only allow sound after the page has been tapped; tapping **Refresh** (「刷新」) is enough

Alerts keep working while you're on the **Stock** (「库存」) or **Closing** (「收摊」) tab.

## Can several people / devices run the booth at once?

Yes! As long as they're on the same hotspot, you can:

- put 2 tablets out as "ordering stations"
- have 3 booth staff each use their phone as a "packing station"

All data syncs in real time.

## Can the vendor place orders for customers?

Yes. In the Vendor view header, tap **Go to ordering** (「去点单」) to open the Customer order page, add items and place the order for the customer; when done, tap **Back to vendor view** (「回摊主端」) in the header to take the payment.
If your products have barcodes, the **▦ Scan** (「▦ 扫码」) button on the order page or a barcode scanner adds items faster; see [Barcode scanning](/en/guide/barcode-scan).

## How do I give discounts / round down?

For discounts you set up ahead of time, use [Bundles](/en/guide/lots); the customer's cart applies them automatically.
For an on-the-spot price cut, just change **Amount received** (「实收」) in the **Confirm payment** dialog; the difference is recorded as a Manual discount.

## How do I record goods given away or damaged?

Vendor view → **Stock** (「库存」) tab → **Record gift/scrap** (「登记赠送/报废」). If you don't record them, they'll show up as discrepancies in the closing stocktake. See [Closing & settlement](/en/guide/closing).
