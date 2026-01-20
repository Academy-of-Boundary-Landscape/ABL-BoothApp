---
title: Unexpected Situations
---

# Unexpected Situations

Don't panic. At the stall, things like misclicks, wrong orders, or dead batteries happen—most are "reversible".

## Customer placed a wrong order / How do I cancel an order?

On the vendor's order fulfillment page:

- **Do NOT click "Complete"**
- Simply click the red **"Cancel"** button

The order will be voided, and the locked inventory will be automatically returned.

## I accidentally clicked "Complete" and want to undo it. What should I do?

Don't panic:

1. Go to the host device's "Order Management" page
2. Find the order that was mistakenly completed
3. Change its status to **"Cancelled"**

The system will:

- Return the inventory
- Correct the sales statistics

## What if the host device suddenly runs out of battery / crashes?

TanHe uses SQLite with **real-time disk persistence**.  
After restarting the device and software, all product information, historical orders, and inventory data will **automatically restore** to the moment before the crash.

:::tip On-site Tip
It's best to have a power bank or extension cord for the host device. Not because TanHe is unstable, but because on-site power can be unreliable.
:::