public class Complexity {
    private int width;
    private int height;

    public Complexity(int width, int height) {
        if (width > 0) {
            this.width = width;
        }
        this.height = height;
    }

    public int branchy(int x) {
        if (x > 0) {
            return 1;
        } else if (x < -10) {
            return -2;
        }
        for (int i = 0; i < x; i++) {
            while (i > 2) {
                break;
            }
        }
        int[] values = {1, 2};
        for (int value : values) {
            x += value;
        }
        do {
            x--;
        } while (x > 5);
        switch (x) {
            case 1:
                return 1;
            case 2:
                return 2;
            default:
                break;
        }
        int y = x > 3 ? 1 : 0;
        String label = switch (y) {
            case 1 -> "one";
            default -> "other";
        };
        if (y > 1 && y < 100 || x == -5) {
            return y;
        }
        try {
            canThrow();
        } catch (Exception error) {
            return -1;
        }
        return label.length();
    }

    private void canThrow() throws Exception {}
}

interface Labeled {
    default String label(int size) {
        if (size > 10) {
            return "big";
        }
        return "small";
    }
}

enum Kind {
    CIRCLE,
    SQUARE;

    boolean isCircle() {
        return this == CIRCLE;
    }
}

record Point(int x, int y) {
    Point {
        if (x < 0) {
            throw new IllegalArgumentException();
        }
    }
}
