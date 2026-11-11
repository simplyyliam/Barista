# Battery example

Shows the Windows battery percentage and charging state in the Barista bar using
`@barista/sdk`.

Run it from the repository root while Barista is running:

```sh
pnpm --filter @barista/battery start
```

The item refreshes every 30 seconds and also refreshes when clicked. It reads
`Win32_Battery` through PowerShell, so it only works on Windows devices with a
battery that Windows reports. On a desktop or unsupported device, it displays
`N/A`.
