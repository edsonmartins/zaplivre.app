package com.zaplivre.ui.components

import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Done
import androidx.compose.material.icons.filled.DoneAll
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.zaplivre.ui.theme.BubbleShape
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapType

/** Ticks de status de entrega (espelha o MessageStatusIndicator, para preview). */
enum class BubbleStatus { NONE, SENT, DELIVERED, READ }

/**
 * Container visual da bolha de chat: forma com tail ([BubbleShape]), cor e
 * largura máxima. Compartilhado entre o MessageBubble real e o design preview
 * para garantir um único visual. O [content] recebe a cor de texto adequada.
 */
@OptIn(ExperimentalFoundationApi::class)
@Composable
fun ZapBubbleContainer(
    outgoing: Boolean,
    modifier: Modifier = Modifier,
    onLongPress: (() -> Unit)? = null,
    content: @Composable (textColor: Color) -> Unit,
) {
    val bg = if (outgoing) ZapColor.bubbleOut else ZapColor.bubbleIn
    val fg = if (outgoing) ZapColor.bubbleOutInk else ZapColor.bubbleInInk
    Row(
        modifier = Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 2.dp),
        horizontalArrangement = if (outgoing) Arrangement.End else Arrangement.Start,
    ) {
        Box(
            modifier = modifier
                .widthIn(max = 300.dp)
                .clip(BubbleShape(outgoing))
                .background(bg)
                .then(
                    if (onLongPress != null) {
                        Modifier.combinedClickable(onClick = {}, onLongClick = onLongPress)
                    } else Modifier
                )
                .padding(start = if (outgoing) 12.dp else 14.dp, end = if (outgoing) 14.dp else 12.dp, top = 8.dp, bottom = 8.dp),
        ) {
            content(fg)
        }
    }
}

/**
 * Bolha de texto completa (texto + hora + ticks) para o design preview.
 */
@Composable
fun ZapTextBubble(
    text: String,
    time: String,
    outgoing: Boolean,
    status: BubbleStatus = BubbleStatus.NONE,
) {
    ZapBubbleContainer(outgoing = outgoing) { fg ->
        Column {
            Text(text = text, color = fg, style = ZapType.body)
            Spacer(Modifier.width(0.dp))
            Row(
                modifier = Modifier.align(Alignment.End),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(text = time, style = ZapType.caption, color = fg.copy(alpha = 0.6f))
                if (outgoing && status != BubbleStatus.NONE) {
                    Spacer(Modifier.width(3.dp))
                    val tick = if (status == BubbleStatus.SENT) Icons.Filled.Done else Icons.Filled.DoneAll
                    val tint = if (status == BubbleStatus.READ) ZapColor.spark else fg.copy(alpha = 0.6f)
                    Icon(tick, contentDescription = null, tint = tint, modifier = Modifier.width(16.dp))
                }
            }
        }
    }
}
