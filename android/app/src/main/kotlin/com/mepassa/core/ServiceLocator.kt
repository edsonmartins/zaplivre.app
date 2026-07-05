package com.mepassa.core

/**
 * Service locator mínimo para injeção de dependências sem framework.
 *
 * Em produção aponta para o singleton real ([MePassaClientWrapper]);
 * em testes pode ser substituído por um fake/mock de [MePassaClientApi].
 */
object ServiceLocator {
    var clientApi: MePassaClientApi = MePassaClientWrapper
}
