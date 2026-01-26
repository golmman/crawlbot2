export function createMessage(command, messageHook) {
  let m = "";

  if (command === "/hook1") m = hook1(messageHook);
  else if (command === "/hook2") m = hook2(messageHook);
  else if (command === "/start") m = start(messageHook);
  else console.log(`unknown command: ${command}`);

  return m;
}

// dispatcher
// ping pong

function hook1(messageHook) {
  console.log("hook1");
  messageHook.callback = (messages) => {
    console.log(`hook1 received ${messages.length} messages`);
  };

  return "";
}

function hook2(messageHook) {
  console.log("hook2");
  messageHook.callback = (messages) => {
    console.log(`hook2 received ${messages.length} messages`);
  };

  return "";
}

function start(messageHook) {
  messageHook.callback = (messages) => {
    console.log(`hook2 received ${messages.length} messages`);
  };

  return JSON.stringify({"msg":"register","username":"dirkle","password":"aaa","email":""});
}
