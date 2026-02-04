import { useEffect, useState } from 'react'
import { Routes, Route, Navigate, useNavigate, useLocation } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { homeDir } from '@tauri-apps/api/path'
import OnboardingView from './views/OnboardingView'
import ConversationsView from './views/ConversationsView'
import ChatView from './views/ChatView'
import CallView from './views/CallView'
import GroupListView from './views/GroupListView'
import GroupChatView from './views/GroupChatView'
import { VoipStateProvider, useVoipState } from './state/voipState'
import './styles/voipToast.css'

type VoipToast = {
  id: string
  message: string
  isClosing?: boolean
}

function App() {
  console.log('🔵 App component mounted')

  const [isInitialized, setIsInitialized] = useState(false)
  const [isLoading, setIsLoading] = useState(true)
  const [localPeerId, setLocalPeerId] = useState<string | null>(null)
  const [errorMessage, setErrorMessage] = useState<string>('')
  const [toasts, setToasts] = useState<VoipToast[]>([])
  const navigate = useNavigate()
  const location = useLocation()

  const { setVoipState } = useVoipState()

  useEffect(() => {
    console.log('🔵 useEffect running - about to call initializeApp')

    const initializeApp = async () => {
      try {
        console.log('🔵 initializeApp STARTED')
        const home = await homeDir()
        const dataDir = `${home}/.mepassa`

        console.log('🔵 Initializing MePassa with data_dir:', dataDir)

        const peerId = await invoke<string>('init_client', { dataDir })
        console.log('🔵 init_client returned peer_id:', peerId)
        setLocalPeerId(peerId)
        setIsInitialized(true)

        // Listen on TCP for incoming connections
        console.log('🔵 Calling listen_on for TCP...')
        try {
          await invoke('listen_on', { multiaddr: '/ip4/0.0.0.0/tcp/0' })
          console.log('✅ Listening on TCP')
        } catch (e) {
          console.warn('⚠️ Failed to listen on TCP:', e)
        }

        // Also listen on QUIC for better NAT traversal
        console.log('🔵 Calling listen_on for QUIC...')
        try {
          await invoke('listen_on', { multiaddr: '/ip4/0.0.0.0/udp/0/quic-v1' })
          console.log('✅ Listening on QUIC')
        } catch (e) {
          console.warn('⚠️ Failed to listen on QUIC:', e)
        }

        // Bootstrap the DHT for address discovery
        try {
          await invoke('bootstrap')
          console.log('🌐 Bootstrapped DHT')
        } catch (e) {
          console.warn('⚠️ Failed to bootstrap DHT:', e)
        }

        console.log('✅ MePassa initialized successfully. Peer ID:', peerId)
      } catch (error) {
        console.error('❌ Failed to initialize MePassa:', error)
        const errorMsg = error instanceof Error ? error.message : String(error)
        setErrorMessage(errorMsg)
        setIsInitialized(false)
      } finally {
        setIsLoading(false)
      }
    }

    initializeApp()

    const unsubs: Array<() => void> = []
    const registerVoipListeners = async () => {
      const mute = await listen<{ call_id: string; is_muted: boolean }>(
        'voip:mute_changed',
        (event) => {
          console.log('🔇 voip:mute_changed', event.payload)
          setVoipState((prev) => ({
            ...prev,
            [event.payload.call_id]: {
              ...(prev[event.payload.call_id] || {}),
              isMuted: event.payload.is_muted,
            },
          }))
        }
      )
      const speaker = await listen<{ call_id: string; enabled: boolean }>(
        'voip:speaker_changed',
        (event) => {
          console.log('🔊 voip:speaker_changed', event.payload)
          setVoipState((prev) => ({
            ...prev,
            [event.payload.call_id]: {
              ...(prev[event.payload.call_id] || {}),
              isSpeakerOn: event.payload.enabled,
            },
          }))
        }
      )
      const camera = await listen<{ call_id: string }>(
        'voip:camera_switch_requested',
        (event) => {
          console.log('📸 voip:camera_switch_requested', event.payload)
          setVoipState((prev) => {
            const previous = prev[event.payload.call_id] || {}
            return {
              ...prev,
              [event.payload.call_id]: {
                ...previous,
                cameraSwitchCount: (previous.cameraSwitchCount || 0) + 1,
                lastCameraSwitchAt: Date.now(),
              },
            }
          })
          invoke('show_notification', {
            title: 'Troca de câmera',
            body: `Chamada ${event.payload.call_id}`,
          }).catch(() => undefined)

          const toastId = `${event.payload.call_id}:${Date.now()}`
          setToasts((prev) => [
            ...prev,
            {
              id: toastId,
              message: `Troca de câmera solicitada (${event.payload.call_id.slice(0, 8)}...)`,
            },
          ])
          setTimeout(() => {
            setToasts((prev) =>
              prev.map((t) => (t.id === toastId ? { ...t, isClosing: true } : t))
            )
            setTimeout(() => {
              setToasts((prev) => prev.filter((t) => t.id !== toastId))
            }, 200)
          }, 6000)
        }
      )
      unsubs.push(mute, speaker, camera)
    }

    registerVoipListeners()

    return () => {
      unsubs.forEach((unsub) => unsub())
    }
  }, [])

  useEffect(() => {
    // Only auto-navigate if on root path or onboarding when should be elsewhere
    const shouldNavigate = location.pathname === '/' ||
                          (location.pathname === '/onboarding' && isInitialized) ||
                          (location.pathname === '/conversations' && !isInitialized)

    if (!isLoading && shouldNavigate) {
      if (isInitialized) {
        navigate('/conversations')
      } else {
        navigate('/onboarding')
      }
    }
  }, [isLoading, isInitialized, navigate, location.pathname])

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen bg-gray-100">
        <div className="text-center">
          <div className="animate-spin rounded-full h-16 w-16 border-b-4 border-primary-500 mx-auto"></div>
          <p className="mt-4 text-gray-600 font-medium">Loading MePassa...</p>
          {errorMessage && (
            <div className="mt-4 p-4 bg-red-100 border border-red-400 text-red-700 rounded">
              <p className="font-bold">Error during initialization:</p>
              <p className="text-sm mt-2">{errorMessage}</p>
            </div>
          )}
        </div>
      </div>
    )
  }

  if (errorMessage && !isInitialized) {
    return (
      <div className="flex items-center justify-center h-screen bg-gray-100">
        <div className="max-w-md w-full bg-white rounded-lg shadow-lg p-6">
          <div className="text-center">
            <div className="text-red-500 text-6xl mb-4">⚠️</div>
            <h2 className="text-2xl font-bold text-gray-900 mb-4">Initialization Failed</h2>
            <div className="p-4 bg-red-50 border border-red-200 rounded text-left">
              <p className="text-sm text-gray-700 break-all">{errorMessage}</p>
            </div>
            <button
              onClick={() => window.location.reload()}
              className="mt-6 px-4 py-2 bg-primary-500 text-white rounded hover:bg-primary-600"
            >
              Retry
            </button>
          </div>
        </div>
      </div>
    )
  }

  return (
    <div className="app-root">
      {toasts.length > 0 && (
        <div className="voip-toast-container">
          {toasts.map((toast) => (
            <div
              key={toast.id}
              className={`voip-toast ${toast.isClosing ? 'is-closing' : ''}`}
            >
              <span>{toast.message}</span>
              <button
                className="voip-toast-close"
                onClick={() => {
                  setToasts((prev) =>
                    prev.map((t) => (t.id === toast.id ? { ...t, isClosing: true } : t))
                  )
                  setTimeout(() => {
                    setToasts((prev) => prev.filter((t) => t.id !== toast.id))
                  }, 200)
                }}
                aria-label="Dismiss"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
      <Routes>
      <Route path="/onboarding" element={<OnboardingView localPeerId={localPeerId} />} />
      <Route path="/conversations" element={<ConversationsView localPeerId={localPeerId} />} />
      <Route path="/chat/:peerId" element={<ChatView localPeerId={localPeerId} />} />
      <Route path="/call/:callId/:remotePeerId" element={<CallView localPeerId={localPeerId} />} />
      <Route path="/groups" element={<GroupListView localPeerId={localPeerId} />} />
      <Route path="/group/:groupId" element={<GroupChatView />} />
      <Route path="*" element={<Navigate to={isInitialized ? "/conversations" : "/onboarding"} replace />} />
      </Routes>
    </div>
  )
}

export default function AppWithProviders() {
  return (
    <VoipStateProvider>
      <App />
    </VoipStateProvider>
  )
}
export { App }
