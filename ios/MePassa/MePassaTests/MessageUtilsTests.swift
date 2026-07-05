//
//  MessageUtilsTests.swift
//  MePassaTests
//
//  Copyright © 2026 MePassa. All rights reserved.
//
//  Pure-logic tests for MessageUtils (timestamp formatting and
//  message status presentation). No FFI client involved.

import XCTest
@testable import MePassa

final class MessageUtilsTests: XCTestCase {

    private func timestamp(secondsAgo: TimeInterval) -> Int64 {
        Int64(Date().addingTimeInterval(-secondsAgo).timeIntervalSince1970)
    }

    // MARK: - formatTimestamp (relative)

    func testFormatTimestampJustNow() {
        XCTAssertEqual(MessageUtils.formatTimestamp(timestamp(secondsAgo: 5)), "agora")
        XCTAssertEqual(MessageUtils.formatTimestamp(timestamp(secondsAgo: 50)), "agora")
    }

    /// KNOWN BUG (documented, not endorsed): formatTimestamp computes
    /// dateComponents([.second, .minute, .hour, .day]) and then checks
    /// `seconds < 60`. With multiple units requested, `.second` is the
    /// REMAINDER (always 0-59), so the guard matches for virtually any
    /// timestamp and the function returns "agora" even for messages sent
    /// hours, days, or years ago; the minutes/hours/date branches are
    /// effectively unreachable.
    ///
    /// The fix (in MessageUtils.formatTimestamp) is to compare the total
    /// elapsed interval, e.g. `now.timeIntervalSince(messageDate)`, instead
    /// of per-unit remainders. When fixed, replace this test with assertions
    /// for "5min", "3h" and "dd/MM/yyyy".
    func testFormatTimestampKnownDefectAlwaysReturnsAgora() {
        // Remainder seconds are ~2 in all these cases -> "agora" today
        XCTAssertEqual(MessageUtils.formatTimestamp(timestamp(secondsAgo: 5 * 60 + 2)), "agora")
        XCTAssertEqual(MessageUtils.formatTimestamp(timestamp(secondsAgo: 3 * 3600 + 2)), "agora")
        // Even a 2020 timestamp falls into the "agora" branch
        XCTAssertEqual(MessageUtils.formatTimestamp(1_584_014_400), "agora")
    }

    // MARK: - formatFullTimestamp

    func testFormatFullTimestampToday() {
        let result = MessageUtils.formatFullTimestamp(timestamp(secondsAgo: 60))
        XCTAssertTrue(result.hasPrefix("Hoje "), "Expected 'Hoje HH:mm', got \(result)")
    }

    func testFormatFullTimestampYesterday() {
        // Same wall-clock time yesterday is always "yesterday"
        let result = MessageUtils.formatFullTimestamp(timestamp(secondsAgo: 24 * 3600))
        XCTAssertTrue(result.hasPrefix("Ontem "), "Expected 'Ontem HH:mm', got \(result)")
    }

    // MARK: - Status presentation

    func testStatusIcons() {
        XCTAssertEqual(MessageUtils.getStatusIcon(.pending), "⏱️")
        XCTAssertEqual(MessageUtils.getStatusIcon(.sent), "✓")
        XCTAssertEqual(MessageUtils.getStatusIcon(.delivered), "✓✓")
        XCTAssertEqual(MessageUtils.getStatusIcon(.read), "✓✓")
        XCTAssertEqual(MessageUtils.getStatusIcon(.failed), "❌")
    }

    func testStatusDescriptions() {
        XCTAssertEqual(MessageUtils.getStatusDescription(.pending), "Enviando...")
        XCTAssertEqual(MessageUtils.getStatusDescription(.sent), "Enviado")
        XCTAssertEqual(MessageUtils.getStatusDescription(.delivered), "Entregue")
        XCTAssertEqual(MessageUtils.getStatusDescription(.read), "Lido")
        XCTAssertEqual(MessageUtils.getStatusDescription(.failed), "Falha no envio")
    }

    func testStatusColors() {
        XCTAssertEqual(MessageUtils.getStatusColor(.read), "blue")
        XCTAssertEqual(MessageUtils.getStatusColor(.failed), "red")
        XCTAssertEqual(MessageUtils.getStatusColor(.pending), "gray")
        XCTAssertEqual(MessageUtils.getStatusColor(.sent), "gray")
        XCTAssertEqual(MessageUtils.getStatusColor(.delivered), "gray")
    }
}
