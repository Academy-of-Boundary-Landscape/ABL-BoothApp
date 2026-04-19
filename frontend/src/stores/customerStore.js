import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import api from '@/services/api';
import { socket } from '@/services/socketService';
import { useAlert } from '@/services/useAlert';
import { getImageUrl } from '@/services/url';

// 获取弹窗函数


export const useCustomerStore = defineStore('customer', () => {
  // --- State ---
  const products = ref([]);
  const cart = ref([]);
  const isLoading = ref(false);
  const error = ref(null);
  const activeEventId = ref(null);
  const activeEvent = ref(null); // 新增：当前展会信息

  // --- Actions ---
  async function setupStoreForEvent(eventId) {
    if (!eventId) {
      error.value = "未提供展会ID。";
      return;
    }
    activeEventId.value = parseInt(eventId, 10);
    // 并行加载商品和展会信息，都完成后再允许下单
    await Promise.all([
      fetchProductsForEvent(),
      fetchEventInfo(),
    ]);
  }
  
  async function initializeEventFromUrl() {
    const urlParams = new URLSearchParams(window.location.search);
    const eventIdFromUrl = urlParams.get('event');
    if (eventIdFromUrl) {
      activeEventId.value = parseInt(eventIdFromUrl, 10);
      await Promise.all([
        fetchProductsForEvent(),
        fetchEventInfo(),
      ]);
    } else {
      error.value = "未指定展会ID，无法加载商品。请使用包含 ?event=ID 的链接访问。";
    }
  }
  
  // 获取商品列表 (HTTP)
  async function fetchProductsForEvent() {
    if (!activeEventId.value) return;
    isLoading.value = true;
    error.value = null;
    try {
      const response = await api.get(`/events/${activeEventId.value}/products`);
      products.value = response.data.map(product => ({
        ...product,
        image_url: getImageUrl(product.image_url)
      }));
    } catch (err) {
      error.value = '加载商品失败，请联系摊主。';
      console.error(err);
    } finally {
      isLoading.value = false;
    }
  }

  // 新增：获取展会信息
  async function fetchEventInfo() {
    if (!activeEventId.value) return;
    try {
      const response = await api.get(`/events/${activeEventId.value}`);
      activeEvent.value = {
        ...response.data,
        qrcode_url: getImageUrl(response.data.qrcode_url),
        qrcode_urls: (response.data.qrcode_urls || []).map(u => getImageUrl(u)).filter(Boolean)
      };
    } catch (err) {
      console.error('加载展会信息失败:', err);
      // 不设置error，因为这不是关键功能
    }
  }

  // --- 购物车操作 ---

  function addToCart(product) {
    const existingItem = cart.value.find(item => item.id === product.id);
    if (existingItem) {
      if (existingItem.quantity < product.current_stock) {
        existingItem.quantity++;
      } else {
        const { showAlert, showSuccess, showError } = useAlert();
        showError(`抱歉，"${product.name}" 库存不足！`);
      }
    } else {
      if (product.current_stock > 0) {
        cart.value.push({ ...product, quantity: 1 });
      }
    }
  }

  function removeFromCart(productId) {
    const itemIndex = cart.value.findIndex(item => item.id === productId);
    if (itemIndex !== -1) {
      if (cart.value[itemIndex].quantity > 1) {
        cart.value[itemIndex].quantity--;
      } else {
        cart.value.splice(itemIndex, 1);
      }
    }
  }

  function clearCart() {
    cart.value = [];
  }

  async function submitOrder() {
    if (!activeEventId.value || cart.value.length === 0) return;

    const orderData = {
      items: cart.value.map(item => ({
        product_id: item.id,
        quantity: item.quantity,
      })),
    };

    try {
      // 使用 axios.post 发送 HTTP 请求
      const response = await api.post(`/events/${activeEventId.value}/orders`, orderData);
      // 成功后返回订单数据，让视图可以触发后续操作（如弹窗）
      return response.data;
    } catch (err) {
      console.error("Order submission failed:", err);
      
      
      
      throw new Error(err.response?.data?.error || '下单失败，请重试。');


    }
  }


  // --- Getters ---
  const cartTotal = computed(() => {
    return cart.value.reduce((total, item) => total + item.price * item.quantity, 0);
  });
  
  const cartItemCount = computed(() => {
    return cart.value.reduce((total, item) => total + item.quantity, 0);
  });

  // 收款码 URL（向后兼容单码）
  const qrCodeUrl = computed(() => {
    return activeEvent.value?.qrcode_url || null;
  });

  // 收款码 URL 数组（多码）
  const qrCodeUrls = computed(() => {
    return activeEvent.value?.qrcode_urls || [];
  });


  return {
    products,
    cart,
    isLoading,
    error,
    activeEventId,
    activeEvent, // 新增：导出展会信息
    qrCodeUrl,
    qrCodeUrls,
    fetchProductsForEvent,
    fetchEventInfo, // 新增：导出获取展会信息方法
    addToCart,
    removeFromCart,
    clearCart,
    submitOrder,
    cartTotal,
    cartItemCount,
    initializeEventFromUrl,
    setupStoreForEvent,
  };
});
