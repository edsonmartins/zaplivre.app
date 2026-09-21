package com.zaplivre.core

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ContactQrCodeTest {
    private val peer = "12D3KooWJMY3dKygHLtkruLohCshiPENpJscD5XY33GjfcmS4DKK"

    @Test
    fun `convite do Android peerId@multiaddr`() {
        val qr = ContactQrCode.parse("$peer@/ip4/192.168.0.10/tcp/4001")
        assertEquals(peer, qr?.peerId)
        assertEquals("/ip4/192.168.0.10/tcp/4001", qr?.multiaddr)
    }

    @Test
    fun `JSON versionado do iOS`() {
        val qr = ContactQrCode.parse(
            """{"v":1,"peerId":"$peer","multiaddr":"/ip4/10.0.0.2/tcp/4001","prekeyBundle":"{}"}"""
        )
        assertEquals(peer, qr?.peerId)
        assertEquals("/ip4/10.0.0.2/tcp/4001", qr?.multiaddr)
        assertEquals("{}", qr?.prekeyBundle)
    }

    @Test
    fun `so o peer ID`() {
        assertEquals(ContactQrCode(peer, null, null), ContactQrCode.parse(peer))
    }

    @Test
    fun `recusa conteudo que nao e contato`() {
        assertNull(ContactQrCode.parse("https://example.com"))
        assertNull(ContactQrCode.parse("12D3KooWcurto"))
        assertNull(ContactQrCode.parse("$peer@http://evil"))
        assertNull(ContactQrCode.parse("""{"v":2,"peerId":"$peer"}"""))
    }
}
