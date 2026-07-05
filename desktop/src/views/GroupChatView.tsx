import { useEffect, useState, useRef } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { formatMessageTime } from '../utils/format'

interface Group {
  id: string
  name: string
  description: string | null
  member_count: number
  is_admin: boolean
  created_at: number
}

interface GroupMessage {
  message_id: string
  sender_peer_id: string
  content: string
  created_at: number
  is_own_message: boolean
}

export default function GroupChatView() {
  const { groupId } = useParams<{ groupId: string }>()
  const [group, setGroup] = useState<Group | null>(null)
  const [messages, setMessages] = useState<GroupMessage[]>([])
  const [messageInput, setMessageInput] = useState('')
  const [isSending, setIsSending] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [showGroupInfo, setShowGroupInfo] = useState(false)
  const [groupMembers, setGroupMembers] = useState<string[]>([])
  const [errorMessage, setErrorMessage] = useState<string | null>(null)
  // Ref (não state) para evitar closure obsoleta no setInterval: o intervalo
  // captura o loadMessages criado quando o peer id ainda estava vazio
  const localPeerIdRef = useRef<string>('')
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const navigate = useNavigate()

  useEffect(() => {
    if (!groupId) return

    loadLocalPeerId()
    loadGroup()
    loadMessages()

    // Auto-refresh messages every 3 seconds
    const interval = setInterval(loadMessages, 3000)
    return () => clearInterval(interval)
  }, [groupId])

  useEffect(() => {
    scrollToBottom()
  }, [messages])

  // Carregar membros quando o modal de info abre
  useEffect(() => {
    if (!showGroupInfo || !groupId) return
    invoke<string[]>('get_group_members', { groupId })
      .then(setGroupMembers)
      .catch((error) => console.error('Failed to load group members:', error))
  }, [showGroupInfo, groupId])

  const handleLeaveGroup = async () => {
    if (!groupId) return
    if (!window.confirm('Sair do grupo?')) return

    try {
      await invoke('leave_group', { groupId })
      navigate('/groups')
    } catch (error) {
      console.error('Failed to leave group:', error)
      setErrorMessage('Failed to leave group: ' + String(error))
    }
  }

  const loadGroup = async () => {
    try {
      const groups = await invoke<Group[]>('get_groups')
      const foundGroup = groups.find(g => g.id === groupId)

      if (foundGroup) {
        setGroup(foundGroup)
      } else {
        setErrorMessage('Group not found')
      }
    } catch (error) {
      console.error('Failed to load group:', error)
      setErrorMessage('Failed to load group: ' + String(error))
    }
  }

  const loadLocalPeerId = async () => {
    try {
      const peerId = await invoke<string>('get_local_peer_id')
      localPeerIdRef.current = peerId
      if (groupId) {
        await loadMessages()
      }
    } catch (error) {
      console.error('Failed to load local peer id:', error)
    }
  }

  const loadMessages = async () => {
    try {
      const fetchedMessages = await invoke<Array<{
        message_id: string
        sender_peer_id: string
        content_plaintext?: string | null
        created_at: number
      }>>('get_group_messages', { groupId })

      const mapped = fetchedMessages.map((msg) => ({
        message_id: msg.message_id,
        sender_peer_id: msg.sender_peer_id,
        content: msg.content_plaintext ?? '',
        created_at: msg.created_at,
        is_own_message: msg.sender_peer_id === localPeerIdRef.current,
      }))

      setMessages(mapped.reverse())
    } catch (error) {
      console.error('Failed to load messages:', error)
    } finally {
      setIsLoading(false)
    }
  }

  const sendMessage = async () => {
    const content = messageInput.trim()
    if (!content || !groupId) return

    setMessageInput('')
    setIsSending(true)

    try {
      await invoke('send_group_message', { groupId, content })
      await loadMessages()
    } catch (error) {
      console.error('Failed to send message:', error)
      setErrorMessage('Failed to send message: ' + String(error))
    } finally {
      setIsSending(false)
    }
  }

  const handleAddMember = async () => {
    if (!groupId) return

    const peerId = window.prompt('Peer ID do novo membro')
    if (!peerId || !peerId.trim()) {
      return
    }

    try {
      // O core envia invite + sender keys automaticamente (protocolo in-band)
      await invoke('add_group_member', { groupId, peerId: peerId.trim() })
      setErrorMessage(null)
    } catch (error) {
      console.error('Failed to add member:', error)
      setErrorMessage('Failed to add member: ' + String(error))
    }
  }

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }


  return (
    <div className="flex flex-col h-screen bg-gray-100">
      {/* Header */}
      <div className="bg-white border-b border-gray-200 px-6 py-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <button
              onClick={() => navigate('/groups')}
              className="text-gray-600 hover:text-gray-900 transition-colors"
            >
              <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
              </svg>
            </button>

            <div>
              <h1 className="text-xl font-bold text-gray-900">
                {group?.name || 'Loading...'}
              </h1>
              <p className="text-sm text-gray-500">
                {group ? `${group.member_count} ${group.member_count === 1 ? 'member' : 'members'}` : ''}
              </p>
            </div>
          </div>

          <button
            onClick={() => setShowGroupInfo(true)}
            className="btn-secondary text-sm"
          >
            Group Info
          </button>
        </div>
      </div>

      {/* Messages Area */}
      <div className="flex-1 overflow-y-auto p-6 space-y-4">
        {isLoading ? (
          <div className="flex items-center justify-center h-full">
            <div className="animate-spin rounded-full h-12 w-12 border-b-4 border-primary-500"></div>
          </div>
        ) : errorMessage ? (
          <div className="flex items-center justify-center h-full">
            <div className="bg-red-50 border border-red-200 rounded-lg p-4 max-w-md">
              <p className="text-sm text-red-800">{errorMessage}</p>
              <button
                onClick={() => setErrorMessage(null)}
                className="mt-2 text-sm text-red-600 hover:text-red-800"
              >
                Dismiss
              </button>
            </div>
          </div>
        ) : messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <svg className="w-16 h-16 text-gray-300 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
            </svg>
            <h2 className="text-lg font-semibold text-gray-900 mb-2">No messages yet</h2>
            <p className="text-gray-600">Send the first message!</p>
          </div>
        ) : (
          <>
            {messages.map((message) => (
              <div
                key={message.message_id}
                className={`flex ${message.is_own_message ? 'justify-end' : 'justify-start'}`}
              >
                <div
                  className={`max-w-xs lg:max-w-md xl:max-w-lg ${
                    message.is_own_message
                      ? 'bg-primary-500 text-white'
                      : 'bg-white text-gray-900'
                  } rounded-2xl px-4 py-2 shadow-sm`}
                >
                  {/* Sender name (only for other people's messages) */}
                  {!message.is_own_message && (
                    <p className="text-xs font-semibold text-blue-600 mb-1">
                      {message.sender_peer_id.substring(0, 8)}
                    </p>
                  )}

                  <p className="text-sm whitespace-pre-wrap break-words">
                    {message.content}
                  </p>

                  <p
                    className={`text-xs mt-1 ${
                      message.is_own_message ? 'text-primary-100' : 'text-gray-500'
                    }`}
                  >
                    {formatMessageTime(message.created_at)}
                  </p>
                </div>
              </div>
            ))}
            <div ref={messagesEndRef} />
          </>
        )}
      </div>

      {/* Message Input */}
      <div className="bg-white border-t border-gray-200 px-6 py-4">
        <div className="flex items-end space-x-3">
          <textarea
            value={messageInput}
            onChange={(e) => setMessageInput(e.target.value)}
            onKeyPress={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault()
                sendMessage()
              }
            }}
            placeholder="Type a message..."
            className="flex-1 resize-none rounded-2xl border border-gray-300 px-4 py-2 focus:outline-none focus:ring-2 focus:ring-primary-500 focus:border-transparent max-h-32"
            rows={1}
            disabled={isSending}
          />
          <button
            onClick={sendMessage}
            disabled={!messageInput.trim() || isSending}
            className="btn-primary rounded-full w-10 h-10 flex items-center justify-center disabled:opacity-50 disabled:cursor-not-allowed flex-shrink-0"
          >
            {isSending ? (
              <div className="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
            ) : (
              <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                <path d="M10.894 2.553a1 1 0 00-1.788 0l-7 14a1 1 0 001.169 1.409l5-1.429A1 1 0 009 15.571V11a1 1 0 112 0v4.571a1 1 0 00.725.962l5 1.428a1 1 0 001.17-1.408l-7-14z"></path>
              </svg>
            )}
          </button>
        </div>
      </div>

      {/* Group Info Modal (placeholder) */}
      {showGroupInfo && group && (
        <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
          <div className="bg-white rounded-2xl shadow-2xl p-6 w-full max-w-md mx-4 max-h-[80vh] overflow-y-auto">
            <div className="flex items-center justify-between mb-6">
              <h2 className="text-xl font-bold text-gray-900">Group Info</h2>
              <button
                onClick={() => setShowGroupInfo(false)}
                className="text-gray-500 hover:text-gray-700"
              >
                <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            <div className="space-y-6">
              {/* Group Icon */}
              <div className="flex flex-col items-center">
                <div className="w-20 h-20 rounded-full bg-blue-100 flex items-center justify-center mb-4">
                  <svg className="w-10 h-10 text-blue-600" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M13 6a3 3 0 11-6 0 3 3 0 016 0zM18 8a2 2 0 11-4 0 2 2 0 014 0zM14 15a4 4 0 00-8 0v3h8v-3zM6 8a2 2 0 11-4 0 2 2 0 014 0zM16 18v-3a5.972 5.972 0 00-.75-2.906A3.005 3.005 0 0119 15v3h-3zM4.75 12.094A5.973 5.973 0 004 15v3H1v-3a3 3 0 013.75-2.906z"></path>
                  </svg>
                </div>
                <h3 className="text-lg font-semibold text-gray-900">{group.name}</h3>
                {group.is_admin && (
                  <span className="mt-2 inline-flex items-center px-3 py-1 rounded-full text-sm font-medium bg-blue-100 text-blue-800">
                    Administrator
                  </span>
                )}
              </div>

              {/* Description */}
              {group.description && (
                <div>
                  <h4 className="text-sm font-medium text-gray-700 mb-2">Description</h4>
                  <p className="text-sm text-gray-600">{group.description}</p>
                </div>
              )}

              {/* Stats */}
              <div className="grid grid-cols-2 gap-4">
                <div className="bg-gray-50 rounded-lg p-3">
                  <p className="text-xs text-gray-500 mb-1">Members</p>
                  <p className="text-lg font-semibold text-gray-900">{group.member_count}</p>
                </div>
                <div className="bg-gray-50 rounded-lg p-3">
                  <p className="text-xs text-gray-500 mb-1">Created</p>
                  <p className="text-sm font-semibold text-gray-900">
                    {new Date(group.created_at * 1000).toLocaleDateString()}
                  </p>
                </div>
              </div>

              {/* Member list */}
              {groupMembers.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-gray-700 mb-2">Membros</h4>
                  <ul className="space-y-1 max-h-40 overflow-y-auto">
                    {groupMembers.map((member) => (
                      <li
                        key={member}
                        className="text-xs font-mono text-gray-600 bg-gray-50 rounded px-2 py-1 truncate"
                        title={member}
                      >
                        {member}
                        {member === localPeerIdRef.current && (
                          <span className="ml-2 text-blue-600">(você)</span>
                        )}
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Actions */}
              <div className="space-y-2">
                {group.is_admin && (
                  <button className="w-full btn-secondary text-sm" onClick={handleAddMember}>
                    Add Member
                  </button>
                )}
                <button
                  className="w-full btn-secondary text-sm text-red-600 hover:bg-red-50"
                  onClick={handleLeaveGroup}
                >
                  Leave Group
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
