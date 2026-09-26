---
title: Incidents
---

# Incidents

Don't panic. Slips, wrong orders and dead batteries happen at every booth, and most of it can be undone.

## A customer placed the wrong order / I want to cancel an order

In the Vendor view under **Orders → Pending** (「订单 → 待处理」):

- **Don't tap Finish packing** (「完成配货」)
- Tap **Cancel** (「取消」) on that order

The order is discarded and the reserved stock is returned automatically.

## I accidentally tapped "Finish" and want to undo it

Don't worry:

1. In the Admin console, open this event's **On-site · Orders** (「现场 · 订单」)
2. Find the order you completed by mistake
3. Under **Actions** (「操作」), choose **Set as cancelled** (「设为已取消」)

The system reverses both the goods and the money for that order: stock is added back, the amount is taken off the Payment channel, and sales are corrected.

A cancelled order can't change status again, and **can't go back to Pending**. If the customer only returned some of the items, use **Refund** (「退货」) in the Vendor view instead of cancelling the whole order.

## The host device ran out of battery / froze

BoothKernel stores everything in SQLite and **writes to disk immediately**.  
After restarting the device and the app, all products, past orders and stock data **come back automatically** exactly as they were before the crash.

:::tip On-site advice
Bring a power bank or power strip for the host. Not because BoothKernel is unstable, but because venue power just isn't reliable.
:::

## A customer wants to return something after buying

In the Vendor view, find the order under **Orders → Completed** (「订单 → 已完成」) and tap **Refund** (「退货」). For each line, choose how many to return, whether the goods go back to On-site stock or are written off, and which Payment channel the refund comes from.
**Refunds can't be undone**, so double-check before submitting. See [Closing & settlement](/en/guide/closing).

## The event is already settled and I found a missing entry

After settlement the ledger is frozen and orders can't be edited. For differences you find after getting home, add a **Settlement adjustment** or an **Advance** in the Admin console under **Closing · Settlement** (「收摊 · 结算」).
If you recount payments against your bills, just submit the **Payment recount** again. These three can still be changed after settlement.
