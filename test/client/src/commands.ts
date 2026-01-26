export interface MessageHook {
  callback: ((messages: any[]) => void) | null;
}

export function createMessage(command: string, messageHook: MessageHook): string {
  let m = "";

  if (command === "/hook1") m = hook1(messageHook);
  else if (command === "/hook2") m = hook2(messageHook);
  else if (command === "/start") m = start(messageHook);
  else console.log(`unknown command: ${command}`);

  return m;
}

// dispatcher
// ping pong

function hook1(messageHook: MessageHook): string {
  console.log("hook1");
  messageHook.callback = (messages: any[]) => {
    console.log(`hook1 received ${messages.length} messages`);
  };

  return "";
}

function hook2(messageHook: MessageHook): string {
  console.log("hook2");
  messageHook.callback = (messages: any[]) => {
    console.log(`hook2 received ${messages.length} messages`);
  };

  return "";
}

function start(messageHook: MessageHook): string {
  messageHook.callback = (messages: any[]) => {
    console.log(`hook2 received ${messages.length} messages`);
  };

  return JSON.stringify({ "msg": "register", "username": "dirkle", "password": "aaa", "email": "" });
}
