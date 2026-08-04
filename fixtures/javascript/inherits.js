class Describe {
  describe() {
    return "thing";
  }
}

class Circle extends Describe {
  describe() {
    return "circle";
  }
}

class Plain {
  value() {
    return 1;
  }
}

class Container {}

const ns = { Container };

class Wide extends ns.Container {
  value() {
    return 1;
  }
}
