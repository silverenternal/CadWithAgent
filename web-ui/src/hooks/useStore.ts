import { create } from 'zustand'
import type { Primitive } from '../types'

interface AppState {
  // State
  primitives: Primitive[]
  selectedIds: string[]
  darkMode: boolean
  chatMessages: ChatMessage[]
  isChatLoading: boolean
  
  // Actions
  setPrimitives: (primitives: Primitive[]) => void
  selectPrimitive: (id: string) => void
  deselectPrimitive: (id: string) => void
  clearSelection: () => void
  toggleDarkMode: () => void
  addChatMessage: (message: ChatMessage) => void
  setChatLoading: (loading: boolean) => void
}

export interface ChatMessage {
  id: string
  role: 'user' | 'assistant' | 'system'
  content: string
  timestamp: Date
}

export const useStore = create<AppState>((set, get) => ({
  // Initial state
  primitives: [],
  selectedIds: [],
  darkMode: true,
  chatMessages: [
    {
      id: '1',
      role: 'assistant',
      content: 'Hello! I\'m your AI CAD assistant. How can I help you with your design today?',
      timestamp: new Date(),
    },
  ],
  isChatLoading: false,
  
  // Actions
  setPrimitives: (primitives) => set({ primitives }),
  
  selectPrimitive: (id) => {
    const current = get().selectedIds
    if (!current.includes(id)) {
      set({ selectedIds: [...current, id] })
    }
  },
  
  deselectPrimitive: (id) => {
    const current = get().selectedIds
    set({ selectedIds: current.filter(sid => sid !== id) })
  },
  
  clearSelection: () => set({ selectedIds: [] }),
  
  toggleDarkMode: () => set({ darkMode: !get().darkMode }),
  
  addChatMessage: (message) => {
    const messages = get().chatMessages
    set({ chatMessages: [...messages, message] })
  },
  
  setChatLoading: (loading) => set({ isChatLoading: loading }),
}))
