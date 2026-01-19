const MAP_WIDTH = 200;
const MAP_HEIGHT = 200;

const map = {
  width: MAP_WIDTH,
  height: MAP_HEIGHT,
  cells: new Array(MAP_WIDTH * MAP_HEIGHT),
};

export function updateMap(cells) {
  console.log("updateMap");

  const originX = Math.floor(map.width / 2);
  const originY = Math.floor(map.height / 2);
  let mapIndex = null;

  for (const cell of cells) {
    if (typeof cell.x === "number") {
      mapIndex = originX + cell.x + map.width * (originY + cell.y);
    } else {
      mapIndex += 1;
    }

    if (typeof cell.g === "string") {
      map.cells[mapIndex] = cell.g;
    }
  }
}

export function printMap() {
  console.log("printMap");

  let minX = map.width,
    maxX = 0,
    minY = map.height,
    maxY = 0;

  for (let y = 0; y < map.height; y += 1) {
    for (let x = 0; x < map.width; x += 1) {
      const i = x + y * map.width;
      if (map.cells[i] !== undefined) {
        if (x < minX) minX = x;
        if (x > maxX) maxX = x;
        if (y < minY) minY = y;
        if (y > maxY) maxY = y;
      }
    }
  }

  console.log(`${minX},${minY} - ${maxX},${maxY}`);
  for (let y = minY; y <= maxY; y += 1) {
    for (let x = minX; x <= maxX; x += 1) {
      const i = x + y * map.width;
      if (map.cells[i] === undefined) {
        process.stdout.write(" ");
      } else {
        process.stdout.write(map.cells[i]);
      }
    }

    process.stdout.write("\n");
  }
}
