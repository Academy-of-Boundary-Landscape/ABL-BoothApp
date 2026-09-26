---
title: Data safety & migration
---

# Data safety & migration

This page covers four things: where your data lives, how to move to a new device, what's in the Excel exports, and where your old data went after upgrading from v1.1.

## Where is my data stored? Is it safe?

All data (images, ledger, stock) is stored in the **local database on your host device**.  
We have no cloud servers at all; nobody but you knows your sales figures.

## How do I move my data to a new device?

At the bottom of the **Product library** (「商品库」) page, in the **Product data pack (.boothpack)** (「商品数据包（.boothpack）」) panel, tap **Export .boothpack** (「导出 .boothpack」). This packs all product info, Circle ownership and images into one download.
Send the file to the new device and tap **Import .boothpack** (「导入 .boothpack」) in the same panel (or just drag the file in).

:::warning `.boothpack` only contains the Product library
Events, orders and the ledger are **not** included. Before switching devices, export the Settlement sheet and sales summary for any event you want to keep a record of.
:::

## What's in the exported Excel files?

There are two, and they answer different questions:

- **Sales summary** (**On-site · Stats** (「现场 · 统计」) → Download Excel report): how many of each product sold and for how much; good as a report card;
- **Settlement sheet** (**Closing · Settlement** (「收摊 · 结算」) → Export Excel): how much each Circle gets, how much I owe to whom, and each Payment channel's book vs. actual amounts. Use this one for splitting Consignment money.

See [Export & review](/en/guide/export).

## After upgrading to v1.2, where did my old events and orders go?

v1.2 switched to a new ledger. Old events and orders were not migrated, but they were **not deleted** either: they're kept as-is in `sale_system.db.v1-backup`.
Go to **Settings → Legacy data (v1)** (「设置 → 历史数据（v1）」) and tap **Export old data as Excel** (「导出旧数据为 Excel」) to get them out. The Product library was carried over automatically.
See [Upgrading from v1.1 to v1.2](/en/guide/upgrade-v1.2).
