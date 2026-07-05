import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  formatDate,
  formatDuration,
  formatMessageTime,
  formatRelativeTimestamp,
} from '../format'

describe('formatDuration', () => {
  it('formata zero como 00:00', () => {
    expect(formatDuration(0)).toBe('00:00')
  })

  it('formata 65s como 01:05', () => {
    expect(formatDuration(65)).toBe('01:05')
  })

  it('formata durações longas (1h+) só em minutos', () => {
    expect(formatDuration(3600)).toBe('60:00')
  })
})

describe('formatMessageTime', () => {
  it('interpreta o timestamp em SEGUNDOS (unixepoch do SQLite), não em ms', () => {
    // Se fosse tratado como ms, cairia em 1970 - regressão que o ChatView já vigiava
    const ts = 1_700_000_000 // 2023-11-14
    const expected = new Date(ts * 1000).toLocaleTimeString([], {
      hour: '2-digit',
      minute: '2-digit',
    })
    expect(formatMessageTime(ts)).toBe(expected)
  })
})

describe('formatRelativeTimestamp', () => {
  const NOW = new Date('2026-07-05T12:00:00Z')

  beforeEach(() => {
    vi.useFakeTimers()
    vi.setSystemTime(NOW)
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  const secondsAgo = (s: number) => Math.floor(NOW.getTime() / 1000) - s

  it('retorna travessão para null', () => {
    expect(formatRelativeTimestamp(null)).toBe('—')
  })

  it('menos de 1 minuto vira "Just now"', () => {
    expect(formatRelativeTimestamp(secondsAgo(30))).toBe('Just now')
  })

  it('59 minutos atrás vira "59m ago"', () => {
    expect(formatRelativeTimestamp(secondsAgo(59 * 60))).toBe('59m ago')
  })

  it('23 horas atrás vira "23h ago"', () => {
    expect(formatRelativeTimestamp(secondsAgo(23 * 3600))).toBe('23h ago')
  })

  it('6 dias atrás vira "6d ago"', () => {
    expect(formatRelativeTimestamp(secondsAgo(6 * 86400))).toBe('6d ago')
  })

  it('7 dias ou mais vira data local', () => {
    const ts = secondsAgo(7 * 86400)
    expect(formatRelativeTimestamp(ts)).toBe(new Date(ts * 1000).toLocaleDateString())
  })
})

describe('formatDate', () => {
  it('formata timestamp em segundos como data local', () => {
    const ts = 1_700_000_000
    expect(formatDate(ts)).toBe(new Date(ts * 1000).toLocaleDateString())
  })
})
