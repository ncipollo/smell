class Describe:
    def describe(self):
        return "thing"


class Base:
    def area(self):
        return 1


class Circle(Base, Describe):
    def describe(self):
        return "circle"


class Plain:
    def value(self):
        return 1


class Ranked(Comparable[int]):
    def rank(self):
        return 0


class Registry(Describe, metaclass=type):
    def count(self):
        return 0
