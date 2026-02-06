import { useEffect, useMemo, useState } from 'react'
import { useNavigate, useParams } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import '../styles/VideoCallView.css'

interface VideoFrameEvent {
  call_id: string
  width: number
  height: number
  size: number
  data_b64: string
}

export default function VideoCallView() {
  const { callId, remotePeerId } = useParams<{ callId: string; remotePeerId: string }>()
  const navigate = useNavigate()

  const [videoEnabled, setVideoEnabled] = useState(true)
  const [frameCount, setFrameCount] = useState(0)
  const [lastFrameSize, setLastFrameSize] = useState(0)
  const [lastResolution, setLastResolution] = useState<string>('0x0')

  const peerLabel = useMemo(() => {
    if (!remotePeerId) return 'Unknown'
    return `${remotePeerId.substring(0, 16)}...`
  }, [remotePeerId])

  useEffect(() => {
    if (!callId) return

    const setup = async () => {
      try {
        await invoke('register_video_frame_callback')
        await invoke('enable_video', { callId, codec: 'h264' })
      } catch (error) {
        console.error('Failed to enable video:', error)
      }
    }

    setup()
  }, [callId])

  useEffect(() => {
    const unlistenPromise = listen<VideoFrameEvent>('voip:video_frame', event => {
      if (event.payload.call_id !== callId) return
      setFrameCount(prev => prev + 1)
      setLastFrameSize(event.payload.size)
      setLastResolution(`${event.payload.width}x${event.payload.height}`)
    })

    return () => {
      unlistenPromise.then(unlisten => unlisten())
    }
  }, [callId])

  const handleToggleVideo = async () => {
    if (!callId) return
    const next = !videoEnabled
    setVideoEnabled(next)
    try {
      if (next) {
        await invoke('enable_video', { callId, codec: 'h264' })
      } else {
        await invoke('disable_video', { callId })
      }
    } catch (error) {
      console.error('Failed to toggle video:', error)
    }
  }

  const handleHangup = async () => {
    if (!callId) return
    try {
      await invoke('hangup_call', { callId })
    } catch (error) {
      console.error('Failed to hangup call:', error)
    } finally {
      navigate('/conversations')
    }
  }

  return (
    <div className="video-call-view">
      <div className="video-call-container">
        <div className="video-stage">
          <div className="remote-video">
            <div className="video-placeholder">
              <div className="video-label">Remote video</div>
              <div className="video-metrics">
                Frames: {frameCount} · Last: {lastResolution} · {lastFrameSize} bytes
              </div>
            </div>
          </div>
          <div className="local-video">
            <div className="video-placeholder">
              <div className="video-label">Local preview</div>
              <div className="video-metrics">Camera: {videoEnabled ? 'On' : 'Off'}</div>
            </div>
          </div>
        </div>

        <div className="video-call-footer">
          <div className="peer-info">{peerLabel}</div>
          <div className="video-controls">
            <button
              className={`video-btn ${videoEnabled ? 'active' : ''}`}
              onClick={handleToggleVideo}
            >
              {videoEnabled ? 'Video On' : 'Video Off'}
            </button>
            <button className="video-btn hangup" onClick={handleHangup}>
              Hangup
            </button>
          </div>
        </div>
      </div>
    </div>
  )
}
