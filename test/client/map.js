const map = {};

export function updateMap(cells) {
  console.log("updateMap");

  const i = mapZ2toN(6, 6);

  for (let i = 0; i < 15; ++i) {
    const [z1, z2] = inverseZ2(i);
    const n = mapZ2toN(z1, z2);
    //if (i !== n) {
      console.log(`${[z1, z2]} - ${n}`);
    //}
  }
}

export function printMap() {
  console.log("printMap");
}

function mapZtoN(z) {
  if (z >= 0) {
    return 2 * z;
  } else {
    return -2 * z - 1;
  }
}

function mapCantorPairing(n1, n2) {
  return n2 + ((n1 + n2) * (n1 + n2 + 1)) / 2;
}

function mapZ2toN(z1, z2) {
  return mapCantorPairing(mapZtoN(z1), mapZtoN(z2));
}

function mapNtoZ2(n) {
  const w = Math.floor((Math.sqrt(8 * n + 1) - 1) / 2);
  const t = (w * (w + 1)) / 2;
  const b = n - t;
  const a = w - b;
}

function inverseZ2(n) {
  if (!Number.isInteger(n) || n < 0)
    throw new Error("n must be a nonnegative integer");

  // Cantor inverse: recover a,b from n
  const w = Math.floor((Math.sqrt(8 * n + 1) - 1) / 2);
  const t = (w * (w + 1)) / 2;
  const b = n - t;
  const a = w - b;

  // f^{-1}: map natural m back to integer
  const fInv = (m) => (m % 2 === 0 ? m / 2 : -((m + 1) / 2));

  const z1 = fInv(a);
  const z2 = fInv(b);
  return [z1, z2];
}
