using System;

public interface Describe
{
    string Description()
    {
        return "thing";
    }
}

public interface Sized
{
    int Size();
}

public class Base
{
    public int Area()
    {
        return 1;
    }
}

public class Circle : Base, Describe
{
    public int Radius()
    {
        return 1;
    }
}

public class Plain
{
    public int Value()
    {
        return 1;
    }
}

public interface Sub : Describe
{
    string Name()
    {
        return "sub";
    }
}

public record Point(int X, int Y);

public record Named(int X, int Y, string Label) : Point(X, Y), Sized
{
    public int Size()
    {
        return X + Y;
    }
}

public struct Square : Sized
{
    public int Size()
    {
        return 1;
    }
}

public class Ranked : IComparable<Ranked>
{
    public int CompareTo(Ranked other)
    {
        return 0;
    }
}
