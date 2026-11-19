// Test fixture: prints bytes that are not text.
process.stdout.write(Buffer.from([0, 255, 254, 0, 1, 2, 3]));
