package com.zaplivre.core

/**
 * Service locator mínimo para injeção de dependências sem framework.
 *
 * Em produção aponta para o singleton real ([ZapLivreClientWrapper]);
 * em testes pode ser substituído por um fake/mock de [ZapLivreClientApi].
 */
object ServiceLocator {
    var clientApi: ZapLivreClientApi = ZapLivreClientWrapper
}
