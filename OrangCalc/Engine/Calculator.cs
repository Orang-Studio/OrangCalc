using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using System.Threading;

namespace OrangCalc.Engine
{
    public sealed class CalcResult
    {
        public string Input = "";
        public Expr Parsed;
        public Expr Result;
        public Expr Exact;
        public string Error;
        public List<string> Warnings = new List<string>();
        public CalcOptions Print;
        public string Assigned;
        public bool IsApprox => Result != null && Result.IsApprox;
        public string Comment = "";
    }

    public sealed partial class Calculator
    {
        [ThreadStatic] static Calculator current;
        public static Calculator Current { get => current; set => current = value; }

        public CalcOptions Opts = new CalcOptions();
        public bool ExactPhase = true;
        public bool DistributeNumbers = true;
        public readonly Evaluator Ev;
        public volatile bool AbortRequested;
        public bool Preview;
        public DateTime Deadline = DateTime.MaxValue;

        public readonly List<MathFunction> Functions = new List<MathFunction>();
        public readonly List<Variable> Variables = new List<Variable>();
        public readonly List<Unit> Units = new List<Unit>();
        public readonly List<DataSet> DataSets = new List<DataSet>();
        readonly Dictionary<string, MathFunction> fnByName = new Dictionary<string, MathFunction>();
        readonly Dictionary<string, MathFunction> fnByLower = new Dictionary<string, MathFunction>();
        readonly Dictionary<string, Variable> varByName = new Dictionary<string, Variable>();
        readonly Dictionary<string, Unit> unitByName = new Dictionary<string, Unit>();

        public readonly List<Expr> Answers = new List<Expr>();
        public Expr Memory;
        public event Action DefinitionsChanged;

        public Calculator()
        {
            Current = this;
            Ev = new Evaluator(this);
            Funcs.Register(this);
            Constants.Register(this);
            Engine.Units.Load(this);
            Data.Register(this);
            ApplyPrecision();
        }

        public void NotifyDefinitionsChanged() => DefinitionsChanged?.Invoke();

        public void ApplyPrecision()
        {
            int bits = (int)Math.Ceiling((Opts.Precision + 6) * 3.3219280948873626) + 16;
            if (bits != Real.Bits)
            {
                Real.Bits = bits;
                var save = Current; Current = this;
                foreach (var u in Units.Where(u => !u.IsBase && !u.IsCurrency && u.ApproxFactor && u.Definition.Contains("pi"))) Engine.Units.Resolve(this, u);
                Current = save;
            }
        }

        public void AddFunction(MathFunction f)
        {
            if (string.IsNullOrEmpty(f.Name)) f.Name = f.Names[0];
            if (f.Names.Count == 0) f.Names.Add(f.Name);
            var old = Functions.FirstOrDefault(x => x.Name == f.Name);
            if (old != null) RemoveFunction(old);
            Functions.Add(f);
            foreach (var n in f.Names)
            {
                fnByName[n] = f;
                var l = n.ToLowerInvariant();
                if (!fnByLower.ContainsKey(l)) fnByLower[l] = f;
            }
        }

        public void RemoveFunction(MathFunction f)
        {
            Functions.Remove(f);
            foreach (var n in f.Names)
            {
                if (fnByName.TryGetValue(n, out var x) && x == f) fnByName.Remove(n);
                var l = n.ToLowerInvariant();
                if (fnByLower.TryGetValue(l, out var y) && y == f) fnByLower.Remove(l);
            }
        }

        public void AddVariable(Variable v)
        {
            if (string.IsNullOrEmpty(v.Name)) v.Name = v.Names[0];
            if (v.Names.Count == 0) v.Names.Add(v.Name);
            var old = Variables.FirstOrDefault(x => x.Name == v.Name);
            if (old != null) RemoveVariable(old);
            Variables.Add(v);
            foreach (var n in v.Names) varByName[n] = v;
        }

        public void RemoveVariable(Variable v)
        {
            Variables.Remove(v);
            foreach (var n in v.Names) if (varByName.TryGetValue(n, out var x) && x == v) varByName.Remove(n);
        }

        public void AddUnit(Unit u)
        {
            var old = Units.FirstOrDefault(x => x.Name == u.Name);
            if (old != null) RemoveUnit(old);
            Units.Add(u);
            foreach (var n in u.Names) unitByName[n] = u;
            if (!string.IsNullOrEmpty(u.Plural)) unitByName[u.Plural] = u;
        }

        public void RemoveUnit(Unit u)
        {
            Units.Remove(u);
            foreach (var n in u.Names.Concat(new[] { u.Plural })) if (n != null && unitByName.TryGetValue(n, out var x) && x == u) unitByName.Remove(n);
        }

        public MathFunction Fn(string name)
        {
            if (fnByName.TryGetValue(name, out var f)) return f;
            throw new CalcException("Missing function " + name);
        }

        public MathFunction FindFunction(string name)
        {
            if (!Opts.FunctionsEnabled && !name.StartsWith("_")) return null;
            if (fnByName.TryGetValue(name, out var f) && f.Active) return f;
            if (fnByLower.TryGetValue(name.ToLowerInvariant(), out f) && f.Active && !f.IsUser) return f;
            return null;
        }

        public Variable FindVariable(string name)
        {
            if (varByName.TryGetValue(name, out var v) && v.Active)
            {
                if (!Opts.VariablesEnabled && !v.IsUnknown && !v.Name.StartsWith("ans")) return null;
                return v;
            }
            if (name == "j" && Opts.UseJ) return varByName["i"];
            return null;
        }

        public Unit FindUnit(string name, out Prefix prefix, bool allowPrefix = true)
        {
            prefix = null;
            if (!Opts.UnitsEnabled && !name.StartsWith("_")) return null;
            name = name.Replace('μ', 'µ');
            if (unitByName.TryGetValue(name, out var u) && u.Active) return u;
            if (!allowPrefix) return null;
            foreach (var p in Engine.Units.Prefixes.OrderByDescending(x => x.Name.Length))
            {
                if (name.Length > p.Name.Length && name.StartsWith(p.Name, StringComparison.Ordinal))
                {
                    var rest = name.Substring(p.Name.Length);
                    if (unitByName.TryGetValue(rest, out u) && u.Active && u.Names.Contains(rest) == false && PrefixAllowed(u, p)) { prefix = p; return u; }
                    if (unitByName.TryGetValue(rest, out u) && u.Active && PrefixAllowed(u, p) && rest != u.Abbrev) { prefix = p; return u; }
                }
            }
            foreach (var p in Engine.Units.Prefixes.OrderByDescending(x => x.Symbol.Length))
            {
                if (name.Length > p.Symbol.Length && name.StartsWith(p.Symbol, StringComparison.Ordinal))
                {
                    var rest = name.Substring(p.Symbol.Length);
                    if (unitByName.TryGetValue(rest, out u) && u.Active && PrefixAllowed(u, p) && u.Names.Count > 0 && (u.Abbrev == rest || u.Names.IndexOf(rest) < 2 && rest.Length <= 3)) { prefix = p; return u; }
                }
            }
            if (name.StartsWith("u") && name.Length > 1)
            {
                var rest = name.Substring(1);
                if (unitByName.TryGetValue(rest, out u) && u.Active && u.Prefixes != PrefixUse.None && u.Abbrev == rest) { prefix = Engine.Units.PrefixByName("micro"); return u; }
            }
            return null;
        }

        static bool PrefixAllowed(Unit u, Prefix p)
        {
            if (u.Prefixes == PrefixUse.None) return false;
            if (p.Binary) return u.Prefixes == PrefixUse.Binary;
            return true;
        }

        public DataSet FindDataSet(string name) => DataSets.FirstOrDefault(d => d.Names.Contains(name));

        public Expr VarRef(Variable v)
        {
            if (v.IsUnknown) return new SymE(v.Name);
            return new VarE(v);
        }

        public Variable UnknownFor(string name) => varByName.TryGetValue(name, out var v) && v.IsUnknown ? v : null;

        public void CheckAbort()
        {
            if (AbortRequested) throw new CalcException("Calculation aborted");
            if (DateTime.UtcNow > Deadline) throw new CalcException("Calculation timed out");
        }

        AssumeSign SignOf(SymE s)
        {
            var u = UnknownFor(s.Name);
            return u != null ? u.ASign : Opts.DefaultSign;
        }

        AssumeType TypeOf(SymE s)
        {
            var u = UnknownFor(s.Name);
            return u != null ? u.AType : Opts.DefaultType;
        }

        public bool IsKnownReal(Expr e)
        {
            switch (e)
            {
                case NumE n: return n.V.IsReal;
                case SymE s: return TypeOf(s) != AssumeType.Number;
                case VarE v: return v.V.Name != "i";
                case UnitE _: return true;
                case AddE a: return a.Terms.All(IsKnownReal);
                case MulE m: return m.Factors.All(IsKnownReal);
                case PowE p: return IsKnownReal(p.B) && (p.X is NumE x && x.V.IsInt || IsKnownPositive(p.B) && IsKnownReal(p.X));
                case CallE c: return c.F.Name == "abs" || (new[] { "sin", "cos", "tan", "exp", "atan", "sinh", "cosh", "tanh" }.Contains(c.F.Name) && c.Args.All(IsKnownReal));
            }
            return false;
        }

        public bool IsKnownPositive(Expr e)
        {
            switch (e)
            {
                case NumE n: return n.V.IsReal && n.V.Re.Sign > 0;
                case SymE s: return SignOf(s) == AssumeSign.Positive;
                case VarE v: return new[] { "pi", "e", "euler", "catalan", "golden", "apery" }.Contains(v.V.Name) || (v.V.IsConstant && v.V.Name != "i");
                case UnitE _: return true;
                case MulE m: return m.Factors.All(IsKnownPositive);
                case AddE a: return a.Terms.All(IsKnownPositive);
                case PowE p: return IsKnownPositive(p.B) && IsKnownReal(p.X);
                case CallE c: return c.F.Name == "exp" && IsKnownReal(c.Args[0]);
            }
            return false;
        }

        public bool IsKnownNonNegative(Expr e)
        {
            if (IsKnownPositive(e)) return true;
            switch (e)
            {
                case NumE n: return n.V.IsReal && n.V.Re.Sign >= 0;
                case SymE s: var sg = SignOf(s); return sg == AssumeSign.Positive || sg == AssumeSign.NonNegative;
                case PowE p: return p.X is NumE x && x.V.IsExactInt && x.V.Re.N.IsEven && IsKnownReal(p.B);
                case CallE c: return c.F.Name == "abs" || c.F.Name == "sqrt";
                case MulE m: return m.Factors.All(IsKnownNonNegative);
                case AddE a: return a.Terms.All(IsKnownNonNegative);
            }
            return false;
        }

        public bool IsKnownInteger(Expr e)
        {
            if (e is NumE n) return n.V.IsInt;
            if (e is SymE s) return TypeOf(s) == AssumeType.Integer || TypeOf(s) == AssumeType.Boolean;
            return false;
        }

        // top level: assignment, conversion, substitution, evaluation, formatting choices
        public CalcResult Calculate(string input, int timeoutMs = 0, bool preview = false)
        {
            Current = this;
            AbortRequested = false;
            Preview = preview;
            Deadline = timeoutMs > 0 ? DateTime.UtcNow.AddMilliseconds(timeoutMs) : DateTime.MaxValue;
            var res = new CalcResult { Input = input, Print = Opts.Clone() };
            ApplyPrecision();
            try
            {
                string text = input.Trim();
                int hash = FindTopLevel(text, "#");
                if (hash >= 0) { res.Comment = text.Substring(hash + 1).Trim(); text = text.Substring(0, hash).Trim(); }
                if (text.Length == 0) return res;

                var asg = Regex.Match(text, @"^\s*([\p{L}_][\p{L}\p{Nd}_]*)\s*(\(([^()]*)\))?\s*:=\s*(.+)$", RegexOptions.Singleline);
                if (asg.Success)
                {
                    if (preview) { res.Result = new StrE("Assign " + asg.Groups[1].Value); return res; }
                    DoAssign(asg, res);
                    return res;
                }

                var targets = new List<string>();
                while (true)
                {
                    int idx = FindConversion(text, out int len);
                    if (idx < 0) break;
                    targets.Insert(0, text.Substring(idx + len).Trim());
                    text = text.Substring(0, idx).Trim();
                }

                string where = null;
                int wi = FindTopLevelWord(text, "where");
                int si = FindTopLevel(text, "/.");
                if (wi >= 0) { where = text.Substring(wi + 5); text = text.Substring(0, wi); }
                else if (si >= 0) { where = text.Substring(si + 2); text = text.Substring(0, si); }

                var warnings = new List<string>();
                var parsed = Parser.ParseText(text, this, warnings);
                res.Warnings.AddRange(warnings);
                if (parsed == null) return res;
                res.Parsed = parsed;

                var subs = new List<(Expr from, Expr to)>();
                if (where != null)
                {
                    foreach (var part in SplitTopLevel(where, Opts.CommaDecimal ? ';' : ','))
                    {
                        var pe = Parser.ParseText(part, this);
                        if (pe is CmpE ce && ce.Op == CmpOp.Eq) subs.Add((ce.L, ce.R));
                        else if (pe is LogicE le && le.Op == LogicOp.And) foreach (var it in le.Items.OfType<CmpE>()) subs.Add((it.L, it.R));
                        else throw new CalcException("Invalid where condition: " + part);
                    }
                }

                Expr result = Run(parsed, subs);
                var print = res.Print;
                Expr exact = null;
                foreach (var t in targets) result = ApplyTarget(result, t, print, ref exact);

                if (Opts.Approx == ApproxMode.Dual && exact == null) exact = lastExact;
                if (exact != null && result != null && exact.Key != result.Key && !exact.IsApprox) res.Exact = exact;
                res.Result = result;
                if (!preview) StoreAnswer(result);
            }
            catch (CalcException ex) { res.Error = ex.Message; }
            catch (OverflowException) { res.Error = "Number overflow"; }
            catch (OutOfMemoryException) { res.Error = "Out of memory"; }
            catch (DivideByZeroException) { res.Error = "Division by zero"; }
            catch (StackOverflowException) { res.Error = "Expression too complex"; }
            catch (InsufficientExecutionStackException) { res.Error = "Expression too deeply nested"; }
            catch (ArgumentException ex) { res.Error = ex.Message; }
            finally { Deadline = DateTime.MaxValue; Preview = false; }
            return res;
        }

        Expr lastExact;

        // evaluation without parsing, used by calculate and tools
        public Expr Run(Expr parsed, List<(Expr from, Expr to)> subs = null)
        {
            Current = this;
            lastExact = null;
            ExactPhase = Opts.Approx != ApproxMode.Approximate;
            Expr r = Ev.Eval(parsed);
            if (subs != null && subs.Count > 0)
            {
                foreach (var s in subs)
                {
                    var from = Ev.Eval(s.from); var to = Ev.Eval(s.to);
                    r = Substitute(r, from, to);
                }
                r = Ev.Eval(r);
            }
            if (r is CmpE eq && eq.Op == CmpOp.Eq)
            {
                var syms = Symbols(eq);
                if (syms.Count == 1)
                {
                    try { r = Solver.SolveForDisplay(this, eq, syms[0]); } catch (CalcException) { }
                }
            }
            r = Structure(r, Opts.Structure);
            if (Opts.Approx != ApproxMode.Exact && ExactPhase)
            {
                if (Opts.Approx == ApproxMode.Dual) lastExact = r;
                ExactPhase = false;
                r = ApproxPass(r);
            }
            if (Opts.Approx == ApproxMode.Approximate) r = ToApprox(r);
            r = AutoConvert(r);
            ExactPhase = true;
            return r;
        }

        Expr ApproxPass(Expr e)
        {
            if (!e.HasSymbols) return Ev.Eval(e);
            if (e is PowE p && p.B is VarE ve && ve.V.Name == "e" && p.X.HasSymbols) return Simp.Pow(p.B, ApproxPass(p.X));
            if (e.Kids.Count == 0) return Ev.Eval(e);
            var kids = e.Kids.Select(ApproxPass).ToList();
            var rebuilt = e.Rebuild(kids);
            switch (rebuilt)
            {
                case AddE a: return Simp.Add(a.Terms);
                case MulE m: return Simp.Mul(m.Factors);
                case PowE pw: return Simp.Pow(pw.B, pw.X);
                case CallE ce: return Ev.EvalCall(ce, true);
                case CmpE cm: return Ev.EvalCmp(cm);
                default: return rebuilt;
            }
        }

        public Expr Structure(Expr r, StructMode mode)
        {
            if (mode == StructMode.Simplify || mode == StructMode.Expand)
            {
                if (r.HasSymbols || mode == StructMode.Expand) r = Simp.Expand(r);
                if (r.HasSymbols) r = Polynomials.CombineFractions(r);
            }
            else if (mode == StructMode.Factorize) r = Polynomials.Factorize(this, r);
            return r;
        }

        public Expr AutoConvert(Expr r)
        {
            if (!r.HasUnits) return r;
            switch (Opts.AutoConvert)
            {
                case UnitConversion.Base: r = Engine.Units.ToBase(r); break;
                case UnitConversion.Optimal:
                case UnitConversion.OptimalSI: r = Engine.Units.Optimal(r); break;
            }
            r = Engine.Units.OptimizePrefix(r, false);
            if (Opts.MixedUnits) r = Engine.Units.ToMixed(r, false);
            return r;
        }

        public static Expr ToApprox(Expr e) => e.Map(x => x is NumE n && !n.V.Approx ? new NumE(n.V.AsApprox()) : x);

        public Expr Substitute(Expr e, Expr from, Expr to) => e.Map(x => x.Same(from) ? to : x);

        public static List<string> Symbols(Expr e)
        {
            var l = new List<string>();
            e.Any(x => { if (x is SymE s && !l.Contains(s.Name)) l.Add(s.Name); return false; });
            return l;
        }

        void StoreAnswer(Expr r)
        {
            if (r == null) return;
            r = Funcs.Unhold(r);
            Answers.Insert(0, r);
            if (Answers.Count > 20) Answers.RemoveAt(Answers.Count - 1);
        }

        void DoAssign(Match m, CalcResult res)
        {
            string name = m.Groups[1].Value;
            string body = m.Groups[4].Value.Trim();
            if (m.Groups[2].Success)
            {
                var argNames = m.Groups[3].Value.Split(',').Select(a => a.Trim()).Where(a => a.Length > 0).ToList();
                string formula = body;
                for (int i = 0; i < argNames.Count; i++)
                    formula = Regex.Replace(formula, @"(?<![\p{L}\p{Nd}_\\])" + Regex.Escape(argNames[i]) + @"(?![\p{L}\p{Nd}_])", "\\" + (char)(i < 3 ? 'x' + i : 'a' + i - 3));
                var f = UserItems.MakeFunction(this, name, formula, argNames);
                AddFunction(f);
                res.Assigned = name + "(" + string.Join(", ", argNames) + ")";
                res.Result = new StrE("Function " + name + "() defined");
                NotifyDefinitionsChanged();
                return;
            }
            var parsed = Parser.ParseText(body, this);
            var value = Run(parsed);
            value = Funcs.Unhold(value);
            var v = new Variable { Name = name, Names = { name }, Value = value, ValueText = Formatter.PlainText(value), IsUser = true, Category = "User variables", Title = name };
            AddVariable(v);
            res.Parsed = parsed;
            res.Result = value;
            res.Assigned = name;
            NotifyDefinitionsChanged();
        }

        Expr ApplyTarget(Expr result, string target, CalcOptions print, ref Expr exact)
        {
            string t = target.Trim();
            string tl = t.ToLowerInvariant();
            Match bm;
            switch (tl)
            {
                case "bin": case "binary": print.OutputBase = 2; return result;
                case "oct": case "octal": print.OutputBase = 8; return result;
                case "dec": case "decimal": print.OutputBase = 10; return result;
                case "hex": case "hexadecimal": print.OutputBase = 16; return result;
                case "duo": case "duodecimal": print.OutputBase = 12; return result;
                case "sexa": case "sexagesimal": print.OutputBase = Bases.Sexagesimal; return result;
                case "sexa2": case "sexa3": print.OutputBase = Bases.Sexagesimal; return result;
                case "latitude": case "latitude2": print.OutputBase = Bases.Latitude; return result;
                case "longitude": case "longitude2": print.OutputBase = Bases.Longitude; return result;
                case "roman": print.OutputBase = Bases.Roman; return result;
                case "time": print.OutputBase = Bases.Time; return result;
                case "bijective": print.OutputBase = Bases.Bijective26; return result;
                case "bcd": print.OutputBase = Bases.Bcd; return result;
                case "unicode": print.OutputBase = Bases.Unicode; return result;
                case "fp16": case "binary16": print.OutputBase = Bases.Fp16; return result;
                case "fp32": case "float": case "binary32": print.OutputBase = Bases.Fp32; return result;
                case "fp64": case "double": case "binary64": print.OutputBase = Bases.Fp64; return result;
                case "fp80": print.OutputBase = Bases.Fp80; return result;
                case "fp128": case "binary128": print.OutputBase = Bases.Fp128; return result;
                case "golden": case "φ": print.OutputBase = Bases.Golden; return result;
                case "supergolden": case "ψ": print.OutputBase = Bases.Supergolden; return result;
                case "pi": case "π": print.OutputBase = Bases.PiBase; return result;
                case "e": print.OutputBase = Bases.EBase; return result;
                case "sqrt(2)": case "√2": case "√(2)": print.OutputBase = Bases.Sqrt2; return result;
                case "sci": case "scientific": print.Display = NumberDisplay.Scientific; return result;
                case "eng": case "engineering": print.Display = NumberDisplay.Engineering; return result;
                case "simple": print.Display = NumberDisplay.Simple; return result;
                case "fraction": case "frac": print.Fraction = FractionMode.Fraction; return ExactAgain(result, ref exact);
                case "mixed": print.Fraction = FractionMode.Combined; return Engine.Units.ToMixed(ExactAgain(result, ref exact), true);
                case "decimals": case "decimal fraction": print.Fraction = FractionMode.Decimal; return result;
                case "percent": case "%": print.Fraction = FractionMode.Percent; return result;
                case "permille": case "‰": print.Fraction = FractionMode.Permille; return result;
                case "permyriad": case "‱": print.Fraction = FractionMode.Permyriad; return result;
                case "factors": case "factor": case "factorize": return Polynomials.Factorize(this, ExactAgain(result, ref exact));
                case "expand": return Simp.Expand(result);
                case "partial fraction": case "partialfraction": case "partfrac": return Polynomials.PartialFractions(this, ExactAgain(result, ref exact));
                case "rectangular": case "cartesian": case "rect": print.Complex = ComplexForm.Rectangular; return result;
                case "exponential": case "exp": print.Complex = ComplexForm.Exponential; return result;
                case "polar": print.Complex = ComplexForm.Polar; return result;
                case "cis": print.Complex = ComplexForm.Cis; return result;
                case "angle": case "phasor": print.Complex = ComplexForm.Angle; return result;
                case "optimal": return Engine.Units.OptimizePrefix(Engine.Units.Optimal(result), true);
                case "base": case "base units": return Engine.Units.ToBase(result);
                case "prefix": return Engine.Units.OptimizePrefix(result, true);
                case "utc": case "gmt": return Dates.ToUtc(result, TimeSpan.Zero);
                case "calendars": return Dates.CalendarsText(result);
                case "bases": return Formatter.AllBasesText(result, print);
                case "approximate": case "approx": return ToApprox(result);
                case "exact": return ExactAgain(result, ref exact);
            }
            if ((bm = Regex.Match(tl, @"^(utc|gmt)\s*([+-])\s*(\d{1,2})(?::?(\d{2}))?$")).Success)
            {
                var off = new TimeSpan(int.Parse(bm.Groups[3].Value), bm.Groups[4].Success ? int.Parse(bm.Groups[4].Value) : 0, 0);
                if (bm.Groups[2].Value == "-") off = -off;
                return Dates.ToUtc(result, off);
            }
            if ((bm = Regex.Match(tl, @"^base\s*(-?\d+)$")).Success) { print.OutputBase = int.Parse(bm.Groups[1].Value); return result; }
            if ((bm = Regex.Match(tl, @"^base\s+(.+)$")).Success) { print.OutputBase = Bases.Custom; print.OutputBaseCustom = bm.Groups[1].Value; return result; }
            if ((bm = Regex.Match(tl, @"^(\d+)\s*(bit|bits)$")).Success) { print.BinaryBits = int.Parse(bm.Groups[1].Value); print.OutputBase = 2; return result; }

            var targetExpr = Parser.ParseText(t, this);
            ExactPhase = true;
            var te = Ev.Eval(targetExpr);
            ExactPhase = false;
            te = Ev.Eval(te);
            ExactPhase = true;
            if (te.HasUnits)
            {
                Simp.SplitCoef(te, out var cf, out var tu);
                bool mixedTarget = tu is UnitE tue && Engine.Units.MixedChains.Any(ch => ch[0] == tue.U.Name) && false;
                var conv = Engine.Units.Convert(result, tu);
                if (!cf.IsOne) conv = Simp.Mul(Simp.Div(conv, tu), new NumE(Cx.One / cf), tu);
                if (Opts.MixedUnits && mixedTarget) conv = Engine.Units.ToMixed(conv, false);
                return conv;
            }
            if (te is NumE tn && tn.V.IsReal && tn.V.Re.IsExactInt && result is NumE) return Simp.Div(result, te);
            throw new CalcException("Unknown conversion target \"" + t + "\"");
        }

        Expr ExactAgain(Expr result, ref Expr exact)
        {
            if (!result.IsApprox) return result;
            if (lastExact != null && !lastExact.IsApprox) { exact = null; return lastExact; }
            return RationalApprox(result);
        }

        // converts approximate numbers into nearby fractions
        public static Expr RationalApprox(Expr e) => e.Map(x =>
        {
            if (x is NumE n && n.V.IsReal && n.V.Approx) return new NumE(new Cx(Funcs.ToFraction(n.V.Re, 1000000)));
            return x;
        });

        static int FindConversion(string s, out int len)
        {
            int depth = 0; bool q = false; char qc = '\0';
            int found = -1; len = 0;
            for (int i = 0; i < s.Length; i++)
            {
                char ch = s[i];
                if (q) { if (ch == qc) q = false; continue; }
                if (ch == '"' || ch == '\'') { q = true; qc = ch; continue; }
                if (ch == '(' || ch == '[') depth++;
                else if (ch == ')' || ch == ']') depth--;
                if (depth != 0) continue;
                if (ch == '→') { found = i; len = 1; }
                else if (ch == '-' && i + 1 < s.Length && s[i + 1] == '>') { found = i; len = 2; }
                else if ((ch == 't' || ch == 'T') && i + 1 < s.Length && (s[i + 1] == 'o' || s[i + 1] == 'O') && (i == 0 || char.IsWhiteSpace(s[i - 1])) && i + 2 < s.Length && char.IsWhiteSpace(s[i + 2]) && i > 0)
                { found = i; len = 2; }
            }
            return found;
        }

        public static int FindTopLevel(string s, string token)
        {
            int depth = 0; bool q = false; char qc = '\0';
            for (int i = 0; i < s.Length; i++)
            {
                char ch = s[i];
                if (q) { if (ch == qc) q = false; continue; }
                if (ch == '"' || ch == '\'') { q = true; qc = ch; continue; }
                if (ch == '(' || ch == '[') depth++;
                else if (ch == ')' || ch == ']') depth--;
                if (depth == 0 && string.CompareOrdinal(s, i, token, 0, token.Length) == 0) return i;
            }
            return -1;
        }

        static int FindTopLevelWord(string s, string word)
        {
            int depth = 0;
            for (int i = 1; i + word.Length <= s.Length; i++)
            {
                char ch = s[i];
                if (ch == '(' || ch == '[') depth++;
                else if (ch == ')' || ch == ']') depth--;
                if (depth != 0 || !char.IsWhiteSpace(s[i - 1])) continue;
                if (string.Compare(s, i, word, 0, word.Length, StringComparison.OrdinalIgnoreCase) != 0) continue;
                if (i + word.Length == s.Length || char.IsWhiteSpace(s[i + word.Length])) return i;
            }
            return -1;
        }

        public static List<string> SplitTopLevel(string s, char sep)
        {
            var res = new List<string>();
            int depth = 0, start = 0;
            for (int i = 0; i < s.Length; i++)
            {
                char ch = s[i];
                if (ch == '(' || ch == '[') depth++;
                else if (ch == ')' || ch == ']') depth--;
                else if (ch == sep && depth == 0) { res.Add(s.Substring(start, i - start)); start = i + 1; }
            }
            res.Add(s.Substring(start));
            return res.Select(x => x.Trim()).Where(x => x.Length > 0).ToList();
        }

        // evaluates a text expression to a number, used by tools and plotting
        public Expr EvalText(string text)
        {
            Current = this;
            var p = Parser.ParseText(text, this);
            if (p == null) throw new CalcException("Empty expression");
            return Run(p);
        }
    }
}
