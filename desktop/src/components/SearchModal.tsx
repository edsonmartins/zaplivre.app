import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { X, Search } from 'lucide-react'

interface SearchResult {
  id: string
  sender_peer_id: string
  recipient_peer_id: string | null
  content: string | null
  created_at: number
}

interface SearchModalProps {
  localPeerId: string | null
  onClose: () => void
  onOpenChat: (peerId: string) => void
}

/**
 * UX-03: busca global de mensagens (comando search_messages, FTS do core)
 */
export default function SearchModal({ localPeerId, onClose, onOpenChat }: SearchModalProps) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<SearchResult[]>([])
  const [isSearching, setIsSearching] = useState(false)
  const [searched, setSearched] = useState(false)

  const handleSearch = async () => {
    const q = query.trim()
    if (!q) return
    setIsSearching(true)
    try {
      const found = await invoke<SearchResult[]>('search_messages', {
        query: q,
        limit: 50,
      })
      setResults(found)
      setSearched(true)
    } catch (error) {
      console.error('Search failed:', error)
      setResults([])
      setSearched(true)
    } finally {
      setIsSearching(false)
    }
  }

  const peerOf = (msg: SearchResult) =>
    msg.sender_peer_id === localPeerId ? msg.recipient_peer_id : msg.sender_peer_id

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl shadow-2xl p-6 w-full max-w-lg mx-4 max-h-[80vh] flex flex-col">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-gray-900">Buscar mensagens</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-gray-600" title="Fechar">
            <X className="w-5 h-5" />
          </button>
        </div>

        <div className="flex gap-2 mb-4">
          <input
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && handleSearch()}
            placeholder="Buscar em todas as conversas..."
            className="input-base flex-1"
            autoFocus
          />
          <button
            onClick={handleSearch}
            disabled={!query.trim() || isSearching}
            className="btn-primary px-4 disabled:opacity-50"
            title="Buscar"
          >
            <Search className="w-5 h-5" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto divide-y divide-gray-100">
          {searched && results.length === 0 && (
            <p className="text-sm text-gray-500 text-center py-8">Nenhuma mensagem encontrada.</p>
          )}
          {results.map((msg) => {
            const peer = peerOf(msg)
            return (
              <button
                key={msg.id}
                onClick={() => peer && onOpenChat(peer)}
                className="w-full text-left px-2 py-3 hover:bg-gray-50"
              >
                <p className="text-sm text-gray-900 truncate">{msg.content ?? '(sem conteúdo)'}</p>
                <p className="text-xs text-gray-500 mt-1 font-mono truncate">
                  {peer ? `${peer.slice(0, 20)}...` : '?'} ·{' '}
                  {new Date(msg.created_at * 1000).toLocaleString()}
                </p>
              </button>
            )
          })}
        </div>
      </div>
    </div>
  )
}
