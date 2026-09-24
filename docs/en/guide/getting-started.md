# Getting Started

This guide will help you set up BoothKernel in 5 minutes.

## Download & Installation

### Windows
1. Download the latest `.exe` installer from [GitHub Releases](https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases)
2. Double-click to install
3. Launch the application

### Android
1. Download the `.apk` file from [GitHub Releases](https://github.com/Academy-of-Boundary-Landscape/ABL-BoothApp/releases)
2. Enable "Install from Unknown Sources" in Settings
3. Install and open the app

## First Launch

### 1. Choose Your Role

On first launch, select your role:
- **Admin**: Manage products, view statistics, export data
- **Vendor**: Take orders, process payments
- **Customer Portal**: Display products for customers

### 2. Set Up Products (Admin Only)

Navigate to "Master Products" to add your items:
1. Click "Add Product"
2. Enter name, price, and initial stock
3. Optionally upload product images
4. Save

### 3. Create an Event

1. Go to "Events" → "Create Event"
2. Enter event name and date
3. Select products to sell
4. Start the event

## Multi-Device Setup

### Connect Devices via LAN

1. **Admin device**: Note the IP address displayed in the app
2. **Other devices**: Enter the admin IP in connection settings
3. All devices should be on the same network (WiFi/hotspot)

That's it! You're ready to take orders.

## Closing & Settlement

When the event is over, open the **Vendor** screen and tap **Closing** in the top right. The wizard has four steps:

1. **Clear orders** — complete or cancel every pending order first. The remaining steps are blocked while any pending order is left.
2. **Stocktake** (optional) — enter the actual remaining quantity of each item. If you skip it, the settlement sheet is marked as an estimate ("not stocktaken").
3. **Take back** — confirm the stock you are carrying home; on-site stock drops to zero.
4. **Settle** — freezes the event: its status becomes **Settled**.

After settling, orders, refunds, restocks, gifts, scraps, stocktakes, take-backs and re-settling are all refused. Only advances, settlement adjustments and a settlement recount can still be added later. The settlement sheet is available in the admin app under **Events → Settlement**.

## Next Steps

- [Network Setup Guide](/en/guide/network)
- [Workflow Guide](/en/guide/workflow)
- [FAQ](/en/faq/)
