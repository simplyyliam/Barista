// Test fixture: starts a grandchild that lives for a minute, then hangs. After the timeout
// neither may be left in Task Manager.
import { spawn } from "node:child_process";
spawn(process.execPath, ["-e", "setTimeout(() => {}, 60000)"], { stdio: "ignore" });
console.log("child started");
setInterval(() => {}, 1000);
