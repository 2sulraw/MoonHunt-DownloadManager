import { ref } from 'vue'

export interface Notification {
  id: string
  message: string
  type: 'success' | 'error' | 'info'
  timestamp: number
}

const notifications = ref<Notification[]>([])
let nextNotificationId = 1
const dismissalTimers = new Map<string, ReturnType<typeof setTimeout>>()

export function useNotification() {
  function notify(message: string, type: 'success' | 'error' | 'info' = 'info') {
    const id = `notif-${nextNotificationId++}`
    const notification: Notification = {
      id,
      message,
      type,
      timestamp: Date.now(),
    }

    notifications.value.push(notification)

    // Auto-remove after 3 seconds
    const timer = setTimeout(() => {
      notifications.value = notifications.value.filter(n => n.id !== id)
      dismissalTimers.delete(id)
    }, 3000)

    dismissalTimers.set(id, timer)
  }

  return {
    notifications,
    notify,
  }
}
