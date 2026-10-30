using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    // everything that touches the engine runs on one worker thread with a large stack
    public sealed class CalcService : IDisposable
    {
        public readonly Calculator Calc;
        readonly BlockingCollection<Action> queue = new BlockingCollection<Action>();
        readonly Thread worker;
        public volatile bool Busy;

        public CalcService(Calculator c)
        {
            Calc = c;
            worker = new Thread(Loop, 256 * 1024 * 1024) { IsBackground = true, Name = "OrangCalc engine" };
            worker.Start();
        }

        void Loop()
        {
            Calculator.Current = Calc;
            foreach (var a in queue.GetConsumingEnumerable())
            {
                Busy = true;
                try { a(); }
                catch (Exception) { }
                finally { Busy = false; }
            }
        }

        public Task<T> Run<T>(Func<T> f)
        {
            var tcs = new TaskCompletionSource<T>();
            queue.Add(() =>
            {
                try { tcs.SetResult(f()); }
                catch (Exception ex) { tcs.SetException(ex); }
            });
            return tcs.Task;
        }

        public Task Run(Action f) => Run(() => { f(); return true; });

        public void Abort() => Calc.AbortRequested = true;

        public void Dispose() => queue.CompleteAdding();
    }

    // formatted result ready for display, produced on the worker thread
    public sealed class Display
    {
        public string Input = "";
        public string ParsedText = "";
        public List<OutTok> Tokens = new List<OutTok>();
        public List<OutTok> ExactTokens;
        public string Text = "";
        public string Ascii = "";
        public string AsciiNoUnits = "";
        public string ExactText = "";
        public bool Approx;
        public string Error;
        public List<string> Warnings = new List<string>();
        public List<List<string>> Matrix;
        public Expr Value;
        public string Assigned;
        public bool Empty;
        public string Comment = "";

        public static Display From(CalcResult r, Calculator c)
        {
            var d = new Display { Input = r.Input, Error = r.Error, Warnings = r.Warnings, Assigned = r.Assigned, Comment = r.Comment };
            var po = r.Print ?? c.Opts;
            if (r.Parsed != null)
            {
                var pp = c.Opts.Clone();
                pp.OutputBase = 10;
                try { d.ParsedText = Formatter.Format(r.Parsed, pp, out _); } catch (Exception) { d.ParsedText = r.Input; }
            }
            if (r.Error != null) return d;
            if (r.Result == null) { d.Empty = true; return d; }
            d.Value = Funcs.Unhold(r.Result);
            try
            {
                d.Tokens = Formatter.Tokens(r.Result, po, out bool rounded);
                d.Text = string.Concat(d.Tokens.Select(t => t.Text));
                d.Approx = rounded || r.Result.IsApprox;
                d.Ascii = Formatter.Ascii(r.Result, po);
                d.AsciiNoUnits = Formatter.Ascii(Evaluator.StripUnits(Funcs.Unhold(r.Result)), po);
                if (r.Exact == null && c.Opts.Approx == ApproxMode.Dual && Formatter.HasNonTerminating(r.Result) && !r.Result.HasSymbols)
                {
                    var fo = po.Clone(); fo.Fraction = FractionMode.Fraction;
                    d.ExactTokens = Formatter.Tokens(r.Result, fo, out _);
                    d.ExactText = string.Concat(d.ExactTokens.Select(t => t.Text));
                    if (d.ExactText == d.Text) { d.ExactTokens = null; d.ExactText = ""; }
                    else d.Approx = true;
                }
                if (r.Exact != null)
                {
                    d.ExactTokens = Formatter.Tokens(r.Exact, po, out _);
                    d.ExactText = string.Concat(d.ExactTokens.Select(t => t.Text));
                    if (d.ExactText == d.Text) { d.ExactTokens = null; d.ExactText = ""; }
                }
                if (r.Result is MatE m && !m.IsVector && m.RowCount <= 16 && m.ColCount <= 16 && m.RowCount > 1)
                    d.Matrix = m.Rows.Select(row => row.Select(x => Formatter.Format(x, po, out _)).ToList()).ToList();
            }
            catch (CalcException ex) { d.Error = ex.Message; }
            return d;
}   }   }