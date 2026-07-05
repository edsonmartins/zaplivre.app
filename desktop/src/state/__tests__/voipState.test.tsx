/**
 * Testes do VoipStateProvider: hidratação e persistência do estado de
 * chamadas VoIP em localStorage ('mepassa:voip_state').
 */
import { act, renderHook } from '@testing-library/react'
import React from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { VoipStateProvider, useVoipState } from '../voipState'

const STORAGE_KEY = 'mepassa:voip_state'

function wrapper({ children }: { children: React.ReactNode }) {
  return <VoipStateProvider>{children}</VoipStateProvider>
}

describe('voipState', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    localStorage.clear()
  })

  it('useVoipState fora do provider lança erro claro', () => {
    // React loga o erro de render no console - silenciar para não poluir a saída
    vi.spyOn(console, 'error').mockImplementation(() => {})

    expect(() => renderHook(() => useVoipState())).toThrow(
      /useVoipState must be used within VoipStateProvider/
    )
  })

  it('hidrata o estado inicial a partir do localStorage', () => {
    localStorage.setItem(
      STORAGE_KEY,
      JSON.stringify({ 'call-1': { isMuted: true, isSpeakerOn: false } })
    )

    const { result } = renderHook(() => useVoipState(), { wrapper })

    expect(result.current.voipState).toEqual({
      'call-1': { isMuted: true, isSpeakerOn: false },
    })
  })

  it('setVoipState persiste no localStorage (round-trip JSON)', () => {
    const { result } = renderHook(() => useVoipState(), { wrapper })

    act(() => {
      result.current.setVoipState({ 'call-9': { isSpeakerOn: true, cameraSwitchCount: 2 } })
    })

    expect(result.current.voipState).toEqual({
      'call-9': { isSpeakerOn: true, cameraSwitchCount: 2 },
    })
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY)!)).toEqual({
      'call-9': { isSpeakerOn: true, cameraSwitchCount: 2 },
    })
  })

  it('JSON inválido no storage resulta em estado {} sem quebrar', () => {
    localStorage.setItem(STORAGE_KEY, '{isto não é json')

    const { result } = renderHook(() => useVoipState(), { wrapper })

    expect(result.current.voipState).toEqual({})
  })

  it('localStorage.setItem lançando não propaga o erro', () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('QuotaExceededError')
    })

    const { result } = renderHook(() => useVoipState(), { wrapper })

    expect(() => {
      act(() => {
        result.current.setVoipState({ 'call-1': { isMuted: true } })
      })
    }).not.toThrow()

    // O estado em memória continua funcionando mesmo sem persistência
    expect(result.current.voipState).toEqual({ 'call-1': { isMuted: true } })
  })
})
