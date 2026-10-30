using System;
using System.Collections.Generic;
using System.Drawing;
using System.Linq;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public sealed class ResultView : Control
    {
        List<OutTok> tokens = new List<OutTok>();
        List<OutTok> exactTokens;
        List<List<string>> matrixCells;
        string parsed = "";
        string message = "";
        bool isError, approx, busy;
        public bool Colorize = true;
        public Font ResultFont = Theme.ResultFont;

        public ResultView()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw, true);
            BackColor = Theme.Window;
            Cursor = Cursors.IBeam;
        }

        public void ShowResult(string parsedText, List<OutTok> result, bool isApprox, List<OutTok> exact, List<List<string>> matrix)
        {
            parsed = parsedText ?? ""; tokens = result ?? new List<OutTok>(); approx = isApprox; exactTokens = exact; matrixCells = matrix;
            isError = false; message = ""; busy = false;
            Invalidate();
        }

        public void ShowError(string parsedText, string error)
        {
            parsed = parsedText ?? ""; message = error; isError = true; tokens = new List<OutTok>(); exactTokens = null; matrixCells = null; busy = false;
            Invalidate();
        }

        public void ShowBusy(bool b) { busy = b; Invalidate(); }

        public void Clear()
        {
            parsed = ""; message = ""; isError = false; tokens = new List<OutTok>(); exactTokens = null; matrixCells = null; busy = false;
            Invalidate();
        }

        public string PlainText => string.Concat(tokens.Select(t => t.Text));

        protected override void OnPaint(PaintEventArgs e)
        {
            var g = e.Graphics;
            g.Clear(BackColor);
            g.TextRenderingHint = System.Drawing.Text.TextRenderingHint.ClearTypeGridFit;
            int pad = 14;
            var top = new Rectangle(pad, 6, Width - pad * 2, 20);
            if (parsed.Length > 0)
                TextRenderer.DrawText(g, parsed, Theme.UiFont, top, Theme.TextFaint, TextFormatFlags.Right | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis | TextFormatFlags.NoPrefix);
            var area = new Rectangle(pad, 28, Width - pad * 2, Height - 34);
            if (busy)
            {
                TextRenderer.DrawText(g, "Calculating…", Theme.UiFont, area, Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.Right | TextFormatFlags.VerticalCenter);
                return;
            }
            if (isError)
            {
                TextRenderer.DrawText(g, message, Theme.HistoryFont, area, Theme.Error, TextFormatFlags.Right | TextFormatFlags.VerticalCenter | TextFormatFlags.WordBreak | TextFormatFlags.NoPrefix);
                return;
            }
            if (matrixCells != null) { DrawMatrix(g, area); return; }
            if (tokens.Count == 0) return;
            var lines = new List<List<OutTok>>();
            if (exactTokens != null)
            {
                var l1 = new List<OutTok> { new OutTok("= ", TokType.Operator) };
                l1.AddRange(exactTokens);
                lines.Add(l1);
                var l2 = new List<OutTok> { new OutTok("≈ ", TokType.Operator) };
                l2.AddRange(tokens);
                lines.Add(l2);
            }
            else
            {
                var l = new List<OutTok> { new OutTok(approx ? "≈ " : "= ", TokType.Operator) };
                l.AddRange(tokens);
                lines.Add(l);
            }
            var font = ResultFont;
            float size = font.Size;
            List<List<OutTok>> wrapped = null;
            Font use = font;
            for (int attempt = 0; attempt < 8; attempt++)
            {
                use = attempt == 0 ? font : new Font(font.FontFamily, size, font.Style);
                wrapped = new List<List<OutTok>>();
                foreach (var l in lines) wrapped.AddRange(Wrap(g, l, use, area.Width));
                int h = wrapped.Count * use.Height;
                if (h <= area.Height || size < 9) break;
                size *= 0.85f;
            }
            int lineH = use.Height;
            int total = wrapped.Count * lineH;
            int y = area.Top + Math.Max(0, (area.Height - total) / 2);
            foreach (var line in wrapped)
            {
                int w = line.Sum(t => Measure(g, t.Text, use));
                int x = area.Right - w;
                foreach (var t in line)
                {
                    var c = Colorize ? Theme.TokenColor(t.Type) : Theme.Text;
                    if (t.Text == "= " || t.Text == "≈ ") c = Theme.TextFaint;
                    TextRenderer.DrawText(g, t.Text, use, new Point(x, y), c, TextFormatFlags.NoPadding | TextFormatFlags.NoPrefix);
                    x += Measure(g, t.Text, use);
                }
                y += lineH;
                if (y > Height) break;
            }
            if (use != font) use.Dispose();
        }

        static int Measure(Graphics g, string s, Font f) => s.Length == 0 ? 0 : TextRenderer.MeasureText(g, s, f, new Size(int.MaxValue, int.MaxValue), TextFormatFlags.NoPadding | TextFormatFlags.NoPrefix).Width;

        static List<List<OutTok>> Wrap(Graphics g, List<OutTok> line, Font f, int width)
        {
            var res = new List<List<OutTok>>();
            var cur = new List<OutTok>();
            int w = 0;
            foreach (var t in line)
            {
                int tw = Measure(g, t.Text, f);
                if (w + tw > width && cur.Count > 0)
                {
                    res.Add(cur); cur = new List<OutTok>(); w = 0;
                }
                if (tw > width && t.Text.Length > 1)
                {
                    var s = t.Text;
                    while (s.Length > 0)
                    {
                        int n = s.Length;
                        while (n > 1 && Measure(g, s.Substring(0, n), f) > width - w) n--;
                        cur.Add(new OutTok(s.Substring(0, n), t.Type));
                        s = s.Substring(n);
                        if (s.Length > 0) { res.Add(cur); cur = new List<OutTok>(); w = 0; }
                        else w += Measure(g, cur.Last().Text, f);
                    }
                    continue;
                }
                cur.Add(t); w += tw;
            }
            if (cur.Count > 0) res.Add(cur);
            return res;
        }

        void DrawMatrix(Graphics g, Rectangle area)
        {
            int rows = matrixCells.Count, cols = matrixCells.Max(r => r.Count);
            var font = Theme.HistoryFont;
            var colW = new int[cols];
            for (int c = 0; c < cols; c++) colW[c] = matrixCells.Max(r => c < r.Count ? Measure(g, r[c], font) : 0) + 18;
            int rowH = font.Height + 6;
            int totalW = colW.Sum() + 16, totalH = rows * rowH;
            float scale = 1;
            if (totalH > area.Height || totalW > area.Width) scale = Math.Min((float)area.Height / totalH, (float)area.Width / totalW);
            if (scale < 0.55f) scale = 0.55f;
            Font use = scale < 1 ? new Font(font.FontFamily, font.Size * scale) : font;
            if (scale < 1)
            {
                for (int c = 0; c < cols; c++) colW[c] = matrixCells.Max(r => c < r.Count ? Measure(g, r[c], use) : 0) + 14;
                rowH = use.Height + 4;
                totalW = colW.Sum() + 16; totalH = rows * rowH;
            }
            int x0 = area.Right - totalW, y0 = area.Top + Math.Max(0, (area.Height - totalH) / 2);
            using (var p = new Pen(Theme.TextDim, 1.5f))
            {
                g.DrawLines(p, new[] { new Point(x0 + 6, y0), new Point(x0, y0), new Point(x0, y0 + totalH), new Point(x0 + 6, y0 + totalH) });
                int xr = x0 + totalW;
                g.DrawLines(p, new[] { new Point(xr - 6, y0), new Point(xr, y0), new Point(xr, y0 + totalH), new Point(xr - 6, y0 + totalH) });
            }
            for (int r = 0; r < rows; r++)
            {
                int x = x0 + 8;
                for (int c = 0; c < cols; c++)
                {
                    var s = c < matrixCells[r].Count ? matrixCells[r][c] : "";
                    TextRenderer.DrawText(g, s, use, new Rectangle(x, y0 + r * rowH, colW[c], rowH), Theme.TokNumber, TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.NoPrefix);
                    x += colW[c];
                }
            }
            if (use != font) use.Dispose();
}   }   }