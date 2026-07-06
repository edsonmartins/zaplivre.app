package com.zaplivre.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.zaplivre.ui.theme.ZapColor
import com.zaplivre.ui.theme.ZapMetric
import com.zaplivre.ui.theme.ZapType

/**
 * Avatar sem foto: círculo com cor estável derivada do [seed] e a inicial do
 * [name]. Espelha o AvatarView do iOS. Opcionalmente mostra o ponto de presença.
 */
@Composable
fun ZapAvatar(
    seed: String,
    name: String,
    modifier: Modifier = Modifier,
    size: Dp = ZapMetric.avatar,
    online: Boolean = false,
) {
    val color = ZapColor.accent(seed)
    val initial = name.trim().firstOrNull()?.uppercaseChar()?.toString() ?: "?"
    Box(modifier = modifier.size(size), contentAlignment = Alignment.Center) {
        Box(
            modifier = Modifier
                .size(size)
                .clip(CircleShape)
                .background(color),
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = initial,
                color = Color.White,
                fontWeight = FontWeight.SemiBold,
                fontSize = (size.value * 0.42f).sp,
            )
        }
        if (online) {
            Box(
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .size(size * 0.28f)
                    .clip(CircleShape)
                    .background(ZapColor.canvas),
                contentAlignment = Alignment.Center,
            ) {
                Box(
                    modifier = Modifier
                        .size(size * 0.20f)
                        .clip(CircleShape)
                        .background(ZapColor.online)
                )
            }
        }
    }
}

/**
 * Logo ZapLivre: quadrado arredondado com o gradiente spark (raio) e um ícone
 * de raio branco. Assinatura visual da marca (onboarding, splash).
 */
@Composable
fun ZapLogo(size: Dp = 96.dp, modifier: Modifier = Modifier) {
    Box(
        modifier = modifier
            .size(size)
            .clip(RoundedCornerShape(size * 0.28f))
            .background(ZapColor.sparkBrush),
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = Icons.Filled.Bolt,
            contentDescription = null,
            tint = Color.White,
            modifier = Modifier.size(size * 0.56f),
        )
    }
}

/**
 * Botão primário com o gradiente spark de fundo — a ação principal da tela.
 * Usar com restrição (uma por tela), como o iOS.
 */
@Composable
fun ZapGradientButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    leading: @Composable (RowScope.() -> Unit)? = null,
) {
    val bg: Brush = if (enabled) ZapColor.sparkBrush else {
        val disabled = ZapColor.hairline
        Brush.linearGradient(listOf(disabled, disabled))
    }
    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(54.dp)
            .clip(RoundedCornerShape(ZapMetric.buttonRadius))
            .background(bg)
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            leading?.invoke(this)
            Text(
                text = text,
                color = if (enabled) Color.White else ZapColor.slate,
                style = ZapType.rowName,
                textAlign = TextAlign.Center,
            )
        }
    }
}
