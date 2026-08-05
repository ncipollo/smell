interface Describe {
  describe(): string;
}

interface Shaped extends Describe {
  area(): number;
}

abstract class Base {
  id(): number {
    return 1;
  }
}

class Circle extends Base implements Describe {
  describe(): string {
    return "circle";
  }
}

class Ranked implements Comparable<Ranked> {
  compareTo(other: Ranked): number {
    return 0;
  }
}

class Plain {
  value(): number {
    return 1;
  }
}
