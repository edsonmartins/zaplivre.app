/**
 * Helpers de formatação compartilhados pelas views.
 *
 * Todos os timestamps vêm do SQLite em SEGUNDOS (unixepoch), nunca em ms.
 */

/** Duração de chamada em mm:ss (ex.: 65 → "01:05") */
export function formatDuration(seconds: number): string {
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
}

/** Hora da mensagem no chat (ex.: "14:32") */
export function formatMessageTime(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

/** Timestamp relativo da lista de conversas ("Just now", "5m ago", …) */
export function formatRelativeTimestamp(timestamp: number | null): string {
  if (!timestamp) return '—'
  const date = new Date(timestamp * 1000)
  const now = new Date()
  const diffMs = now.getTime() - date.getTime()
  const diffMins = Math.floor(diffMs / 60000)
  const diffHours = Math.floor(diffMs / 3600000)
  const diffDays = Math.floor(diffMs / 86400000)

  if (diffMins < 1) return 'Just now'
  if (diffMins < 60) return `${diffMins}m ago`
  if (diffHours < 24) return `${diffHours}h ago`
  if (diffDays < 7) return `${diffDays}d ago`
  return date.toLocaleDateString()
}

/** Data curta (ex.: data de criação do grupo) */
export function formatDate(timestamp: number): string {
  const date = new Date(timestamp * 1000)
  return date.toLocaleDateString()
}
