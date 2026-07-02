import { QRCodeSVG } from 'qrcode.react'
import { X, Copy, Check, RefreshCw } from 'lucide-react'
import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'

interface QRCodeModalProps {
  localPeerId: string
  onClose: () => void
}

export default function QRCodeModal({ localPeerId, onClose }: QRCodeModalProps) {
  const [copied, setCopied] = useState(false)
  const [listeningAddresses, setListeningAddresses] = useState<string[]>([])
  const [isLoading, setIsLoading] = useState(true)

  // Fetch listening addresses on mount
  useEffect(() => {
    const fetchAddresses = async () => {
      try {
        const addresses = await invoke<string[]>('get_listening_addresses')
        console.log('📍 Listening addresses:', addresses)
        setListeningAddresses(addresses)
      } catch (error) {
        console.error('Failed to get listening addresses:', error)
      } finally {
        setIsLoading(false)
      }
    }
    fetchAddresses()
  }, [])

  // Build QR code data: peer_id@address (iOS will parse this)
  // Filter to only include routable addresses (not localhost / wildcard)
  const getRoutableAddress = () => {
    const isWildcard = (addr: string) =>
      addr.includes('/ip4/0.0.0.0/') || addr.includes('/ip6/::/')

    const isLoopback = (addr: string) =>
      addr.includes('/127.0.0.1/') || addr.includes('/::1/')

    const isPrivateV4 = (addr: string) =>
      addr.includes('/ip4/10.') ||
      addr.includes('/ip4/192.168.') ||
      addr.match(/\/ip4\/172\.(1[6-9]|2[0-9]|3[0-1])\./) !== null

    // Prefer non-localhost, non-wildcard TCP addresses (private LAN first)
    const tcpAddrs = listeningAddresses.filter(addr =>
      addr.includes('/tcp/') &&
      !isLoopback(addr) &&
      !isWildcard(addr)
    )
    const privateTcp = tcpAddrs.find(isPrivateV4)
    if (privateTcp) return privateTcp
    if (tcpAddrs.length > 0) return tcpAddrs[0]

    // Fall back to any TCP address (even if wildcard/loopback)
    const anyTcp = listeningAddresses.find(addr => addr.includes('/tcp/'))
    if (anyTcp) return anyTcp

    // Fall back to any address
    return listeningAddresses[0] || ''
  }

  const routeAddr = getRoutableAddress()
  // Format: peerId@multiaddr - iOS will split on @ to get both parts
  const qrCodeData = routeAddr ? `${localPeerId}@${routeAddr}` : localPeerId
  
  useEffect(() => {
    if (isLoading) return
    console.log('📦 QR data:', {
      localPeerId,
      routeAddr,
      qrCodeData,
      listeningAddresses
    })
  }, [isLoading, localPeerId, routeAddr, qrCodeData, listeningAddresses])

  const handleCopyPeerId = async () => {
    try {
      await navigator.clipboard.writeText(localPeerId)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (error) {
      console.error('Failed to copy peer ID:', error)
    }
  }

  const handleRefreshAddresses = async () => {
    setIsLoading(true)
    try {
      const addresses = await invoke<string[]>('get_listening_addresses')
      console.log('📍 Listening addresses (refresh):', addresses)
      setListeningAddresses(addresses)
    } catch (error) {
      console.error('Failed to refresh listening addresses:', error)
    } finally {
      setIsLoading(false)
    }
  }

  // Close on Escape key
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose()
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-50 backdrop-blur-sm"
      onClick={onClose}
      onKeyDown={handleKeyDown}
      tabIndex={-1}
    >
      <div
        className="bg-white rounded-2xl shadow-2xl max-w-md w-full mx-4 max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-200">
          <h2 className="text-xl font-bold text-gray-900">Meu QR Code</h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-full transition-colors"
            aria-label="Fechar"
          >
            <X className="w-5 h-5 text-gray-500" />
          </button>
        </div>

        {/* Content */}
        <div className="px-6 py-8 space-y-6">
          {/* QR Code */}
          <div className="flex justify-center">
            <div className="p-6 bg-white rounded-xl shadow-lg border-2 border-gray-100">
              {isLoading ? (
                <div className="w-[220px] h-[220px] flex items-center justify-center">
                  <RefreshCw className="w-8 h-8 animate-spin text-gray-400" />
                </div>
              ) : (
                <QRCodeSVG
                  value={qrCodeData}
                  size={220}
                  level="M"
                  includeMargin={true}
                />
              )}
            </div>
          </div>

          {/* Listening Address Info */}
          {routeAddr && (
            <div className="text-xs text-gray-500 text-center">
              <p className="font-medium">Endereço de escuta:</p>
              <p className="font-mono break-all">{routeAddr}</p>
            </div>
          )}
          {!routeAddr && !isLoading && (
            <div className="text-xs text-amber-700 text-center bg-amber-50 border border-amber-100 rounded-lg p-2">
              Nenhum endereço roteável encontrado. Clique em “Atualizar” ou tente novamente após alguns segundos.
            </div>
          )}

          {/* Peer ID Display */}
          <div className="space-y-2">
            <p className="text-xs font-medium text-gray-500 text-center">
              Peer ID
            </p>
            <div className="relative">
              <div className="px-4 py-3 bg-gray-50 rounded-lg font-mono text-xs text-gray-700 break-all text-center border border-gray-200">
                {localPeerId}
              </div>
              <button
                onClick={handleCopyPeerId}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-2 hover:bg-gray-200 rounded-lg transition-colors"
                title="Copiar Peer ID"
              >
                {copied ? (
                  <Check className="w-4 h-4 text-green-600" />
                ) : (
                  <Copy className="w-4 h-4 text-gray-600" />
                )}
              </button>
            </div>
            {copied && (
              <p className="text-xs text-green-600 text-center animate-fade-in">
                ✓ Peer ID copiado!
              </p>
            )}
          </div>

          {/* Action Buttons */}
          <div className="space-y-3">
            <button
              onClick={handleRefreshAddresses}
              className="w-full flex items-center justify-center gap-2 px-6 py-3 bg-gray-100 text-gray-700 font-semibold rounded-xl hover:bg-gray-200 transition-colors"
            >
              <RefreshCw className={`w-5 h-5 ${isLoading ? 'animate-spin' : ''}`} />
              Atualizar endereço
            </button>

            <button
              onClick={handleCopyPeerId}
              className="w-full flex items-center justify-center gap-2 px-6 py-3 bg-gray-100 text-gray-700 font-semibold rounded-xl hover:bg-gray-200 transition-colors"
            >
              <Copy className="w-5 h-5" />
              {copied ? 'Copiado!' : 'Copiar Peer ID'}
            </button>
          </div>

          {/* Info Section */}
          <div className="bg-blue-50 border border-blue-100 rounded-xl p-4">
            <h3 className="font-semibold text-blue-900 mb-2 text-sm">
              Como usar o QR Code
            </h3>
            <ul className="text-xs text-blue-800 space-y-1">
              <li>• Compartilhe este QR code com seus contatos</li>
              <li>• Eles podem escaneá-lo no app mobile</li>
              <li>• Ou copie e envie seu Peer ID diretamente</li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  )
}
