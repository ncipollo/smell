function simple() {
  console.log("hi");
}

function branchy(x) {
  if (x > 0) {
    return 1;
  } else if (x < -10) {
    return -2;
  }
  for (let i = 0; i < x; i++) {
    while (i > 2) {
      break;
    }
  }
  let y = x;
  do {
    y--;
  } while (y > 5);
  for (const n of [1, 2, 3]) {
    if (n === y) {
      break;
    }
  }
  switch (x) {
    case 1:
      return 1;
    case 2:
      return 2;
    default:
      break;
  }
  const label = null;
  const fallback = label ?? "none";
  if ((y > 1 && y < 100) || x === -5) {
    return y;
  }
  try {
    canThrow();
    return fallback.length;
  } catch (error) {
    return -1;
  }
}

function canThrow() {}

const double = (n) => (n > 0 ? n * 2 : 0);

class Shape {
  constructor(width) {
    this.width = width;
  }

  get area() {
    return this.width > 0 ? this.width * this.width : 0;
  }

  set area(value) {
    if (value > 0) {
      this.width = value;
    }
  }

  describe() {
    return this.area > 10 ? "big" : "small";
  }
}
