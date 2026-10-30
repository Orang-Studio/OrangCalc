using System;
using System.Collections.Generic;
using System.Linq;

namespace OrangCalc.Engine
{
    public abstract class Item
    {
        public string Name;
        public List<string> Names = new List<string>();
        public string Title = "";
        public string Category = "";
        public string Description = "";
        public bool IsUser;
        public bool Hidden;
        public bool Active = true;

        public string DisplayTitle => string.IsNullOrEmpty(Title) ? Name : Title;
        public bool HasName(string n) => Names.Contains(n);
        public override string ToString() => DisplayTitle;
    }

    public sealed class Variable : Item
    {
        public Expr Value;
        public string ValueText = "";
        public bool IsConstant;
        public bool IsApproximate;
        public bool IsUnknown;
        public AssumeType AType = AssumeType.Real;
        public AssumeSign ASign = AssumeSign.Unknown;
        public Func<Calculator, Expr> Dynamic;
        public bool KeepSymbolic;
        public bool HasUnit;
    }

    public enum PrefixUse { None, Some, All, Binary }

    public sealed class Prefix
    {
        public string Name, Symbol;
        public Real Value;
        public int Exp;
        public bool Binary;
        public bool Common;
        public Prefix(string name, string sym, int exp, bool binary = false, bool common = true)
        {
            Name = name; Symbol = sym; Exp = exp; Binary = binary; Common = common;
            Value = binary ? Real.Int(2).PowInt(exp) : Real.Int(10).PowInt(exp);
        }
    }

    // exponents of m, kg, s, A, K, mol, cd, bit, currency, rad
    public sealed class Dim
    {
        public const int Count = 10;
        public readonly int[] E = new int[Count];
        public static readonly string[] Symbols = { "m", "kg", "s", "A", "K", "mol", "cd", "bit", "EUR", "rad" };

        public Dim() { }
        public Dim(int index) { E[index] = 1; }
        public bool IsNone => E.All(x => x == 0);
        public Dim Mul(Dim o) { var r = new Dim(); for (int i = 0; i < Count; i++) r.E[i] = E[i] + o.E[i]; return r; }
        public Dim Pow(int p) { var r = new Dim(); for (int i = 0; i < Count; i++) r.E[i] = E[i] * p; return r; }
        public bool SameAs(Dim o) { for (int i = 0; i < Count; i++) if (E[i] != o.E[i]) return false; return true; }
        public int Complexity => E.Sum(Math.Abs);
        public bool WithoutAngleSame(Dim o) { for (int i = 0; i < Count - 1; i++) if (E[i] != o.E[i]) return false; return true; }
        public string Key => string.Join(",", E);
    }

    public sealed class Unit : Item
    {
        public string Plural = "";
        public Real Factor = Real.One;
        public Real Offset = Real.Zero;
        public Dim Dim = new Dim();
        public PrefixUse Prefixes = PrefixUse.None;
        public string Definition = "";
        public string System = "";
        public bool IsCurrency;
        public bool IsBase;
        public bool Resolved = true;
        public bool ApproxFactor;
        public bool MixedChain;

        public string Abbrev => Names.Count > 0 ? Names[0] : Name;
        public bool HasOffset => !Offset.IsZero;
    }

    public enum ArgType { Any, Number, Integer, Real, NonNegInt, PositiveInt, Matrix, Vector, Text, Symbol, Boolean, Date, Angle, Unevaluated, Function }

    public sealed class ArgDef
    {
        public string Name;
        public ArgType Type;
        public string Default;
        public ArgDef(string name, ArgType type = ArgType.Any, string def = null) { Name = name; Type = type; Default = def; }
    }

    public delegate Expr FnImpl(Calculator c, List<Expr> args);

    public sealed class MathFunction : Item
    {
        public List<ArgDef> Args = new List<ArgDef>();
        public int MinArgs, MaxArgs;
        public FnImpl Impl;
        public string Formula = "";
        public Expr Parsed;
        public string Condition = "";
        public string Example = "";
        public bool Elementwise;
        public bool HoldAll;
        public HashSet<int> HoldArgs = new HashSet<int>();
        public bool Approximates;
        public bool RawArgs;

        public bool IsUserFormula => Impl == null;
        public string Signature => Names[0] + "(" + string.Join(", ", Args.Select((a, i) => i >= MinArgs ? "[" + a.Name + "]" : a.Name)) + (MaxArgs < 0 ? ", …" : "") + ")";
    }

    public sealed class DataProperty
    {
        public string Name, Title, Unit = "", Description = "";
        public bool IsKey, IsText, Approximate = true, Hidden;
        public DataProperty(string name, string title, string unit = "", bool text = false)
        {
            Name = name; Title = title; Unit = unit; IsText = text;
        }
    }

    public sealed class DataObject
    {
        public Dictionary<string, string> Values = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        public string Get(string p) => Values.TryGetValue(p, out var v) ? v : null;
    }

    public sealed class DataSet : Item
    {
        public List<DataProperty> Properties = new List<DataProperty>();
        public List<DataObject> Objects = new List<DataObject>();
        public string DefaultProperty = "";
        public string Copyright = "";

        public DataProperty Prop(string n) => Properties.FirstOrDefault(p => string.Equals(p.Name, n, StringComparison.OrdinalIgnoreCase) || string.Equals(p.Title, n, StringComparison.OrdinalIgnoreCase));

        public DataObject Find(string key)
        {
            foreach (var o in Objects)
                foreach (var p in Properties.Where(p => p.IsKey))
                {
                    var v = o.Get(p.Name);
                    if (v != null && string.Equals(v, key, StringComparison.OrdinalIgnoreCase)) return o;
                }
            return null;
        }
    }
}
