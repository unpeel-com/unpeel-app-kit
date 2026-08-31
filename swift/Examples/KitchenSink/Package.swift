// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "UnpeelAppKitKitchenSink",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .executable(name: "KitchenSink", targets: ["KitchenSink"]),
    ],
    dependencies: [
        // This executable consumes the renderer library; the library never
        // depends on the harness.
        .package(path: "../.."),
        .package(
            url: "https://github.com/migueldeicaza/SwiftTerm.git",
            exact: "1.19.0"
        ),
    ],
    targets: [
        .executableTarget(
            name: "KitchenSink",
            dependencies: [
                .product(name: "UnpeelAppKitUI", package: "swift"),
                .product(name: "SwiftTerm", package: "SwiftTerm"),
            ]
        ),
    ]
)
