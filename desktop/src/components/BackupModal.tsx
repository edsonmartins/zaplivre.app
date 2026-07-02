import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { X, Copy, Check } from 'lucide-react'

interface BackupModalProps {
  onClose: () => void
}

/**
 * DSK-09: exporta o backup Base64 da identidade (keychain do sistema).
 * Sem este backup, perder a máquina = perder o peer ID.
 */
export default function BackupModal({ onClose }: BackupModalProps) {
  const [backup, setBackup] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [copied, setCopied] = useState(false)

  useEffect(() => {
    invoke<string>('export_identity_backup')
      .then(setBackup)
      .catch((e) => setError(String(e)))
  }, [])

  const handleCopy = async () => {
    if (!backup) return
    await navigator.clipboard.writeText(backup)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-white rounded-2xl shadow-2xl p-6 w-full max-w-md mx-4">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-gray-900">Backup da identidade</h2>
          <button onClick={onClose} className="text-gray-400 hover:text-gray-600" title="Fechar">
            <X className="w-5 h-5" />
          </button>
        </div>

        {error ? (
          <p className="text-sm text-red-600">{error}</p>
        ) : (
          <>
            <p className="text-sm text-gray-600 mb-3">
              Guarde este código em local seguro. Ele restaura seu peer ID em outra máquina — quem
              tiver este código controla sua identidade.
            </p>
            <textarea
              readOnly
              value={backup ?? 'Carregando...'}
              className="w-full h-32 text-xs font-mono border border-gray-300 rounded-lg p-3 bg-gray-50 resize-none"
            />
            <button
              onClick={handleCopy}
              disabled={!backup}
              className="btn-primary w-full mt-3 flex items-center justify-center gap-2"
            >
              {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
              {copied ? 'Copiado' : 'Copiar backup'}
            </button>
          </>
        )}
      </div>
    </div>
  )
}
