// A tiny app that will register status items in the bar. It is used in tests and demos.
// v0.0: it only proves the workspace wiring (the SDK builds and imports). v0.2 makes it
// connect to the bar and create an item.
import * as BA from "@barista/sdk"

console.log(`${BA.SDK_NAME} ${BA.SDK_VERSION}, protocol ${BA.PROTOCOL_VERSION}`)
console.log("test-client: connecting to the bar arrives in v0.2")
