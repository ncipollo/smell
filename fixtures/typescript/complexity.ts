function simple(): void {
  console.log("hi");
}

function branchy(x: number): number {
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
  const label: string | null = null;
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

function canThrow(): void {}

const double = (n: number): number => (n > 0 ? n * 2 : 0);

class Shape {
  width: number;

  constructor(width: number) {
    this.width = width;
  }

  get area(): number {
    return this.width > 0 ? this.width * this.width : 0;
  }

  set area(value: number) {
    if (value > 0) {
      this.width = value;
    }
  }

  describe(): string {
    return this.area > 10 ? "big" : "small";
  }
}
