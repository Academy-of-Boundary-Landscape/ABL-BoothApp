# Network Connection FAQ

## After scanning the QR code with my phone, it shows "Connection Timeout" or the webpage cannot be opened?

This is the most common issue. Please troubleshoot in the following order:

1.  **Check the Hotspot**: Ensure the host device (computer/tablet) and the phone that scanned the QR code are connected to the same Wi-Fi/hotspot.
2.  **Firewall Blocking**: For Windows hosts, check if the firewall has allowed "TanBox" through, or try temporarily disabling the firewall.
3.  **IP Address Changed**: If you restarted the hotspot, the IP address might have changed. Please click "Regenerate QR Code" on the host device.

## Can it be used without internet or with poor signal?

**Absolutely.** TanBox is designed with an "offline-first" approach. We recommend using one device to create a **mobile hotspot** to form a local network, and have other devices connect to it. This does not require an internet connection and does not consume mobile data.

## I'm on a campus/public Wi-Fi, and devices can't connect to each other?

Public networks often have "AP Isolation" enabled, which prevents devices from communicating with each other. **You must use a hotspot** to create the network. This is the most reliable solution for convention venues.

## If the network disconnects midway, will data be lost?

**No.** As long as the host device (the App side) is not closed, the data persists. After reconnecting to the network, simply refresh the page on the mobile device to restore the previous state.