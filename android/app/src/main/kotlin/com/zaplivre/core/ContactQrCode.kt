package com.zaplivre.core

import org.json.JSONObject

/**
 * Conteúdo de um QR de contato, nos formatos que os apps geram:
 * - JSON versionado do iOS: `{"v":1,"peerId":…,"multiaddr":…,"prekeyBundle":…}`
 * - convite do Android e QRs antigos: `peerId@multiaddr`
 * - só o peer ID
 */
data class ContactQrCode(
    val peerId: String,
    val multiaddr: String?,
    val prekeyBundle: String?,
) {
    companion object {
        private val BASE58 = Regex("^[1-9A-HJ-NP-Za-km-z]+$")

        /**
         * Peer ID libp2p Ed25519: "12D3KooW" + base58, 52 caracteres. IDs "Qm…"
         * (RSA) não carregam a chave Ed25519 que o protocolo usa para verificar
         * o bundle do contato.
         */
        fun isValidPeerId(value: String): Boolean =
            value.length == 52 && value.startsWith("12D3KooW") && BASE58.matches(value)

        /** null quando o conteúdo não é um contato ZapLivre válido. */
        fun parse(raw: String): ContactQrCode? {
            val text = raw.trim()
            val parsed = if (text.startsWith("{")) {
                runCatching {
                    val json = JSONObject(text)
                    if (json.optInt("v") != 1) return null
                    ContactQrCode(
                        peerId = json.getString("peerId"),
                        multiaddr = json.optString("multiaddr").takeIf { it.isNotBlank() },
                        prekeyBundle = json.optString("prekeyBundle").takeIf { it.isNotBlank() },
                    )
                }.getOrNull() ?: return null
            } else {
                val (peer, addr) = text.split("@", limit = 2).let { it[0] to it.getOrNull(1) }
                ContactQrCode(peer, addr?.takeIf { it.isNotBlank() }, null)
            }
            if (!isValidPeerId(parsed.peerId)) return null
            val addr = parsed.multiaddr
            if (addr != null && !(addr.startsWith("/ip4/") || addr.startsWith("/ip6/") ||
                    addr.startsWith("/dns4/") || addr.startsWith("/dns6/"))
            ) {
                return null
            }
            return parsed
        }
    }
}
