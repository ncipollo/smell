interface Describe {
    default String describe() {
        return "thing";
    }
}

class Base {
    int area() {
        return 1;
    }
}

class Circle extends Base implements Describe, Comparable<Circle> {
    public int compareTo(Circle other) {
        return 0;
    }
}

class Plain {
    int value() {
        return 1;
    }
}

interface Sub extends Describe {
    default String name() {
        return "sub";
    }
}
