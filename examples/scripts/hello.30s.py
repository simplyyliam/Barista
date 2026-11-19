# The smallest Python script item: a greeting that changes with the time of day.
# Drop it in %APPDATA%\Barista\scripts. Runs every 30 seconds ("30s" in the name).
import datetime

now = datetime.datetime.now()
greeting = "Good morning" if now.hour < 12 else "Good afternoon" if now.hour < 18 else "Good evening"

print(f"Hello, {now:%H:%M}")  # the bar label
print("---")                  # the menu starts here
print(f"{greeting}!")
print("More")
print("--Today is " + now.strftime("%A, %d %B"))
print("--Python says hi")
print("Say hi back | href=mailto:hello@example.com")
print("Refresh now | refresh=true")
