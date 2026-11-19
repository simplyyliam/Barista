// Test fixture: never finishes. The runner must stop it at the timeout and kill the whole tree.
console.log("sleeping");
setInterval(() => {}, 1000);
