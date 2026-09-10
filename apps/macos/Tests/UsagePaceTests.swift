import XCTest
@testable import Notch

final class UsagePaceTests: XCTestCase {
    private let now = Date(timeIntervalSince1970: 1_800_000_000)

    private func pace(
        used: Double? = 0.5,
        remaining: TimeInterval? = 302400,
        duration: TimeInterval? = 604800
    ) -> UsagePace? {
        LimitWindow(id: "w", label: "Weekly", usedFraction: used,
                    resetsAt: remaining.map { now.addingTimeInterval($0) }, duration: duration)
            .usagePace(now: now)
    }

    private struct PaceFixture: Decodable {
        let name: String
        let used: Double?
        let remaining: Double?
        let duration: Double?
        let points: Double?
        let summary: String?
    }

    func testSharedNativePaceContract() throws {
        let url = try XCTUnwrap(Bundle(for: Self.self).url(forResource: "usage-pace-fixtures", withExtension: "json"))
        let fixtures = try JSONDecoder().decode([PaceFixture].self, from: Data(contentsOf: url))
        for fixture in fixtures {
            let actual = pace(used: fixture.used, remaining: fixture.remaining, duration: fixture.duration)
            if let points = fixture.points {
                let actual = try XCTUnwrap(actual, fixture.name)
                XCTAssertEqual(actual.percentagePoints, points, accuracy: 0.000001, fixture.name)
                XCTAssertEqual(actual.summary, fixture.summary, fixture.name)
                XCTAssertEqual(actual.isDeficit, points > 0, fixture.name)
            } else { XCTAssertNil(actual, fixture.name) }
        }
    }

    func testNonFiniteReadingsHaveNoPace() {
        for invalid in [pace(used: .infinity), pace(used: .nan), pace(duration: .infinity), pace(remaining: .infinity)] {
            XCTAssertNil(invalid)
        }
    }

    func testDurationCodingRemainsBackwardCompatible() throws {
        let original = LimitWindow(id: "w", label: "Weekly", usedFraction: 0.98,
                                   resetsAt: now.addingTimeInterval(86400), duration: 604800)
        XCTAssertEqual(try JSONDecoder().decode(LimitWindow.self,
                       from: JSONEncoder().encode(original)), original)

        let archived = try JSONDecoder().decode(LimitWindow.self,
            from: Data(#"{"id":"w","label":"Weekly","usedFraction":0.98}"#.utf8))
        XCTAssertNil(archived.duration)
    }
}

@MainActor
final class UsagePacePreferenceTests: XCTestCase {
    func testDefaultsOffAndSurvivesARelaunch() throws {
        let name = "UsagePacePreferenceTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        defer { defaults.removePersistentDomain(forName: name) }
        XCTAssertFalse(Preferences(defaults: defaults).showUsagePace)
        Preferences(defaults: defaults).showUsagePace = true
        XCTAssertTrue(Preferences(defaults: defaults).showUsagePace)
    }
}
