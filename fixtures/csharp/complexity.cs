using System;

public class Complexity
{
    private int width;
    private int height;

    public Complexity(int width, int height)
    {
        if (width > 0)
        {
            this.width = width;
        }
        this.height = height;
    }

    ~Complexity()
    {
        if (width > 0)
        {
            width = 0;
        }
    }

    public int Area => width > 0 ? width * height : 0;

    public int Count { get; set; }

    public string Label
    {
        get
        {
            if (width > 10)
            {
                return "big";
            }
            return "small";
        }
        set
        {
            if (value.Length > 0)
            {
                width = value.Length;
            }
        }
    }

    public static Complexity operator +(Complexity left, Complexity right)
    {
        if (left.width > right.width)
        {
            return left;
        }
        return right;
    }

    public int Branchy(int x)
    {
        if (x > 0)
        {
            return 1;
        }
        else if (x < -10)
        {
            return -2;
        }

        for (int i = 0; i < x; i++)
        {
            while (i > 2)
            {
                break;
            }
        }

        int[] values = { 1, 2 };
        foreach (int value in values)
        {
            x += value;
        }

        do
        {
            x--;
        }
        while (x > 5);

        switch (x)
        {
            case 1:
                return 1;
            case 2:
                return 2;
            default:
                break;
        }

        int y = x > 3 ? 1 : 0;
        string label = y switch
        {
            1 => "one",
            2 when x > 0 => "two",
            _ => "other",
        };

        if (y > 1 && y < 100 || x == -5)
        {
            return y;
        }

        string maybe = null;
        string text = maybe ?? label;
        maybe ??= text;

        try
        {
            CanThrow();
        }
        catch (Exception)
        {
            return -1;
        }

        int Doubled(int n)
        {
            if (n > 0)
            {
                return n * 2;
            }
            return 0;
        }

        Func<int, int> triple = n => n > 0 ? n * 3 : 0;
        return Doubled(y) + triple(y) + text.Length;
    }

    private void CanThrow()
    {
    }
}

public interface Labeled
{
    string Name();

    string Label(int size)
    {
        if (size > 10)
        {
            return "big";
        }
        return "small";
    }
}

public struct Celsius
{
    public double Degrees;

    public Celsius(double degrees)
    {
        Degrees = degrees;
    }

    public static explicit operator Celsius(double degrees)
    {
        if (degrees < -273.15)
        {
            return new Celsius(-273.15);
        }
        return new Celsius(degrees);
    }
}
