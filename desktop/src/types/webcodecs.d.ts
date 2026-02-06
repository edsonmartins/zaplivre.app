type EncodedVideoChunkType = 'key' | 'delta'

interface EncodedVideoChunkInit {
  type: EncodedVideoChunkType
  timestamp: number
  data: BufferSource
}

declare class EncodedVideoChunk {
  constructor(init: EncodedVideoChunkInit)
}

interface VideoDecoderConfig {
  codec: string
  description?: BufferSource
  optimizeForLatency?: boolean
}

interface VideoDecoderInit {
  output: (frame: VideoFrame) => void
  error: (error: DOMException) => void
}

declare class VideoDecoder {
  constructor(init: VideoDecoderInit)
  configure(config: VideoDecoderConfig): void
  decode(chunk: EncodedVideoChunk): void
}

declare class VideoFrame {
  readonly displayWidth: number
  readonly displayHeight: number
  readonly codedWidth: number
  readonly codedHeight: number
  close(): void
}
