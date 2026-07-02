import { useEffect, useState, useRef } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

interface Message {
  id: string
  sender_peer_id: string
  recipient_peer_id: string
  content: string | null
  message_type: string
  created_at: number
  status: string
}

interface MediaItem {
  id: number
  media_hash: string
  message_id: string
  media_type: string
  file_name: string | null
  file_size: number | null
  mime_type: string | null
  local_path: string | null
  thumbnail_path: string | null
  width: number | null
  height: number | null
  duration_seconds: number | null
}

interface ChatViewProps {
  localPeerId: string | null
}

export default function ChatView({ localPeerId }: ChatViewProps) {
  const { peerId } = useParams<{ peerId: string }>()
  const navigate = useNavigate()
  const [messages, setMessages] = useState<Message[]>([])
  const [mediaIndex, setMediaIndex] = useState<Record<string, MediaItem>>({})
  const [mediaUrls, setMediaUrls] = useState<Record<string, string>>({})
  const [reactions, setReactions] = useState<Record<string, { emoji: string; peer_id: string }[]>>({})
  const [forwardMessageId, setForwardMessageId] = useState<string | null>(null)
  const [forwardTargets, setForwardTargets] = useState<{ peer_id: string; display_name: string | null }[]>([])

  const QUICK_EMOJIS = ['👍', '❤️', '😂', '😮']

  const loadReactions = async (msgs: Message[]) => {
    try {
      const entries = await Promise.all(
        msgs.map(async (m) => {
          const r = await invoke<{ emoji: string; peer_id: string }[]>(
            'get_message_reactions',
            { messageId: m.id }
          )
          return [m.id, r] as const
        })
      )
      setReactions(Object.fromEntries(entries.filter(([, r]) => r.length > 0)))
    } catch (error) {
      console.error('Failed to load reactions:', error)
    }
  }

  const toggleReaction = async (messageId: string, emoji: string) => {
    const mine = (reactions[messageId] ?? []).some(
      (r) => r.emoji === emoji && r.peer_id === localPeerId
    )
    try {
      await invoke(mine ? 'remove_reaction' : 'add_reaction', { messageId, emoji })
      const updated = await invoke<{ emoji: string; peer_id: string }[]>(
        'get_message_reactions',
        { messageId }
      )
      setReactions((prev) => ({ ...prev, [messageId]: updated }))
    } catch (error) {
      console.error('Failed to toggle reaction:', error)
    }
  }

  const openForward = async (messageId: string) => {
    try {
      const convs = await invoke<{ peer_id: string | null; display_name: string | null }[]>(
        'list_conversations'
      )
      setForwardTargets(
        convs
          .filter((c): c is { peer_id: string; display_name: string | null } =>
            Boolean(c.peer_id) && c.peer_id !== peerId
          )
      )
      setForwardMessageId(messageId)
    } catch (error) {
      console.error('Failed to load forward targets:', error)
    }
  }

  const handleForward = async (toPeerId: string) => {
    if (!forwardMessageId) return
    try {
      await invoke('forward_message', { messageId: forwardMessageId, toPeerId })
    } catch (error) {
      console.error('Failed to forward:', error)
    } finally {
      setForwardMessageId(null)
    }
  }
  const [newMessage, setNewMessage] = useState('')
  const [isSending, setIsSending] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const scrollContainerRef = useRef<HTMLDivElement>(null)
  const previousMessageCount = useRef<number>(0)
  // Filtro de exibição para mensagens LEGADAS do hack antigo de sender key
  // (a distribuição agora é in-band no core e não gera mensagens de chat)
  const legacyGroupKeyPrefix = 'mepassa-group-key:v1:'

  useEffect(() => {
    if (!peerId) return

    loadMessages()
    loadMediaIndex()
    markAsRead()

    // EVT-03: eventos do core substituem o polling de 2s
    let unsubs: Array<() => void> = []
    const register = async () => {
      const received = await listen<{ message_id: string; from_peer_id: string }>(
        'message:received',
        (event) => {
          if (event.payload.from_peer_id === peerId) {
            loadMessages()
            markAsRead()
          }
        }
      )
      const status = await listen<{ message_id: string; status: string }>(
        'message:status',
        () => loadMessages()
      )
      unsubs = [received, status]
    }
    register()

    // Safety net: refresh lento caso algum evento se perca
    const interval = setInterval(loadMessages, 30000)

    return () => {
      clearInterval(interval)
      unsubs.forEach((unsub) => unsub())
    }
  }, [peerId])

  useEffect(() => {
    const el = scrollContainerRef.current
    if (!el) return
    const threshold = 24
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight
    const shouldStick = distanceFromBottom <= threshold || previousMessageCount.current === 0
    if (shouldStick) {
      scrollToBottom()
    }
  }, [messages])

  const loadMessages = async () => {
    if (!peerId) return

    try {
      const msgs = await invoke<Message[]>('get_conversation_messages', {
        peerId,
        limit: 100,
        offset: 0,
      })

      const filtered = msgs.filter(
        (msg) => !(msg.content && msg.content.startsWith(legacyGroupKeyPrefix))
      )

      // Detect new received messages
      if (previousMessageCount.current > 0 && filtered.length > previousMessageCount.current) {
        const newMessages = filtered.slice(previousMessageCount.current)
        for (const msg of newMessages) {
          // Only notify for received messages (not sent by me)
          if (msg.sender_peer_id !== localPeerId) {
            try {
              await invoke('show_notification', {
                title: `Nova mensagem de ${msg.sender_peer_id.substring(0, 8)}...`,
                body: (msg.content || '').substring(0, 100)
              })
            } catch (error) {
              console.error('Failed to show notification:', error)
            }
          }
        }
      }

      const ordered = [...filtered].sort((a, b) => a.created_at - b.created_at)
      loadReactions(ordered)
      previousMessageCount.current = ordered.length
      setMessages(ordered)
    } catch (error) {
      console.error('Failed to load messages:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const loadMediaIndex = async () => {
    if (!peerId) return

    try {
      const conversationId = `1:1:${peerId}`
      const media = await invoke<MediaItem[]>('get_conversation_media', {
        conversationId,
        mediaType: null,
        limit: 200,
      })

      const index: Record<string, MediaItem> = {}
      for (const item of media) {
        index[item.message_id] = item
      }
      setMediaIndex(index)
    } catch (error) {
      console.error('Failed to load media index:', error)
    }
  }

  useEffect(() => {
    const pendingImages = messages.filter(
      (msg) => msg.message_type === 'image' && mediaIndex[msg.id]
    )

    for (const msg of pendingImages) {
      const media = mediaIndex[msg.id]
      if (!media) continue
      if (mediaUrls[media.media_hash]) continue

      void (async () => {
        try {
          const base64 = await invoke<string>('download_media', {
            mediaHash: media.media_hash,
          })
          const binary = atob(base64)
          const len = binary.length
          const bytes = new Uint8Array(len)
          for (let i = 0; i < len; i += 1) {
            bytes[i] = binary.charCodeAt(i)
          }
          const blob = new Blob([bytes], { type: media.mime_type || 'image/jpeg' })
          const url = URL.createObjectURL(blob)
          setMediaUrls((prev) => ({ ...prev, [media.media_hash]: url }))
        } catch (error) {
          console.error('Failed to download media:', error)
        }
      })()
    }

    return () => {
      // cleanup happens on unmount
    }
  }, [messages, mediaIndex, mediaUrls])

  const markAsRead = async () => {
    if (!peerId) return

    try {
      await invoke('mark_conversation_read', { peerId })
    } catch (error) {
      console.error('Failed to mark as read:', error)
    }
  }

  // UX-02: anexar arquivo (imagem vai comprimida; o resto como documento)
  const handleAttachFile = async () => {
    if (!peerId) return

    try {
      const { open } = await import('@tauri-apps/plugin-dialog')
      const selected = await open({ multiple: false, title: 'Enviar arquivo' })
      if (!selected || typeof selected !== 'string') return

      setIsSending(true)
      await invoke<string>('send_file_message', {
        toPeerId: peerId,
        filePath: selected,
      })
      await loadMessages()
      await loadMediaIndex()
    } catch (error) {
      console.error('Failed to send file:', error)
    } finally {
      setIsSending(false)
    }
  }

  const handleSend = async () => {
    if (!newMessage.trim() || !peerId || isSending) return

    setIsSending(true)

    try {
      await invoke('send_text_message', {
        toPeerId: peerId,
        content: newMessage.trim(),
      })

      setNewMessage('')
      // Reload messages to show sent message
      await loadMessages()
    } catch (error) {
      console.error('Failed to send message:', error)
      alert('Failed to send message. Please try again.')
    } finally {
      setIsSending(false)
    }
  }

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }

  const handleScroll = () => {
    // no-op: reserved for future scroll indicators
  }

  const formatTime = (timestamp: number): string => {
    // created_at vem do SQLite em SEGUNDOS (unixepoch)
    const date = new Date(timestamp * 1000)
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
  }

  const isSentByMe = (msg: Message): boolean => {
    return msg.sender_peer_id === localPeerId
  }

  const handleStartCall = async () => {
    if (!peerId) return

    try {
      const callId = await invoke<string>('start_call', { toPeerId: peerId })
      navigate(`/call/${callId}/${peerId}`)
    } catch (error) {
      console.error('Failed to start call:', error)
      alert('Failed to start call. Please try again.')
    }
  }

  return (
    <div className="flex flex-col h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center">
          <button
            onClick={() => navigate('/conversations')}
            className="mr-4 text-gray-600 hover:text-gray-900"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15 19l-7-7 7-7"
              />
            </svg>
          </button>
          <div className="flex-1">
            <h2 className="text-lg font-semibold text-gray-900">
              {peerId?.substring(0, 16)}...
            </h2>
            <p className="text-xs text-gray-500 font-mono truncate max-w-md">{peerId}</p>
          </div>
          <button
            onClick={handleStartCall}
            className="ml-4 text-primary-600 hover:text-primary-700 p-2 rounded-full hover:bg-primary-50 transition-colors"
            title="Iniciar chamada"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M3 5a2 2 0 012-2h3.28a1 1 0 01.948.684l1.498 4.493a1 1 0 01-.502 1.21l-2.257 1.13a11.042 11.042 0 005.516 5.516l1.13-2.257a1 1 0 011.21-.502l4.493 1.498a1 1 0 01.684.949V19a2 2 0 01-2 2h-1C9.716 21 3 14.284 3 6V5z"
              />
            </svg>
          </button>
        </div>
      </div>

      {/* Messages */}
      <div
        className="flex-1 overflow-y-auto px-6 py-4 space-y-4"
        ref={scrollContainerRef}
        onScroll={handleScroll}
      >
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="animate-spin rounded-full h-12 w-12 border-b-4 border-primary-500"></div>
          </div>
        ) : messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <svg
              className="w-20 h-20 text-gray-300 mb-4"
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
            <h3 className="text-lg font-semibold text-gray-900 mb-2">No messages yet</h3>
            <p className="text-gray-600">Send a message to start the conversation</p>
          </div>
        ) : (
          <>
            {messages.map((msg) => (
              <div
                key={msg.id}
                className={`group flex ${isSentByMe(msg) ? 'justify-end' : 'justify-start'}`}
              >
                {/* Ações no hover (UX-02): reações rápidas + encaminhar */}
                <div
                  className={`hidden group-hover:flex items-center gap-1 self-center mx-2 ${
                    isSentByMe(msg) ? 'order-first' : 'order-last'
                  }`}
                >
                  {QUICK_EMOJIS.map((emoji) => (
                    <button
                      key={emoji}
                      onClick={() => toggleReaction(msg.id, emoji)}
                      className="text-sm hover:scale-125 transition-transform"
                      title={`Reagir ${emoji}`}
                    >
                      {emoji}
                    </button>
                  ))}
                  <button
                    onClick={() => openForward(msg.id)}
                    className="text-gray-400 hover:text-gray-600 ml-1"
                    title="Encaminhar"
                  >
                    <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 5l7 7-7 7M5 5l7 7-7 7" />
                    </svg>
                  </button>
                </div>
                <div className={isSentByMe(msg) ? 'message-sent' : 'message-received'}>
                  {msg.message_type === 'image' && mediaIndex[msg.id] ? (
                    mediaUrls[mediaIndex[msg.id].media_hash] ? (
                      <img
                        src={mediaUrls[mediaIndex[msg.id].media_hash]}
                        alt={mediaIndex[msg.id].file_name || 'image'}
                        className="max-w-[240px] rounded-lg"
                      />
                    ) : (
                      <p className="whitespace-pre-wrap text-sm text-gray-500">
                        Carregando imagem...
                      </p>
                    )
                  ) : (
                    <p className="whitespace-pre-wrap">{msg.content || ''}</p>
                  )}
                  <p
                    className={`text-xs mt-1 ${
                      isSentByMe(msg) ? 'text-primary-100' : 'text-gray-500'
                    }`}
                  >
                    {formatTime(msg.created_at)}
                  </p>
                  {(reactions[msg.id]?.length ?? 0) > 0 && (
                    <div className="flex flex-wrap gap-1 mt-1">
                      {Object.entries(
                        (reactions[msg.id] ?? []).reduce<Record<string, number>>(
                          (acc, r) => ({ ...acc, [r.emoji]: (acc[r.emoji] ?? 0) + 1 }),
                          {}
                        )
                      ).map(([emoji, count]) => (
                        <button
                          key={emoji}
                          onClick={() => toggleReaction(msg.id, emoji)}
                          className="text-xs bg-white bg-opacity-70 rounded-full px-2 py-0.5 border border-gray-200"
                        >
                          {emoji} {count > 1 ? count : ''}
                        </button>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            ))}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Forward modal (UX-02) */}
      {forwardMessageId && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-2xl shadow-2xl p-6 w-full max-w-md mx-4 max-h-[70vh] flex flex-col">
            <h2 className="text-xl font-bold text-gray-900 mb-4">Encaminhar para...</h2>
            <div className="flex-1 overflow-y-auto divide-y divide-gray-100">
              {forwardTargets.length === 0 && (
                <p className="text-sm text-gray-500 py-4">Nenhuma outra conversa disponível.</p>
              )}
              {forwardTargets.map((t) => (
                <button
                  key={t.peer_id}
                  onClick={() => handleForward(t.peer_id)}
                  className="w-full text-left px-2 py-3 hover:bg-gray-50"
                >
                  <p className="text-sm font-semibold text-gray-900 truncate">
                    {t.display_name || `${t.peer_id.slice(0, 20)}...`}
                  </p>
                </button>
              ))}
            </div>
            <button
              onClick={() => setForwardMessageId(null)}
              className="btn-secondary mt-4"
            >
              Cancelar
            </button>
          </div>
        </div>
      )}

      {/* Input */}
      <div className="bg-white border-t border-gray-200 px-6 py-4">
        <div className="flex items-center space-x-3">
          {/* Anexar arquivo (UX-02) */}
          <button
            onClick={handleAttachFile}
            disabled={isSending}
            className="text-gray-500 hover:text-gray-700 disabled:opacity-50"
            title="Anexar arquivo"
          >
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M15.172 7l-6.586 6.586a2 2 0 102.828 2.828l6.414-6.586a4 4 0 00-5.656-5.656l-6.415 6.585a6 6 0 108.486 8.486L20.5 13"
              />
            </svg>
          </button>
          <input
            type="text"
            value={newMessage}
            onChange={(e) => setNewMessage(e.target.value)}
            onKeyPress={(e) => e.key === 'Enter' && !e.shiftKey && handleSend()}
            placeholder="Type a message..."
            className="input-base flex-1"
            disabled={isSending}
          />
          <button
            onClick={handleSend}
            disabled={!newMessage.trim() || isSending}
            className="btn-primary px-6 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSending ? (
              <div className="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
            ) : (
              <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
                />
              </svg>
            )}
          </button>
        </div>
      </div>
    </div>
  )
}
