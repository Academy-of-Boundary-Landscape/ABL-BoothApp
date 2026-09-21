import { io } from 'socket.io-client'
import { SERVER_ORIGIN } from './url'

const URL = import.meta.env.VITE_API_URL || SERVER_ORIGIN

export const socket = io(URL, {
  path: '/socket.io',
  autoConnect: false,
  namespace: '/sale',
  // FIXME(1.2): 这里的意图不明——原注释写「强制只用 polling，禁用 websocket」，
  // 但 transports 把 websocket 排在第一位。改动会影响局域网实时订单推送，
  // 需要在真机上验证过再动。见 docs/superpowers/specs/2026-09-22-safety-net-design.md
  transports: ['websocket', 'polling'],
})
