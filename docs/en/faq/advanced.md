---
title: Advanced Tips
---

# Advanced Tips

Here are some "official features may not be complex, but you can achieve them with clever tricks" methods.

## How to Set Up Discounts / Spend-and-Save Promotions?

To keep the accounting logic simple, the system currently lacks complex coupon features.

**Trick:**  
You can create a product named "Discount/Rounding" and set its price to **-5 yuan**.  
Add this product to the order during checkout, and the total price will automatically be reduced by 5 yuan.

## Does it Support "Bundled Sales" or "Sets"?

It is recommended to directly create a new product named "XX Set" and set its bundled price.  
This way, when tracking sales, you can clearly see how many sets have been sold.

## How Should I Handle Sold-Out Products?

When stock is insufficient or sold out, the customer-facing interface will automatically gray out the product and prevent ordering.  
You do not need to manually hide or take down the product.