import React, { createContext, useContext, useMemo, useState, useEffect } from 'react'

export type VoipCallState = {
  isMuted?: boolean
  isSpeakerOn?: boolean
  cameraSwitchCount?: number
  lastCameraSwitchAt?: number
}

export type VoipState = Record<string, VoipCallState>

type VoipStateContextValue = {
  voipState: VoipState
  setVoipState: React.Dispatch<React.SetStateAction<VoipState>>
}

const STORAGE_KEY = 'mepassa:voip_state'
const VoipStateContext = createContext<VoipStateContextValue | null>(null)

export function VoipStateProvider({ children }: { children: React.ReactNode }) {
  const [voipState, setVoipState] = useState<VoipState>(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY)
      return raw ? (JSON.parse(raw) as VoipState) : {}
    } catch {
      return {}
    }
  })

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(voipState))
    } catch {
      // Ignore storage errors (private mode, etc.)
    }
  }, [voipState])
  const value = useMemo(() => ({ voipState, setVoipState }), [voipState])
  return <VoipStateContext.Provider value={value}>{children}</VoipStateContext.Provider>
}

export function useVoipState() {
  const ctx = useContext(VoipStateContext)
  if (!ctx) {
    throw new Error('useVoipState must be used within VoipStateProvider')
  }
  return ctx
}
