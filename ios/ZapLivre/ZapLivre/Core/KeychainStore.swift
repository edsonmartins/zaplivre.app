//
//  KeychainStore.swift
//  ZapLivre
//
//  Secure storage for identity keypair (iOS Keychain)
//

import Foundation
import Security

enum KeychainStore {
    private static let service = "com.zaplivre.identity"
    private static let account = "identity.keypair"

    static func loadIdentity() throws -> Data? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound {
            return nil
        }
        guard status == errSecSuccess else {
            throw ZapLivreCoreError.storageError("Keychain read failed: \(status)")
        }
        return item as? Data
    }

    static func saveIdentity(_ data: Data) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]

        let attributes: [String: Any] = [
            kSecValueData as String: data
        ]

        let status: OSStatus
        if try loadIdentity() != nil {
            status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
        } else {
            var addQuery = query
            addQuery[kSecValueData as String] = data
            status = SecItemAdd(addQuery as CFDictionary, nil)
        }

        guard status == errSecSuccess else {
            throw ZapLivreCoreError.storageError("Keychain write failed: \(status)")
        }
    }

    static func deleteIdentity() throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account
        ]

        let status = SecItemDelete(query as CFDictionary)
        if status != errSecSuccess && status != errSecItemNotFound {
            throw ZapLivreCoreError.storageError("Keychain delete failed: \(status)")
        }
    }
}
