import { useEffect, useMemo, useRef, useState } from 'react'
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

interface VideoFrameRgbaEvent {
  call_id: string
  width: number
  height: number
  size: number
  data_b64: string
}

type DecoderState = {
  decoder: VideoDecoder | null
  configured: boolean
  codec: string | null
  config: ArrayBuffer | null
}

function toHexByte(value: number) {
  return value.toString(16).padStart(2, '0')
}

function splitAnnexBNalus(data: Uint8Array): Uint8Array[] {
  const nalus: Uint8Array[] = []
  let start = -1
  let i = 0
  while (i + 3 < data.length) {
    const isStart4 =
      data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 0 && data[i + 3] === 1
    const isStart3 = data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 1
    if (isStart4 || isStart3) {
      const startCodeLen = isStart4 ? 4 : 3
      if (start >= 0 && start < i) {
        nalus.push(data.slice(start, i))
      }
      start = i + startCodeLen
      i += startCodeLen
      continue
    }
    i += 1
  }
  if (start >= 0 && start < data.length) {
    nalus.push(data.slice(start))
  }
  if (!nalus.length && data.length) {
    nalus.push(data)
  }
  return nalus
}

function buildAvcDecoderConfig(sps: Uint8Array, pps: Uint8Array) {
  const profileIdc = sps[1] ?? 0
  const profileCompat = sps[2] ?? 0
  const levelIdc = sps[3] ?? 0
  const codec = `avc1.${toHexByte(profileIdc)}${toHexByte(profileCompat)}${toHexByte(levelIdc)}`

  const avcc = new Uint8Array(
    7 + 2 + sps.length + 1 + 2 + pps.length
  )
  let offset = 0
  avcc[offset++] = 1
  avcc[offset++] = profileIdc
  avcc[offset++] = profileCompat
  avcc[offset++] = levelIdc
  avcc[offset++] = 0xfc | 3
  avcc[offset++] = 0xe0 | 1
  avcc[offset++] = (sps.length >> 8) & 0xff
  avcc[offset++] = sps.length & 0xff
  avcc.set(sps, offset)
  offset += sps.length
  avcc[offset++] = 1
  avcc[offset++] = (pps.length >> 8) & 0xff
  avcc[offset++] = pps.length & 0xff
  avcc.set(pps, offset)

  return { codec, config: avcc.buffer }
}

function nalusToAvccSample(nalus: Uint8Array[]) {
  const total = nalus.reduce((sum, nalu) => sum + 4 + nalu.length, 0)
  const out = new Uint8Array(total)
  let offset = 0
  for (const nalu of nalus) {
    const len = nalu.length
    out[offset++] = (len >> 24) & 0xff
    out[offset++] = (len >> 16) & 0xff
    out[offset++] = (len >> 8) & 0xff
    out[offset++] = len & 0xff
    out.set(nalu, offset)
    offset += len
  }
  return out
}

export default function VideoCallView() {
  const { callId, remotePeerId } = useParams<{ callId: string; remotePeerId: string }>()
  const navigate = useNavigate()

  const [videoEnabled, setVideoEnabled] = useState(true)
  const [frameCount, setFrameCount] = useState(0)
  const [lastFrameSize, setLastFrameSize] = useState(0)
  const [lastResolution, setLastResolution] = useState<string>('0x0')
  const [videoError, setVideoError] = useState<string | null>(null)
  const [useNativeFrames, setUseNativeFrames] = useState(false)
  const remoteCanvasRef = useRef<HTMLCanvasElement | null>(null)
  const decoderRef = useRef<DecoderState>({
    decoder: null,
    configured: false,
    codec: null,
    config: null,
  })

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
    const rgbaListener = listen<VideoFrameRgbaEvent>('voip:video_frame_rgba', event => {
      if (event.payload.call_id !== callId) return
      setUseNativeFrames(true)

      const canvas = remoteCanvasRef.current
      if (!canvas) return
      const ctx = canvas.getContext('2d')
      if (!ctx) return

      const width = event.payload.width
      const height = event.payload.height
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width
        canvas.height = height
      }

      const rgba = Uint8Array.from(atob(event.payload.data_b64), c => c.charCodeAt(0))
      const imageData = new ImageData(new Uint8ClampedArray(rgba), width, height)
      ctx.putImageData(imageData, 0, 0)
    })

    const unlistenPromise = listen<VideoFrameEvent>('voip:video_frame', event => {
      if (event.payload.call_id !== callId) return
      if (useNativeFrames) return
      setFrameCount(prev => prev + 1)
      setLastFrameSize(event.payload.size)
      setLastResolution(`${event.payload.width}x${event.payload.height}`)

      if (!('VideoDecoder' in window)) {
        setVideoError('VideoDecoder nao esta disponivel neste ambiente.')
        return
      }

      const decoderState = decoderRef.current
      if (!decoderState.decoder) {
        const decoder = new VideoDecoder({
          output: frame => {
            const canvas = remoteCanvasRef.current
            if (!canvas) {
              frame.close()
              return
            }
            const ctx = canvas.getContext('2d')
            if (!ctx) {
              frame.close()
              return
            }

            const width = frame.displayWidth || frame.codedWidth
            const height = frame.displayHeight || frame.codedHeight
            if (canvas.width !== width || canvas.height !== height) {
              canvas.width = width
              canvas.height = height
            }

            ctx.drawImage(frame, 0, 0, canvas.width, canvas.height)
            frame.close()
          },
          error: err => {
            setVideoError(`Decoder error: ${err.message}`)
          },
        })
        decoderState.decoder = decoder
      }

      const frameBytes = Uint8Array.from(atob(event.payload.data_b64), c => c.charCodeAt(0))
      const nalus = splitAnnexBNalus(frameBytes)
      if (!nalus.length) return

      let sps: Uint8Array | null = null
      let pps: Uint8Array | null = null
      let isKey = false
      for (const nalu of nalus) {
        const naluType = nalu[0] & 0x1f
        if (naluType === 7) sps = nalu
        if (naluType === 8) pps = nalu
        if (naluType === 5) isKey = true
      }

      if (!decoderState.configured && sps && pps) {
        const { codec, config } = buildAvcDecoderConfig(sps, pps)
        decoderState.codec = codec
        decoderState.config = config
        decoderState.decoder?.configure({
          codec,
          description: config,
          optimizeForLatency: true,
        })
        decoderState.configured = true
      }

      if (!decoderState.configured) {
        return
      }

      const avccSample = nalusToAvccSample(nalus)
      const chunk = new EncodedVideoChunk({
        type: isKey ? 'key' : 'delta',
        timestamp: Math.floor(performance.now() * 1000),
        data: avccSample,
      })

      try {
        decoderState.decoder?.decode(chunk)
      } catch (err) {
        if (err instanceof Error) {
          setVideoError(`Decode error: ${err.message}`)
        }
      }
    })

    return () => {
      rgbaListener.then(unlisten => unlisten())
      unlistenPromise.then(unlisten => unlisten())
    }
  }, [callId, useNativeFrames])

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
            <canvas ref={remoteCanvasRef} className="video-canvas" />
            <div className="video-overlay">
              <div className="video-label">Remote video</div>
              <div className="video-metrics">
                Frames: {frameCount} · Last: {lastResolution} · {lastFrameSize} bytes
              </div>
              {videoError && <div className="video-error">{videoError}</div>}
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
