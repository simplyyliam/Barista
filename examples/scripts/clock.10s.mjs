// A tiny Node script item: the time and the day of the year. Rename it to clock.2s.mjs and
// the interval changes without restarting Barista.
const now = new Date();
const time = now.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
const start = new Date(now.getFullYear(), 0, 0);
const day = Math.floor((now - start) / 86_400_000);

console.log(time);
console.log("---");
console.log(`Day ${day} of the year`);
console.log("Details");
console.log(`--${now.toDateString()}`);
console.log(`--Week ${Math.ceil(day / 7)}`);
console.log("Open the calendar | shell=cmd param1=/c param2=start param3=outlookcal:");
console.log("Refresh now | refresh=true");
