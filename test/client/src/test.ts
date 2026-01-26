import fs from "fs";
import * as map from "./map.ts";

const raw1 = fs.readFileSync("../research/login/09-map.json", "utf8");
const mapJson1 = JSON.parse(raw1);

const raw2 = fs.readFileSync("../research/move/01-msgs.json", "utf8");
const mapJson2 = JSON.parse(raw2);

map.updateMap(mapJson1.msgs[0].cells);
map.printMap();

map.updateMap(mapJson2.msgs[3].cells);
map.printMap();
