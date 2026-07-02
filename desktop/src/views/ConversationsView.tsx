import { useEffect, useState, useRef } from 'react'
import { useNavigate } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import QRCodeModal from '../components/QRCodeModal'

interface Conversation {
  id: string
  peer_id: string | null
  display_name: string | null
  last_message_id: string | null
  last_message_at: number | null
  unread_count: number
}

interface ConversationsViewProps {
  localPeerId: string | null
}

export default function ConversationsView({ localPeerId }: ConversationsViewProps) {
  const [conversations, setConversations] = useState<Conversation[]>([])
  const [isLoading, setIsLoading] = useState(true)
  const [showNewChatDialog, setShowNewChatDialog] = useState(false)
  const [showQRModal, setShowQRModal] = useState(false)
  const [newPeerId, setNewPeerId] = useState('')
  const [peerCount, setPeerCount] = useState(0)
  const navigate = useNavigate()
  const previousConversations = useRef<Conversation[]>([])
  useEffect(() => {
    loadConversations()
    loadPeerCount()

    // EVT-03: recarregar a lista quando o core avisa de mensagem nova
    let unsubs: Array<() => void> = []
    const register = async () => {
      const received = await listen('message:received', () => loadConversations())
      const status = await listen('message:status', () => loadConversations())
      unsubs = [received, status]
    }
    register()

    // Contagem de peers + safety net em intervalo lento
    const interval = setInterval(() => {
      loadPeerCount()
      loadConversations()
    }, 30000)

    return () => {
      clearInterval(interval)
      unsubs.forEach((unsub) => unsub())
    }
  }, [])

  const loadConversations = async () => {
    try {
      const convs = await invoke<Conversation[]>('list_conversations')

      // Detect new messages
      if (previousConversations.current.length > 0) {
        for (const newConv of convs) {
          const oldConv = previousConversations.current.find(c => c.peer_id === newConv.peer_id)

          // New conversation or new unread messages
          if (!oldConv || (newConv.unread_count > 0 && newConv.unread_count > oldConv.unread_count)) {
            // Show notification
            try {
              const label = newConv.display_name || newConv.peer_id || 'Contato'
              await invoke('show_notification', {
                title: 'Nova mensagem',
                body: `Mensagem de ${label.substring(0, 16)}...`
              })
            } catch (error) {
              console.error('Failed to show notification:', error)
            }
          }
        }
      }

      // Update state
      previousConversations.current = convs
      setConversations(convs)
      // Sender keys de grupo agora são distribuídas pelo core (protocolo
      // in-band) - a varredura manual de mensagens foi removida
    } catch (error) {
      console.error('Failed to load conversations:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const loadPeerCount = async () => {
    try {
      const count = await invoke<number>('get_connected_peers_count')
      setPeerCount(count)
    } catch (error) {
      console.error('Failed to load peer count:', error)
    }
  }

  const handleNewChat = async () => {
    if (!newPeerId.trim()) return

    try {
      // Navigate to chat view
      navigate(`/chat/${newPeerId}`)
      setShowNewChatDialog(false)
      setNewPeerId('')
    } catch (error) {
      console.error('Failed to start new chat:', error)
    }
  }

  const formatTimestamp = (timestamp: number | null): string => {
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

  return (
    <div className="flex flex-col h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold text-gray-900">MePassa</h1>
            {localPeerId && (
              <p className="text-xs text-gray-500 font-mono truncate max-w-xs">
                {localPeerId}
              </p>
            )}
          </div>
          <div className="flex items-center space-x-4">
            <div className="flex items-center space-x-2 text-sm text-gray-600">
              <div className={`w-2 h-2 rounded-full ${peerCount > 0 ? 'bg-green-500' : 'bg-gray-400'}`}></div>
              <span>{peerCount} peers</span>
            </div>
            <button
              onClick={() => setShowQRModal(true)}
              className="btn-secondary text-sm flex items-center gap-2"
              title="Ver meu QR Code"
              disabled={!localPeerId}
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v1m6 11h2m-6 0h-2v4m0-11v3m0 0h.01M12 12h4.01M16 20h4M4 12h4m12 0h.01M5 8h2a1 1 0 001-1V5a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1zm12 0h2a1 1 0 001-1V5a1 1 0 00-1-1h-2a1 1 0 00-1 1v2a1 1 0 001 1zM5 20h2a1 1 0 001-1v-2a1 1 0 00-1-1H5a1 1 0 00-1 1v2a1 1 0 001 1z" />
              </svg>
              QR Code
            </button>
            <button
              onClick={() => navigate('/groups')}
              className="btn-secondary text-sm"
            >
              Groups
            </button>
            <button
              onClick={() => setShowNewChatDialog(true)}
              className="btn-primary text-sm"
            >
              + New Chat
            </button>
          </div>
        </div>
      </div>

      {/* Conversations List */}
      <div className="flex-1 overflow-y-auto">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="animate-spin rounded-full h-12 w-12 border-b-4 border-primary-500"></div>
          </div>
        ) : conversations.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center px-6">
            <svg
              className="w-24 h-24 text-gray-300 mb-4"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z"
              />
            </svg>
            <h2 className="text-xl font-semibold text-gray-900 mb-2">No conversations yet</h2>
            <p className="text-gray-600 mb-6">
              Start a new chat by clicking the "New Chat" button
            </p>
            <button
              onClick={() => setShowNewChatDialog(true)}
              className="btn-primary"
            >
              Start Your First Chat
            </button>
          </div>
        ) : (
          <div className="divide-y divide-gray-200">
            {conversations.map((conv) => (
              <div
                key={conv.peer_id || conv.id}
                onClick={() => conv.peer_id && navigate(`/chat/${conv.peer_id}`)}
                className={`px-6 py-4 hover:bg-gray-50 transition-colors ${conv.peer_id ? 'cursor-pointer' : 'cursor-not-allowed opacity-70'}`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center justify-between mb-1">
                      <p className="text-sm font-semibold text-gray-900 truncate">
                        {(conv.display_name || conv.peer_id || 'Contato').substring(0, 16)}...
                      </p>
                      <p className="text-xs text-gray-500 ml-2">
                        {formatTimestamp(conv.last_message_at)}
                      </p>
                    </div>
                    <p className="text-sm text-gray-600 truncate">
                      {conv.last_message_id ? 'Última mensagem disponível' : 'No messages yet'}
                    </p>
                  </div>
                  {conv.unread_count > 0 && (
                    <div className="ml-4">
                      <span className="inline-flex items-center justify-center w-6 h-6 text-xs font-bold text-white bg-primary-500 rounded-full">
                        {conv.unread_count}
                      </span>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* New Chat Dialog */}
      {showNewChatDialog && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-2xl shadow-2xl p-6 w-full max-w-md mx-4">
            <h2 className="text-xl font-bold text-gray-900 mb-4">New Chat</h2>
            <input
              type="text"
              value={newPeerId}
              onChange={(e) => setNewPeerId(e.target.value)}
              placeholder="Enter peer ID..."
              className="input-base mb-4"
              autoFocus
              onKeyPress={(e) => e.key === 'Enter' && handleNewChat()}
            />
            <div className="flex space-x-3">
              <button
                onClick={() => setShowNewChatDialog(false)}
                className="btn-secondary flex-1"
              >
                Cancel
              </button>
              <button
                onClick={handleNewChat}
                disabled={!newPeerId.trim()}
                className="btn-primary flex-1 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Start Chat
              </button>
            </div>
          </div>
        </div>
      )}

      {/* QR Code Modal */}
      {showQRModal && localPeerId && (
        <QRCodeModal
          localPeerId={localPeerId}
          onClose={() => setShowQRModal(false)}
        />
      )}
    </div>
  )
}
