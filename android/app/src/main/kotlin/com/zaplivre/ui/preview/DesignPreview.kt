package com.zaplivre.ui.preview

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Chat
import androidx.compose.material.icons.filled.Group
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.Phone
import androidx.compose.material.icons.filled.Photo
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Divider
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationBarItemDefaults
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.unit.dp
import com.zaplivre.ui.components.BubbleStatus
import com.zaplivre.ui.components.ZapAvatar
import com.zaplivre.ui.components.ZapTextBubble
import com.zaplivre.ui.screens.conversations.ConversationUi
import com.zaplivre.ui.screens.conversations.ConversationsContent
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapMetric
import com.zaplivre.ui.theme.ZapType

/**
 * Harness de validação visual do design system (dados mock, sem client P2P).
 * Espelha o -designPreview do iOS. Acionado por `--ez design_preview true` na
 * MainActivity. Permite screenshotar todas as telas mesmo com o client offline.
 */
@Composable
fun DesignPreviewHost() {
    var tab by remember { mutableIntStateOf(0) }
    val tabs = listOf(
        PreviewTab("Conversas", Icons.Filled.Chat),
        PreviewTab("Chat", Icons.Default.List),
        PreviewTab("Ajustes", Icons.Default.Settings),
        PreviewTab("Grupos", Icons.Default.Group),
    )

    Scaffold(
        containerColor = ZapColor.canvas,
        bottomBar = {
            NavigationBar(containerColor = ZapColor.surface) {
                tabs.forEachIndexed { i, t ->
                    NavigationBarItem(
                        selected = tab == i,
                        onClick = { tab = i },
                        icon = { Icon(t.icon, t.label) },
                        label = { Text(t.label) },
                        colors = NavigationBarItemDefaults.colors(
                            selectedIconColor = ZapColor.primary,
                            selectedTextColor = ZapColor.primary,
                            indicatorColor = ZapColor.chatCanvas,
                            unselectedIconColor = ZapColor.slate,
                            unselectedTextColor = ZapColor.slate,
                        ),
                    )
                }
            }
        }
    ) { pv ->
        Box(Modifier.fillMaxSize().padding(pv)) {
            when (tab) {
                0 -> ConversationsContent(
                    rows = MockData.conversations,
                    onSearchClick = {}, onGroupsClick = {}, onSettingsClick = {},
                )
                1 -> ChatPreview()
                2 -> SettingsPreviewContent()
                3 -> GroupsPreviewContent()
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ChatPreview() {
    Scaffold(
        containerColor = ZapColor.chatCanvas,
        topBar = {
            Column {
                TopAppBar(
                    title = {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            ZapAvatar("12D3KooWCarla", "Carla Nogueira", size = ZapMetric.avatarSmall, online = true)
                            Spacer(Modifier.width(10.dp))
                            Column {
                                Text("Carla Nogueira", style = ZapType.rowName, color = ZapColor.ink)
                                Text("online", style = ZapType.caption, color = ZapColor.online)
                            }
                        }
                    },
                    navigationIcon = {
                        IconButton(onClick = {}) { Icon(Icons.Filled.ArrowBack, "Voltar", tint = ZapColor.ink) }
                    },
                    actions = {
                        IconButton(onClick = {}) { Icon(Icons.Default.Search, "Buscar", tint = ZapColor.slate) }
                        IconButton(onClick = {}) { Icon(Icons.Default.Photo, "Mídia", tint = ZapColor.slate) }
                        IconButton(onClick = {}) { Icon(Icons.Default.Phone, "Ligar", tint = ZapColor.primary) }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = ZapColor.canvas, titleContentColor = ZapColor.ink,
                    ),
                )
                Divider(color = ZapColor.hairline)
            }
        },
        bottomBar = { ChatInputPreview() },
    ) { pv ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(pv),
            contentPadding = PaddingValues(vertical = 12.dp),
            verticalArrangement = Arrangement.spacedBy(2.dp),
        ) {
            items(MockData.messages) { m ->
                ZapTextBubble(m.text, m.time, m.outgoing, m.status)
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ChatInputPreview() {
    Surface(color = ZapColor.surface, modifier = Modifier.fillMaxWidth()) {
        Column {
            Divider(color = ZapColor.hairline)
            Row(
                modifier = Modifier.padding(horizontal = 6.dp, vertical = 8.dp).fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                IconButton(onClick = {}) { Icon(Icons.Default.Add, "Anexar", tint = ZapColor.slate) }
                OutlinedTextField(
                    value = "", onValueChange = {},
                    modifier = Modifier.weight(1f),
                    placeholder = { Text("Mensagem", color = ZapColor.slate) },
                    shape = RoundedCornerShape(24.dp),
                    colors = OutlinedTextFieldDefaults.colors(
                        focusedContainerColor = ZapColor.chatCanvas,
                        unfocusedContainerColor = ZapColor.chatCanvas,
                        focusedBorderColor = Color.Transparent,
                        unfocusedBorderColor = Color.Transparent,
                    ),
                )
                Box(
                    modifier = Modifier.padding(start = 2.dp).size(46.dp)
                        .clip(CircleShape).background(ZapColor.sparkBrush),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(Icons.Filled.Send, "Enviar", tint = Color.White, modifier = Modifier.size(22.dp))
                }
            }
        }
    }
}

private data class PreviewTab(val label: String, val icon: ImageVector)

data class MockMessage(val text: String, val time: String, val outgoing: Boolean, val status: BubbleStatus)

/** Dados fictícios coerentes (nomes brasileiros, prévias realistas). */
object MockData {
    val conversations = listOf(
        ConversationUi("12D3KooWAlice", "Alice Martins", "Perfeito, te vejo amanhã 👍", "09:24", unread = 2, online = true),
        ConversationUi("12D3KooWBruno", "Bruno Carvalho", "Você: já enviei o arquivo", "08:50", unread = 0),
        ConversationUi("12D3KooWCarla", "Carla Nogueira", "kkkk boa 😄", "Ontem", unread = 0, online = true),
        ConversationUi("12D3KooWDaniel", "Daniel Souza", "Bora marcar aquele café?", "Ontem", unread = 5),
        ConversationUi("12D3KooWElaine", "Elaine Ribeiro", "obrigada!! ✨", "Seg", unread = 0),
        ConversationUi("12D3KooWFabio", "Fábio Torres", "documento assinado em anexo", "Seg", unread = 0),
        ConversationUi("12D3KooWGrupo", "Time ZapLivre", "Marina: subi a build nova", "Dom", unread = 12, online = true),
        ConversationUi("12D3KooWHelena", "Helena Prado", "👋", "12/06", unread = 0),
    )

    val messages = listOf(
        MockMessage("Oi Carla! Tudo certo pra amanhã?", "09:10", true, BubbleStatus.READ),
        MockMessage("Oi! Tudo sim 😄", "09:12", false, BubbleStatus.NONE),
        MockMessage("Combinado então. Te mando o endereço mais tarde", "09:12", true, BubbleStatus.READ),
        MockMessage("Perfeito. E leva aquele documento que a gente falou?", "09:14", false, BubbleStatus.NONE),
        MockMessage("Já tá assinado, levo impresso 👍", "09:15", true, BubbleStatus.DELIVERED),
        MockMessage("kkkk boa, você salva sempre", "09:16", false, BubbleStatus.NONE),
        MockMessage("Até amanhã então ✨", "09:17", true, BubbleStatus.SENT),
    )
}
