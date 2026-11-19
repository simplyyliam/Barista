// Test fixture: prints 10 MB. The runner must cut it off at 64 KB.
const chunk = "x".repeat(1024 * 1024);
for (let i = 0; i < 10; i++) process.stdout.write(chunk);
