package com.zaplivre.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.RoundRect
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp

/** Métricas ZapLivre (espelha ZapMetric do iOS). */
object ZapMetric {
    val bubbleRadius = 18.dp
    val cardRadius = 16.dp
    val buttonRadius = 14.dp
    val avatar = 52.dp
    val avatarSmall = 38.dp

    val gutter = 16.dp
    val rowGap = 12.dp
    val tight = 8.dp
}

val ZapShapes = Shapes(
    extraSmall = RoundedCornerShape(10.dp),
    small = RoundedCornerShape(ZapMetric.buttonRadius),
    medium = RoundedCornerShape(ZapMetric.cardRadius),
    large = RoundedCornerShape(ZapMetric.bubbleRadius),
    extraLarge = RoundedCornerShape(28.dp),
)

/**
 * Bolha de chat com "rabinho" (tail), espelhando o BubbleShape do iOS: um único
 * caminho contínuo com raio [ZapMetric.bubbleRadius] em três cantos e o canto
 * inferior do lado do remetente reduzido, com uma pequena aba apontando para
 * fora. [outgoing] = true desenha o tail à direita (mensagem enviada).
 */
class BubbleShape(private val outgoing: Boolean, private val tail: Boolean = true) : Shape {
    override fun createOutline(
        size: androidx.compose.ui.geometry.Size,
        layoutDirection: LayoutDirection,
        density: Density,
    ): Outline {
        val r = with(density) { ZapMetric.bubbleRadius.toPx() }
        val t = with(density) { 7.dp.toPx() } // largura reservada do tail
        val w = size.width
        val h = size.height
        val path = Path()

        if (outgoing) {
            // Corpo arredondado; canto inferior-direito reto com aba (tail).
            path.moveTo(r, 0f)
            path.lineTo(w - r, 0f)
            path.quadraticBezierTo(w, 0f, w, r)                       // canto sup-dir
            if (tail) {
                path.lineTo(w, h)                                     // desce reto
                path.lineTo(w - t, h - 2f)                            // aba do tail
            } else {
                path.lineTo(w, h - r)
                path.quadraticBezierTo(w, h, w - r, h)
            }
            path.lineTo(r, h)
            path.quadraticBezierTo(0f, h, 0f, h - r)                  // canto inf-esq
            path.lineTo(0f, r)
            path.quadraticBezierTo(0f, 0f, r, 0f)                     // canto sup-esq
        } else {
            // Espelhado: tail no canto inferior-esquerdo.
            path.moveTo(r, 0f)
            path.lineTo(w - r, 0f)
            path.quadraticBezierTo(w, 0f, w, r)                       // canto sup-dir
            path.lineTo(w, h - r)
            path.quadraticBezierTo(w, h, w - r, h)                    // canto inf-dir
            if (tail) {
                path.lineTo(t, h - 2f)                                // aba do tail
                path.lineTo(0f, h)                                    // ponta
            } else {
                path.lineTo(r, h)
                path.quadraticBezierTo(0f, h, 0f, h - r)
            }
            path.lineTo(0f, r)
            path.quadraticBezierTo(0f, 0f, r, 0f)                     // canto sup-esq
        }
        path.close()
        return Outline.Generic(path)
    }
}

/** Retângulo com um canto menos arredondado no lado do remetente (sem tail). */
fun bubbleRounded(outgoing: Boolean): RoundedCornerShape {
    val big = ZapMetric.bubbleRadius
    val small = 4.dp
    return if (outgoing) {
        RoundedCornerShape(topStart = big, topEnd = big, bottomStart = big, bottomEnd = small)
    } else {
        RoundedCornerShape(topStart = big, topEnd = big, bottomStart = small, bottomEnd = big)
    }
}

@Suppress("unused")
private fun roundRect(w: Float, h: Float, r: Float) =
    RoundRect(0f, 0f, w, h, CornerRadius(r))
