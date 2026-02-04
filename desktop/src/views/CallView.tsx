import { useEffect, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { useVoipState } from '../state/voipState'
import '../styles/CallView.css'

interface CallViewProps {
  localPeerId: string | null
}

export default function CallView({ localPeerId: _localPeerId }: CallViewProps) {
  const { callId, remotePeerId } = useParams<{ callId: string; remotePeerId: string }>()
  const navigate = useNavigate()

  const { voipState } = useVoipState()
  const [callDuration, setCallDuration] = useState(0)
  const [isCallActive, setIsCallActive] = useState(true)

  const callState = callId ? voipState[callId] : undefined
  const isMuted = callState?.isMuted ?? false
  const isSpeakerOn = callState?.isSpeakerOn ?? false
  const cameraSwitchCount = callState?.cameraSwitchCount ?? 0

  // Timer for call duration
  useEffect(() => {
    if (!isCallActive) return

    const interval = setInterval(() => {
      setCallDuration(prev => prev + 1)
    }, 1000)

    return () => clearInterval(interval)
  }, [isCallActive])

  const formatDuration = (seconds: number): string => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`
  }

  const handleMuteToggle = async () => {
    if (!callId) return

    try {
      await invoke('toggle_mute', { callId })
    } catch (error) {
      console.error('Failed to toggle mute:', error)
    }
  }

  const handleSpeakerToggle = async () => {
    if (!callId) return

    try {
      await invoke('toggle_speakerphone', { callId })
    } catch (error) {
      console.error('Failed to toggle speakerphone:', error)
    }
  }

  const handleCameraSwitch = async () => {
    if (!callId) return

    try {
      await invoke('switch_camera', { callId })
    } catch (error) {
      console.error('Failed to switch camera:', error)
    }
  }

  const handleHangup = async () => {
    if (!callId) return

    try {
      await invoke('hangup_call', { callId })
      setIsCallActive(false)
      navigate('/conversations')
    } catch (error) {
      console.error('Failed to hangup call:', error)
      // Navigate anyway
      navigate('/conversations')
    }
  }

  return (
    <div className="call-view">
      <div className="call-container">
        {/* Header: Peer Info */}
        <div className="call-header">
          <div className="peer-avatar">
            <svg
              width="120"
              height="120"
              viewBox="0 0 120 120"
              fill="none"
              xmlns="http://www.w3.org/2000/svg"
            >
              <circle cx="60" cy="60" r="60" fill="#3b82f6" />
              <path
                d="M60 60c11.046 0 20-8.954 20-20s-8.954-20-20-20-20 8.954-20 20 8.954 20 20 20zm0 10c-13.314 0-40 6.686-40 20v10h80v-10c0-13.314-26.686-20-40-20z"
                fill="white"
              />
            </svg>
          </div>

          <h2 className="peer-name">
            {remotePeerId ? remotePeerId.substring(0, 16) + '...' : 'Unknown'}
          </h2>

          <p className="call-status">
            Chamada em andamento
            {isMuted && ' · 🔇 Muted'}
            {isSpeakerOn && ' · 🔊 Speaker'}
          </p>
          {cameraSwitchCount > 0 && (
            <p className="call-status">📸 Troca de câmera: {cameraSwitchCount}</p>
          )}

          <div className="call-timer">{formatDuration(callDuration)}</div>
        </div>

        {/* Controls */}
        <div className="call-controls">
          {/* Mute Button */}
          <button
            onClick={handleMuteToggle}
            className={`control-btn ${isMuted ? 'muted' : ''}`}
            title={isMuted ? 'Unmute' : 'Mute'}
          >
            {isMuted ? (
              <svg width="32" height="32" viewBox="0 0 24 24" fill="currentColor">
                <path d="M19 11h-1.7c0 .74-.16 1.43-.43 2.05l1.23 1.23c.56-.98.9-2.09.9-3.28zm-4.02.17c0-.06.02-.11.02-.17V5c0-1.66-1.34-3-3-3S9 3.34 9 5v.18l5.98 5.99zM4.27 3L3 4.27l6.01 6.01V11c0 1.66 1.33 3 2.99 3 .22 0 .44-.03.65-.08l1.66 1.66c-.71.33-1.5.52-2.31.52-2.76 0-5.3-2.1-5.3-5.1H5c0 3.41 2.72 6.23 6 6.72V21h2v-3.28c.91-.13 1.77-.45 2.54-.9L19.73 21 21 19.73 4.27 3z" />
              </svg>
            ) : (
              <svg width="32" height="32" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3zm5.91-3c-.49 0-.9.36-.98.85C16.52 14.2 14.47 16 12 16s-4.52-1.8-4.93-4.15c-.08-.49-.49-.85-.98-.85-.61 0-1.09.54-1 1.14.49 3 2.89 5.35 5.91 5.78V21h2v-3.08c3.02-.43 5.42-2.78 5.91-5.78.1-.6-.39-1.14-1-1.14z" />
              </svg>
            )}
          </button>

          {/* Hangup Button */}
          <button
            onClick={handleHangup}
            className="control-btn hangup"
            title="Hangup"
          >
            <svg width="40" height="40" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 9c-1.6 0-3.15.25-4.6.72v3.1c0 .39-.23.74-.56.9-.98.49-1.87 1.12-2.66 1.85-.18.18-.43.28-.7.28-.28 0-.53-.11-.71-.29L.29 13.08c-.18-.17-.29-.42-.29-.7 0-.28.11-.53.29-.71C3.34 8.78 7.46 7 12 7s8.66 1.78 11.71 4.67c.18.18.29.43.29.71 0 .28-.11.53-.29.71l-2.48 2.48c-.18.18-.43.29-.71.29-.27 0-.52-.11-.7-.28-.79-.74-1.69-1.36-2.67-1.85-.33-.16-.56-.5-.56-.9v-3.1C15.15 9.25 13.6 9 12 9z" />
            </svg>
          </button>

          {/* Speaker Button */}
          <button
            onClick={handleSpeakerToggle}
            className={`control-btn ${isSpeakerOn ? 'speaker-on' : ''}`}
            title={isSpeakerOn ? 'Speaker Off' : 'Speaker On'}
          >
            {isSpeakerOn ? (
              <svg width="32" height="32" viewBox="0 0 24 24" fill="currentColor">
                <path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02zM14 3.23v2.06c2.89.86 5 3.54 5 6.71s-2.11 5.85-5 6.71v2.06c4.01-.91 7-4.49 7-8.77s-2.99-7.86-7-8.77z" />
              </svg>
            ) : (
              <svg width="32" height="32" viewBox="0 0 24 24" fill="currentColor">
                <path d="M7 9v6h4l5 5V4l-5 5H7z" />
              </svg>
            )}
          </button>

          {/* Camera Switch Button */}
          <button
            onClick={handleCameraSwitch}
            className="control-btn"
            title="Switch Camera"
          >
            <svg width="32" height="32" viewBox="0 0 24 24" fill="currentColor">
              <path d="M20 4h-3.17l-1.84-2H9.01L7.17 4H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V6c0-1.1-.9-2-2-2zm0 14H4V6h4.05l1.83-2h4.24l1.83 2H20v12zm-8-1.5c2.48 0 4.5-2.02 4.5-4.5S14.48 7.5 12 7.5 7.5 9.52 7.5 12s2.02 4.5 4.5 4.5zm0-7c1.38 0 2.5 1.12 2.5 2.5S13.38 14.5 12 14.5 9.5 13.38 9.5 12 10.62 9.5 12 9.5z" />
            </svg>
          </button>
        </div>
      </div>
    </div>
  )
}
