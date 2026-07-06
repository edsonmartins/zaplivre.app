package com.zaplivre.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import kotlin.math.abs

/**
 * Paleta ZapLivre (espelha ios/.../DesignSystem/ZapTheme.swift).
 *
 * A identidade: familiar como um mensageiro moderno, mas com o azul ZapLivre no
 * lugar do verde e um gradiente "spark" (raio) usado com muita restrição — só
 * onde o app quer chamar a ação. Tudo o mais é quieto.
 *
 * As cores adaptam light/dark via [LocalZapColors], provido pelo [ZapLivreTheme].
 * Nas telas, use `ZapColor.primary` etc (leitura em contexto @Composable),
 * exatamente como o `ZapColor` do iOS.
 */
data class ZapColors(
    val primary: Color,
    val spark: Color,
    val ink: Color,
    val slate: Color,
    val canvas: Color,
    val chatCanvas: Color,
    val surface: Color,
    val hairline: Color,
    val bubbleOut: Color,
    val bubbleOutInk: Color,
    val bubbleIn: Color,
    val bubbleInInk: Color,
    val online: Color,
    val danger: Color,
    val onPrimary: Color,
    val isDark: Boolean,
) {
    /** Gradiente signature (o "raio"): azul → ciano. Usar com restrição. */
    val sparkBrush: Brush
        get() = Brush.linearGradient(listOf(Color(0xFF2F6BFF), Color(0xFF37E0FF)))

    /** Paleta de avatares sem foto — cor derivada do id, para dar vida à lista. */
    val avatarPalette: List<Color>
        get() = listOf(
            Color(0xFF2F6BFF), Color(0xFF7C5CFF), Color(0xFF00A6A6),
            Color(0xFFE8618C), Color(0xFFF2884B), Color(0xFF1FA971),
            Color(0xFF4B7BEC), Color(0xFFB8449B),
        )

    /**
     * Cor estável derivada de um id (djb2) — mesma lógica dos avatares.
     * Usada para avatar sem foto e nome de autor em grupos.
     */
    fun accent(seed: String): Color {
        var hash = 5381
        for (b in seed.toByteArray()) hash = ((hash shl 5) + hash) + b.toInt()
        val palette = avatarPalette
        return palette[abs(hash) % palette.size]
    }
}

val LightZapColors = ZapColors(
    primary = Color(0xFF2F6BFF),
    spark = Color(0xFF37E0FF),
    ink = Color(0xFF0D1B2A),
    slate = Color(0xFF667085),
    canvas = Color(0xFFFFFFFF),
    chatCanvas = Color(0xFFEDF1F7),
    surface = Color(0xFFFFFFFF),
    hairline = Color(0xFFE6E9EF),
    bubbleOut = Color(0xFF2F6BFF),
    bubbleOutInk = Color.White,
    bubbleIn = Color(0xFFFFFFFF),
    bubbleInInk = Color(0xFF0D1B2A),
    online = Color(0xFF22C55E),
    danger = Color(0xFFE5484D),
    onPrimary = Color.White,
    isDark = false,
)

val DarkZapColors = ZapColors(
    primary = Color(0xFF3D78FF),
    spark = Color(0xFF37E0FF),
    ink = Color(0xFFE9EDF1),
    slate = Color(0xFF8A97A3),
    canvas = Color(0xFF0B141A),
    chatCanvas = Color(0xFF0B141A),
    surface = Color(0xFF1F2C33),
    hairline = Color(0xFF223038),
    bubbleOut = Color(0xFF1B49B8),
    bubbleOutInk = Color.White,
    bubbleIn = Color(0xFF1F2C33),
    bubbleInInk = Color(0xFFE9EDF1),
    online = Color(0xFF2ED573),
    danger = Color(0xFFFF6369),
    onPrimary = Color.White,
    isDark = true,
)

val LocalZapColors = staticCompositionLocalOf { LightZapColors }

/** Acesso aos tokens ZapLivre em contexto @Composable: `ZapColor.primary`. */
object ZapColor {
    val current: ZapColors
        @Composable @ReadOnlyComposable get() = LocalZapColors.current

    val primary: Color @Composable @ReadOnlyComposable get() = current.primary
    val spark: Color @Composable @ReadOnlyComposable get() = current.spark
    val ink: Color @Composable @ReadOnlyComposable get() = current.ink
    val slate: Color @Composable @ReadOnlyComposable get() = current.slate
    val canvas: Color @Composable @ReadOnlyComposable get() = current.canvas
    val chatCanvas: Color @Composable @ReadOnlyComposable get() = current.chatCanvas
    val surface: Color @Composable @ReadOnlyComposable get() = current.surface
    val hairline: Color @Composable @ReadOnlyComposable get() = current.hairline
    val bubbleOut: Color @Composable @ReadOnlyComposable get() = current.bubbleOut
    val bubbleOutInk: Color @Composable @ReadOnlyComposable get() = current.bubbleOutInk
    val bubbleIn: Color @Composable @ReadOnlyComposable get() = current.bubbleIn
    val bubbleInInk: Color @Composable @ReadOnlyComposable get() = current.bubbleInInk
    val online: Color @Composable @ReadOnlyComposable get() = current.online
    val danger: Color @Composable @ReadOnlyComposable get() = current.danger
    val onPrimary: Color @Composable @ReadOnlyComposable get() = current.onPrimary

    val sparkBrush: Brush @Composable @ReadOnlyComposable get() = current.sparkBrush
    val avatarPalette: List<Color> @Composable @ReadOnlyComposable get() = current.avatarPalette

    @Composable @ReadOnlyComposable
    fun accent(seed: String): Color = current.accent(seed)
}
