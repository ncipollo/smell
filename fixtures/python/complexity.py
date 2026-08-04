def simple():
    identity = lambda v: v
    print(identity("hi"))


def branchy(x):
    if x > 0:
        return 1
    elif x < -10:
        return -2
    for i in range(x):
        while i > 2:
            i -= 1
    match x:
        case 1:
            return 1
        case 2 if x > 0:
            return 2
        case _:
            pass
    label = None
    fallback = label or "none"
    size = "big" if x > 10 else "small"
    evens = [i for i in range(x) if i % 2 == 0]
    if x > 1 and x < 100:
        return x
    try:
        can_throw()
    except ValueError:
        return -1
    return len(fallback) + len(size) + len(evens)


def can_throw():
    pass


class Shape:
    def __init__(self, width):
        if width < 0:
            width = 0
        self.width = width

    def area(self):
        return self.width * self.width if self.width > 0 else 0

    def describe(self):
        return "big" if self.area() > 10 else "small"

    async def resize(self, scale):
        if scale > 0:
            self.width *= scale

    @staticmethod
    def unit():
        return Shape(1)

    class Config:
        def validate(self):
            return True


class Registry:
    def register(self, shape):
        return shape.area() > 0
