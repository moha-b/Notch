import XCTest
@testable import Notch

final class GrokCredentialBoundaryTests: XCTestCase {
    func testLookalikeIssuersNeverSupplyCredentialsToThePublicEndpoint() {
        for issuer in ["https://auth.x.ai.attacker.test", "https://auth.x.ai@attacker.test", "https://auth.x.ai/customer"] {
            XCTAssertNil(GrokCredentials.pick(from: [issuer + "::client": ["key": "unrelated-token"]]), issuer)
        }
        let trusted = GrokCredentials.pick(from: ["https://auth.x.ai::client": ["key": "fixture-token"]])
        XCTAssertEqual(trusted?["key"] as? String, "fixture-token")
    }
}
