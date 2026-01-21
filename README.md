# crawlbot2

## DCSS Server Interface

### Quick Start

```json
{"msg":"register","username":"dirkle","password":"aaa","email":""}
{"msg":"login","username":"dirkle","password":"aaa"}
{"msg":"play","game_id":"dcss-web-trunk"}
{"msg": "input","text": "f"}
{"msg": "input","text": "f"}
{"msg": "input","text": "f"}
```

### Abandon

```json
{"msg":"key","keycode":17}
{"msg":"input","text":"quit\r"}
{"msg":"input","keycode":27}
```


### Commands

Pong
```json
{"msg":"pong"}
```

Register
```json
{"msg":"register","username":"dirkle","password":"aaa","email":""}
```

Login
```json
{"msg":"login","username":"dirkle","password":"aaa"}
```

Play
```json
{"msg":"play","game_id":"dcss-web-trunk"}
```

Pick TrBe
```json
{"msg": "input","text": "f"}
{"msg": "input","text": "f"}
{"msg": "input","text": "f"}
```

Explore
```json
{"msg": "input","text": "o"}
```

Move
```json
{"msg": "input","text": "l"}
```
