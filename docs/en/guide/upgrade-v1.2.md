---
title: Upgrading from v1.1 to v1.2
description: v1.2 switches to a new ledger model. Before upgrading, know this — old events and orders are not migrated, but they are kept intact and can be exported at any time.
---

# Upgrading from v1.1 to v1.2

v1.2 replaces BoothKernel's ledger. Instead of "just a stock count", it's now a double-entry ledger where "every item and every yuan has a source and a destination".
Bundle deals, refunds, gifts/scrap, closing stocktake and consignment payouts are all built on top of it.

The cost: **old events and orders can't be moved into the new ledger**. The old data only knows "how many are left now";
it can't reconstruct "where this item came from, who it was sold to, which channel the money went into".

:::warning Read this whole section before upgrading
- **The Product library comes along automatically**: products, categories, tags, circles, images, AI recognition images and recognition data are all kept.
- **Old events, orders and stock will not appear in v1.2**. They are not deleted; they're kept as-is in a backup file and can be exported to Excel at any time.
- **Don't upgrade on event day or in the middle of an event.** Finish the current event, export its sales report, then upgrade.
:::

## Upgrade steps

1. **Wrap up the event in progress first**: in v1.1, export this event's sales report (**Sales stats → Download Excel**, 「销售统计 → 下载 Excel」).
2. **Update the app**:
   - Windows (v1.1.1 and later): **Settings → About & updates → Check for updates** (「设置 → 关于与更新 → 检查更新」) downloads and installs in one click. v1.1.0 needs you to download the installer and install over it manually, see [Auto update (Chinese)](/guide/auto-update).
   - Android: download the new APK from [GitHub Releases](https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases) and install over the old one (the signature is unchanged, no need to uninstall first, and **don't uninstall first** — uninstalling wipes your data too).
3. **Open the app and log in to the admin console**. If the old database had events or orders, a one-time notice **v1.2 has a new ledger model** (「**v1.2 更新了账本模型**」) pops up:
   - Tap **Export old data to Excel** (「**导出旧数据为 Excel**」) to get `legacy_v1_export.xlsx`, containing all old events and orders;
   - Tap **Got it** (「**知道了**」) to close.
4. To export again later, go to **Settings → Legacy data (v1)** (「**设置 → 历史数据（v1）**」) and tap **Export old data to Excel** at any time.

## Where the old data is

During the upgrade, the old database is **renamed and kept** as `sale_system.db.v1-backup`, in the same folder as the new database:

- Windows: `%APPDATA%\com.abl.BoothKernel\`
- Android: the app's private folder; you can only get it out via **Export old data to Excel** above

BoothKernel never deletes this file automatically.

## What to do after upgrading

1. **Check circle ownership**. In v1.2 every product belongs to a **Circle** (its Owner), and settlement splits accounts by circle.
   If you only sell your own stuff, nothing to do: products default to **Your circle**. If you sell on consignment for other circles, first create them on the **Circles** page (「**社团**」),
   then change the **Circle** (「所属社团」) of their products in the Product library. See [Closing & Settlement](./closing).
2. **List products in a new event**. Old events didn't come along, so for your next event create a new one and use **Pick from Product library** (「从商品库选」) under **Pre-event · Products** (「展前 · 商品」) in the workbench.
3. **Disable "negative-price discount products"**. In v1.1 we suggested "create a -¥5 product as a discount". v1.2 has proper
   [Bundles & Deals](./lots) and the **Amount received** (「实收」) field for adjusting the price at payment. Negative-price products pollute the stats, so disable them in the Product library.
4. **Change the default passwords**. The admin console keeps reminding you at the top until you change `admin123` / `vendor123`.

## Will future upgrades wipe data again?

No. This reset is a one-time cost of switching ledger models. From v1.2 on, the app takes a snapshot of the database before every upgrade,
restores it automatically if the upgrade fails, and only changes the database structure incrementally.
