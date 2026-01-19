const fs = require("fs");
const map = require("./map.js");

const raw = fs.readFileSync("../research/login/09-map.json", "utf8");
const mapJson = JSON.parse(raw);

map.updateMap(mapJson.msgs[0].cells)
map.printMap();
