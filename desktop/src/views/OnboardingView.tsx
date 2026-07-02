import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { homeDir } from '@tauri-apps/api/path'

interface OnboardingViewProps {
  localPeerId: string | null
}

export default function OnboardingView({ localPeerId }: OnboardingViewProps) {
  const navigate = useNavigate()
  const [showRestore, setShowRestore] = useState(false)
  const [restoreText, setRestoreText] = useState('')
  const [restoreError, setRestoreError] = useState<string | null>(null)
  const [isRestoring, setIsRestoring] = useState(false)

  const handleGetStarted = () => {
    navigate('/conversations')
  }

  // DSK-09: restaurar backup - salva no keychain e o app REINICIA sozinho
  // com a identidade importada
  const handleRestore = async () => {
    const backup = restoreText.trim()
    if (!backup) return
    setIsRestoring(true)
    setRestoreError(null)
    try {
      const home = await homeDir()
      await invoke('import_identity_backup', {
        backup,
        dataDir: `${home}/.mepassa`,
      })
      // import_identity_backup reinicia o app; nada mais a fazer aqui
    } catch (error) {
      setRestoreError(String(error))
      setIsRestoring(false)
    }
  }

  return (
    <div className="flex items-center justify-center h-screen bg-gradient-to-br from-primary-50 to-primary-100">
      <div className="max-w-md w-full bg-white rounded-2xl shadow-2xl p-8">
        <div className="text-center">
          {/* Logo */}
          <div className="mb-6">
            <div className="w-20 h-20 bg-primary-500 rounded-full mx-auto flex items-center justify-center">
              <svg
                className="w-12 h-12 text-white"
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
            </div>
          </div>

          {/* Title */}
          <h1 className="text-3xl font-bold text-gray-900 mb-2">Welcome to MePassa</h1>
          <p className="text-gray-600 mb-6">
            Hybrid P2P messaging with E2E encryption
          </p>

          {/* Peer ID */}
          {localPeerId && (
            <div className="bg-gray-50 rounded-lg p-4 mb-6">
              <p className="text-xs text-gray-500 mb-1 uppercase font-semibold">Your Peer ID</p>
              <p className="text-sm text-gray-900 font-mono break-all">{localPeerId}</p>
            </div>
          )}

          {/* Features */}
          <div className="text-left mb-8 space-y-3">
            <div className="flex items-start">
              <svg
                className="w-5 h-5 text-primary-500 mr-3 mt-0.5 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              <div>
                <p className="font-semibold text-gray-900">80% P2P Direct</p>
                <p className="text-sm text-gray-600">Maximum privacy, zero server cost</p>
              </div>
            </div>

            <div className="flex items-start">
              <svg
                className="w-5 h-5 text-primary-500 mr-3 mt-0.5 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              <div>
                <p className="font-semibold text-gray-900">E2E Encrypted</p>
                <p className="text-sm text-gray-600">Signal Protocol encryption</p>
              </div>
            </div>

            <div className="flex items-start">
              <svg
                className="w-5 h-5 text-primary-500 mr-3 mt-0.5 flex-shrink-0"
                fill="currentColor"
                viewBox="0 0 20 20"
              >
                <path
                  fillRule="evenodd"
                  d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
                  clipRule="evenodd"
                />
              </svg>
              <div>
                <p className="font-semibold text-gray-900">Always Works</p>
                <p className="text-sm text-gray-600">TURN relay + Store & Forward fallback</p>
              </div>
            </div>
          </div>

          {/* Get Started Button - desabilitado enquanto o client não inicializou */}
          <button
            onClick={handleGetStarted}
            disabled={!localPeerId}
            className="btn-primary w-full disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {localPeerId ? 'Get Started' : 'Initializing...'}
          </button>

          {/* Restaurar backup (DSK-09) */}
          {!showRestore ? (
            <button
              onClick={() => setShowRestore(true)}
              className="btn-secondary w-full mt-3"
            >
              Restaurar backup de identidade
            </button>
          ) : (
            <div className="mt-4 text-left">
              <p className="text-sm text-gray-600 mb-2">
                Cole o backup Base64 exportado em outro dispositivo. O app será reiniciado com a
                identidade restaurada (a identidade atual desta máquina será substituída).
              </p>
              <textarea
                value={restoreText}
                onChange={(e) => setRestoreText(e.target.value)}
                placeholder="Backup Base64..."
                className="w-full h-24 text-xs font-mono border border-gray-300 rounded-lg p-3 resize-none"
              />
              {restoreError && <p className="text-sm text-red-600 mt-2">{restoreError}</p>}
              <div className="flex gap-3 mt-3">
                <button onClick={() => setShowRestore(false)} className="btn-secondary flex-1">
                  Cancelar
                </button>
                <button
                  onClick={handleRestore}
                  disabled={!restoreText.trim() || isRestoring}
                  className="btn-primary flex-1 disabled:opacity-50"
                >
                  {isRestoring ? 'Restaurando...' : 'Restaurar e reiniciar'}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
