using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.Globalization;
using System.Linq;
using System.Threading.Tasks;
using System.Windows.Forms;
using OrangCalc.Engine;

namespace OrangCalc.UI
{
    public enum PlotStyle { Line, Points, LinePoints, Dots, Bars, Histogram, Steps }
    public enum PlotSource { Function, Vector, PairedMatrix }
    public enum Smoothing { None, Monotonic, CubicSpline, Bezier }
    public enum LegendPlace { None, TopLeft, TopRight, BottomLeft, BottomRight, Below }

    public sealed class PlotSeries
    {
        public string Title = "", Expression = "x", Variable = "x";
        public PlotSource Source;
        public PlotStyle Style;
        public Smoothing Smooth;
        public bool Secondary, ImagParts;
        public List<PointF> Points = new List<PointF>();
        public List<PointF> Imag = new List<PointF>();
        public string Error;
        public Color Color;
        public override string ToString() => (Title.Length > 0 ? Title : Expression) + (Error != null ? "  (error)" : "");
    }

    public sealed class PlotCanvas : Control
    {
        public readonly List<PlotSeries> Series = new List<PlotSeries>();
        public string Title = "", XLabel = "", YLabel = "";
        public bool Grid = true, Border = true, LogX, LogY, Polar;
        public double LogBase = 10;
        public LegendPlace Legend = LegendPlace.TopRight;
        public float LineWidth = 2f;
        public double XMin = -10, XMax = 10, YMin = double.NaN, YMax = double.NaN;
        double vx0, vx1, vy0, vy1, sy0, sy1;
        Point? drag;
        Point mouse = new Point(-1, -1);
        public event EventHandler ViewChanged;

        public static readonly Color[] Palette =
        {
            Color.FromArgb(240, 138, 60), Color.FromArgb(96, 160, 232), Color.FromArgb(110, 196, 130), Color.FromArgb(196, 140, 230),
            Color.FromArgb(232, 206, 90), Color.FromArgb(232, 100, 100), Color.FromArgb(80, 200, 200), Color.FromArgb(200, 200, 200)
        };

        public PlotCanvas()
        {
            SetStyle(ControlStyles.AllPaintingInWmPaint | ControlStyles.OptimizedDoubleBuffer | ControlStyles.UserPaint | ControlStyles.ResizeRedraw | ControlStyles.Selectable, true);
            BackColor = Theme.Panel;
        }

        double Tx(double x) => LogX ? Math.Log(x) / Math.Log(LogBase) : x;
        double Ty(double y) => LogY ? Math.Log(y) / Math.Log(LogBase) : y;
        double Ux(double t) => LogX ? Math.Pow(LogBase, t) : t;
        double Uy(double t) => LogY ? Math.Pow(LogBase, t) : t;

        public void FitView()
        {
            var pts = Series.Where(s => !s.Secondary).SelectMany(s => s.Points.Concat(s.ImagParts ? s.Imag : Enumerable.Empty<PointF>())).Where(p => Ok(p)).ToList();
            var spts = Series.Where(s => s.Secondary).SelectMany(s => s.Points).Where(p => Ok(p)).ToList();
            if (Polar || Series.Any(s => s.Source != PlotSource.Function))
            {
                var all = pts.Concat(spts).ToList();
                if (all.Count > 0) { vx0 = all.Min(p => Tx(p.X)); vx1 = all.Max(p => Tx(p.X)); }
                else { vx0 = Tx(XMin); vx1 = Tx(XMax); }
            }
            else { vx0 = Tx(LogX ? Math.Max(XMin, 1e-12) : XMin); vx1 = Tx(XMax); }
            YBounds(pts, out vy0, out vy1, true);
            YBounds(spts, out sy0, out sy1, false);
            if (Polar && pts.Count > 0)
            {
                double m = Math.Max(Math.Max(Math.Abs(vx0), Math.Abs(vx1)), Math.Max(Math.Abs(vy0), Math.Abs(vy1)));
                vx0 = vy0 = -m; vx1 = vy1 = m;
            }
            if (vx1 - vx0 < 1e-300) { vx0 -= 1; vx1 += 1; }
            Invalidate();
        }

        bool Ok(PointF p) => !float.IsNaN(p.Y) && !float.IsInfinity(p.Y) && !float.IsNaN(p.X) && (!LogY || p.Y > 0) && (!LogX || p.X > 0);

        void YBounds(List<PointF> pts, out double lo, out double hi, bool primary)
        {
            if (pts.Count == 0) { lo = -1; hi = 1; }
            else
            {
                var ys = pts.Select(p => Ty(p.Y)).OrderBy(v => v).ToList();
                int n = ys.Count;
                lo = ys[0]; hi = ys[n - 1];
                if (n > 20)
                {
                    double q1 = ys[(int)(n * 0.02)], q3 = ys[(int)(n * 0.98)];
                    double span = Math.Max(q3 - q1, 1e-9);
                    lo = Math.Max(lo, q1 - span * 0.5); hi = Math.Min(hi, q3 + span * 0.5);
                }
                double pad = (hi - lo) * 0.06;
                if (pad < 1e-12) pad = Math.Max(1, Math.Abs(hi)) * 0.1;
                lo -= pad; hi += pad;
            }
            if (primary)
            {
                if (!double.IsNaN(YMin)) lo = Ty(YMin);
                if (!double.IsNaN(YMax)) hi = Ty(YMax);
            }
        }

        Rectangle PlotArea()
        {
            int left = 64, right = Series.Any(s => s.Secondary) ? 64 : 22, top = Title.Length > 0 ? 38 : 18, bottom = 42 + (Legend == LegendPlace.Below ? 24 : 0);
            if (YLabel.Length > 0) left += 16;
            if (XLabel.Length > 0) bottom += 16;
            return new Rectangle(left, top, Math.Max(10, Width - left - right), Math.Max(10, Height - top - bottom));
        }

        static double NiceStep(double span, int target)
        {
            double raw = span / Math.Max(1, target);
            double mag = Math.Pow(10, Math.Floor(Math.Log10(raw)));
            double n = raw / mag;
            return (n < 1.5 ? 1 : n < 3 ? 2 : n < 7 ? 5 : 10) * mag;
        }

        static string Fmt(double v)
        {
            if (Math.Abs(v) < 1e-12) return "0";
            double a = Math.Abs(v);
            if (a >= 1e6 || a < 1e-3) return v.ToString("0.##E0", CultureInfo.InvariantCulture);
            return v.ToString("0.####", CultureInfo.InvariantCulture);
        }

        protected override void OnPaint(PaintEventArgs e)
        {
            try { Render(e.Graphics, Width, Height, true); }
            catch (Exception ex) { e.Graphics.Clear(BackColor); TextRenderer.DrawText(e.Graphics, "Unable to draw the plot: " + ex.Message, Theme.UiFont, ClientRectangle, Theme.Error, TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter | TextFormatFlags.WordBreak); }
        }

        public void Render(Graphics g, int w, int h, bool interactive)
        {
            g.Clear(BackColor);
            g.SmoothingMode = SmoothingMode.AntiAlias;
            g.TextRenderingHint = System.Drawing.Text.TextRenderingHint.ClearTypeGridFit;
            var area = PlotArea();
            if (Title.Length > 0) TextRenderer.DrawText(g, Title, Theme.HeaderFont, new Rectangle(0, 6, w, 26), Theme.Text, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter);
            if (vx1 <= vx0) FitView();
            double dx = vx1 - vx0, dy = vy1 - vy0, dsy = sy1 - sy0;
            if (dy <= 0) dy = 1;
            if (dsy <= 0) dsy = 1;
            Func<double, float> X = t => (float)(area.Left + (t - vx0) / dx * area.Width);
            Func<double, float> Y = t => (float)(area.Bottom - (t - vy0) / dy * area.Height);
            Func<double, float> SY = t => (float)(area.Bottom - (t - sy0) / dsy * area.Height);

            using (var gridPen = new Pen(Theme.Line))
            using (var axisPen = new Pen(Theme.LineStrong))
            {
                double xs = NiceStep(dx, Math.Max(2, area.Width / 90));
                for (double t = Math.Ceiling(vx0 / xs) * xs; t <= vx1 + xs * 1e-9; t += xs)
                {
                    float px = X(t);
                    if (Grid) g.DrawLine(gridPen, px, area.Top, px, area.Bottom);
                    TextRenderer.DrawText(g, Fmt(LogX ? Ux(t) : t), Theme.SmallFont, new Rectangle((int)px - 40, area.Bottom + 4, 80, 14), Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter);
                }
                double ys = NiceStep(dy, Math.Max(2, area.Height / 60));
                for (double t = Math.Ceiling(vy0 / ys) * ys; t <= vy1 + ys * 1e-9; t += ys)
                {
                    float py = Y(t);
                    if (Grid) g.DrawLine(gridPen, area.Left, py, area.Right, py);
                    TextRenderer.DrawText(g, Fmt(LogY ? Uy(t) : t), Theme.SmallFont, new Rectangle(area.Left - 62, (int)py - 7, 58, 14), Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.Right);
                }
                if (Series.Any(s => s.Secondary))
                {
                    double ss = NiceStep(dsy, Math.Max(2, area.Height / 60));
                    for (double t = Math.Ceiling(sy0 / ss) * ss; t <= sy1 + ss * 1e-9; t += ss)
                        TextRenderer.DrawText(g, Fmt(t), Theme.SmallFont, new Rectangle(area.Right + 4, (int)SY(t) - 7, 58, 14), Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.Left);
                }
                if (vx0 < 0 && vx1 > 0 && !LogX) g.DrawLine(axisPen, X(0), area.Top, X(0), area.Bottom);
                if (vy0 < 0 && vy1 > 0 && !LogY) g.DrawLine(axisPen, area.Left, Y(0), area.Right, Y(0));
                if (Border) g.DrawRectangle(axisPen, area);
                else { g.DrawLine(axisPen, area.Left, area.Bottom, area.Right, area.Bottom); g.DrawLine(axisPen, area.Left, area.Top, area.Left, area.Bottom); }
            }
            if (XLabel.Length > 0) TextRenderer.DrawText(g, XLabel, Theme.UiFont, new Rectangle(area.Left, area.Bottom + 20, area.Width, 18), Theme.TextDim, TextFormatFlags.NoPrefix | TextFormatFlags.HorizontalCenter);
            if (YLabel.Length > 0)
            {
                var st = g.Save();
                g.TranslateTransform(12, area.Top + area.Height / 2);
                g.RotateTransform(-90);
                using (var b = new SolidBrush(Theme.TextDim)) g.DrawString(YLabel, Theme.UiFont, b, -g.MeasureString(YLabel, Theme.UiFont).Width / 2, 0);
                g.Restore(st);
            }

            var clip = g.Clip;
            g.SetClip(area);
            foreach (var s in Series)
            {
                Func<double, float> yf = s.Secondary ? SY : Y;
                DrawSeries(g, s.Points, s, X, yf, s.Color, area, false);
                if (s.ImagParts && s.Imag.Count > 0) DrawSeries(g, s.Imag, s, X, yf, ControlPaint.Light(s.Color, 0.3f), area, true);
            }
            g.Clip = clip;

            if (Legend != LegendPlace.None && Series.Count > 0) DrawLegend(g, area);
            if (interactive && area.Contains(mouse))
            {
                double mx = Ux(vx0 + (mouse.X - area.Left) / (double)area.Width * dx), my = Uy(vy0 + (area.Bottom - mouse.Y) / (double)area.Height * dy);
                TextRenderer.DrawText(g, "x = " + Fmt(mx) + "   y = " + Fmt(my), Theme.SmallFont, new Rectangle(area.Left + 6, area.Top + 4, 260, 14), Theme.TextFaint, TextFormatFlags.NoPrefix | TextFormatFlags.Left);
            }
        }

        void DrawSeries(Graphics g, List<PointF> raw, PlotSeries s, Func<double, float> X, Func<double, float> Y, Color c, Rectangle area, bool dashed)
        {
            var pts = raw.Where(Ok).Select(p => new PointF(X(Tx(p.X)), Y(Ty(p.Y)))).ToList();
            if (pts.Count == 0) return;
            using (var pen = new Pen(c, LineWidth) { LineJoin = LineJoin.Round })
            using (var brush = new SolidBrush(c))
            using (var soft = new SolidBrush(Color.FromArgb(90, c)))
            {
                if (dashed) pen.DashStyle = DashStyle.Dash;
                switch (s.Style)
                {
                    case PlotStyle.Points:
                    case PlotStyle.LinePoints:
                        if (s.Style == PlotStyle.LinePoints) DrawLine(g, pen, raw, X, Y, s.Smooth);
                        float r = Math.Max(2.5f, LineWidth + 1.5f);
                        foreach (var p in pts) g.FillEllipse(brush, p.X - r, p.Y - r, r * 2, r * 2);
                        break;
                    case PlotStyle.Dots:
                        foreach (var p in pts) g.FillRectangle(brush, p.X - 1, p.Y - 1, 2, 2);
                        break;
                    case PlotStyle.Bars:
                    case PlotStyle.Histogram:
                        {
                            float bw = pts.Count > 1 ? Math.Abs(pts[1].X - pts[0].X) * (s.Style == PlotStyle.Bars ? 0.7f : 1f) : 20;
                            float zero = Math.Max(area.Top, Math.Min(area.Bottom, Y(LogY ? vy0 : 0)));
                            foreach (var p in pts)
                            {
                                var rr = new RectangleF(p.X - bw / 2, Math.Min(p.Y, zero), bw, Math.Abs(zero - p.Y));
                                g.FillRectangle(soft, rr);
                                using (var bp = new Pen(c, 1)) g.DrawRectangle(bp, rr.X, rr.Y, rr.Width, rr.Height);
                            }
                            break;
                        }
                    case PlotStyle.Steps:
                        {
                            var st = new List<PointF>();
                            for (int i = 0; i < pts.Count; i++)
                            {
                                if (i > 0) st.Add(new PointF(pts[i].X, pts[i - 1].Y));
                                st.Add(pts[i]);
                            }
                            if (st.Count > 1) g.DrawLines(pen, st.ToArray());
                            break;
                        }
                    default: DrawLine(g, pen, raw, X, Y, s.Smooth); break;
                }
            }
        }

        void DrawLine(Graphics g, Pen pen, List<PointF> raw, Func<double, float> X, Func<double, float> Y, Smoothing sm)
        {
            var seg = new List<PointF>();
            double lastY = double.NaN;
            void Flush()
            {
                if (seg.Count > 1)
                {
                    if (sm == Smoothing.None || seg.Count < 3) g.DrawLines(pen, seg.ToArray());
                    else if (sm == Smoothing.Bezier) g.DrawCurve(pen, seg.ToArray(), 0.5f);
                    else g.DrawCurve(pen, seg.ToArray(), sm == Smoothing.Monotonic ? 0.25f : 0.6f);
                }
                seg.Clear();
            }
            foreach (var p in raw)
            {
                if (!Ok(p)) { Flush(); lastY = double.NaN; continue; }
                double ty = Ty(p.Y);
                if (!double.IsNaN(lastY) && Math.Abs(ty - lastY) > (vy1 - vy0) * 3) Flush();
                var sp = new PointF(X(Tx(p.X)), Y(ty));
                if (Math.Abs(sp.Y) > 1e6) { Flush(); lastY = double.NaN; continue; }
                seg.Add(sp);
                lastY = ty;
            }
            Flush();
        }

        void DrawLegend(Graphics g, Rectangle area)
        {
            var items = Series.Select(s => s.Title.Length > 0 ? s.Title : s.Expression).ToList();
            int lw = items.Max(t => TextRenderer.MeasureText(t, Theme.UiFont).Width) + 36, lh = items.Count * 20 + 8;
            Rectangle box;
            switch (Legend)
            {
                case LegendPlace.TopLeft: box = new Rectangle(area.Left + 8, area.Top + 8, lw, lh); break;
                case LegendPlace.BottomLeft: box = new Rectangle(area.Left + 8, area.Bottom - lh - 8, lw, lh); break;
                case LegendPlace.BottomRight: box = new Rectangle(area.Right - lw - 8, area.Bottom - lh - 8, lw, lh); break;
                case LegendPlace.Below:
                    {
                        int x = area.Left;
                        for (int i = 0; i < items.Count; i++)
                        {
                            using (var p = new Pen(Series[i].Color, 3)) g.DrawLine(p, x, area.Bottom + 58, x + 18, area.Bottom + 58);
                            TextRenderer.DrawText(g, items[i], Theme.UiFont, new Point(x + 22, area.Bottom + 50), Theme.Text);
                            x += TextRenderer.MeasureText(items[i], Theme.UiFont).Width + 40;
                        }
                        return;
                    }
                default: box = new Rectangle(area.Right - lw - 8, area.Top + 8, lw, lh); break;
            }
            using (var b = new SolidBrush(Theme.Raised)) g.FillRectangle(b, box);
            using (var p = new Pen(Theme.Line)) g.DrawRectangle(p, box);
            for (int i = 0; i < items.Count; i++)
            {
                int y = box.Top + 4 + i * 20;
                using (var p = new Pen(Series[i].Color, 3)) g.DrawLine(p, box.Left + 8, y + 10, box.Left + 26, y + 10);
                TextRenderer.DrawText(g, items[i], Theme.UiFont, new Point(box.Left + 30, y + 2), Theme.Text);
            }
        }

        protected override void OnMouseDown(MouseEventArgs e) { Focus(); if (e.Button == MouseButtons.Left) drag = e.Location; base.OnMouseDown(e); }
        protected override void OnMouseUp(MouseEventArgs e) { if (drag != null) ViewChanged?.Invoke(this, EventArgs.Empty); drag = null; base.OnMouseUp(e); }

        protected override void OnMouseMove(MouseEventArgs e)
        {
            mouse = e.Location;
            if (drag != null)
            {
                var a = PlotArea();
                double ddx = (e.X - drag.Value.X) / (double)a.Width * (vx1 - vx0);
                double ddy = (e.Y - drag.Value.Y) / (double)a.Height * (vy1 - vy0);
                double dsy = (e.Y - drag.Value.Y) / (double)a.Height * (sy1 - sy0);
                vx0 -= ddx; vx1 -= ddx; vy0 += ddy; vy1 += ddy; sy0 += dsy; sy1 += dsy;
                drag = e.Location;
            }
            Invalidate();
            base.OnMouseMove(e);
        }

        protected override void OnMouseWheel(MouseEventArgs e)
        {
            var a = PlotArea();
            double f = e.Delta > 0 ? 0.8 : 1.25;
            double cx = vx0 + (e.X - a.Left) / (double)a.Width * (vx1 - vx0);
            double cy = vy0 + (a.Bottom - e.Y) / (double)a.Height * (vy1 - vy0);
            vx0 = cx + (vx0 - cx) * f; vx1 = cx + (vx1 - cx) * f;
            vy0 = cy + (vy0 - cy) * f; vy1 = cy + (vy1 - cy) * f;
            double csy = (sy0 + sy1) / 2;
            sy0 = csy + (sy0 - csy) * f; sy1 = csy + (sy1 - csy) * f;
            Invalidate();
            ViewChanged?.Invoke(this, EventArgs.Empty);
            base.OnMouseWheel(e);
        }

        public double ViewXMin => Ux(vx0);
        public double ViewXMax => Ux(vx1);

        public Bitmap ToBitmap(int w, int h)
        {
            var bmp = new Bitmap(w, h);
            using (var g = Graphics.FromImage(bmp))
            {
                var saveW = Width; var saveH = Height;
                Render(g, w, h, false);
            }
            return bmp;
        }
    }

    public sealed class PlotPage : Panel, IPage
    {
        readonly MainForm main;
        readonly PlotCanvas canvas = new PlotCanvas { Dock = DockStyle.Fill };
        readonly ListBox seriesList;
        readonly TextBox expr, title, variable, xmin, xmax, samples, ymin, ymax, ptitle, xlabel, ylabel, logBase;
        readonly DarkCombo style, smooth, source, axis, legend;
        readonly CheckBox imag, grid, border, logx, logy, polar, useStep;
        readonly NumericUpDown width;
        int colorIndex;

        public PlotPage(MainForm m)
        {
            main = m;
            BackColor = Theme.Window;
            Padding = new Padding(16, 12, 16, 12);
            var header = Ui.Header("Plot");
            header.Dock = DockStyle.Top;

            var side = Ui.ShareWidth(new Card { Dock = DockStyle.Left, Width = 330, Padding = new Padding(12, 10, 12, 10), AutoScroll = true }, 0.36f, 260, 340);
            Theme.Style(side);
            var form = new TableLayoutPanel { Dock = DockStyle.Top, AutoSize = true, ColumnCount = 2, BackColor = Theme.Panel };
            form.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
            form.ColumnStyles.Add(new ColumnStyle(SizeType.Percent, 100));
            int r = 0;
            T Row<T>(string label, T c) where T : Control
            {
                if (label != null) form.Controls.Add(new Label { Text = label, AutoSize = true, ForeColor = Theme.TextDim, Margin = new Padding(0, 7, 8, 2) }, 0, r);
                c.Dock = DockStyle.Fill; c.Margin = new Padding(0, 3, 0, 3);
                form.Controls.Add(c, label == null ? 0 : 1, r);
                if (label == null) form.SetColumnSpan(c, 2);
                r++;
                return c;
            }
            Label Section(string t)
            {
                var l = new Label { Text = t, AutoSize = true, ForeColor = Theme.Accent, Font = Theme.UiBold, Margin = new Padding(0, 12, 0, 4) };
                form.Controls.Add(l, 0, r); form.SetColumnSpan(l, 2); r++;
                return l;
            }
            Section("Data");
            expr = Row("Expression:", Ui.Box("sin(x)"));
            expr.Font = Theme.MonoFont;
            title = Row("Title:", Ui.Box(""));
            source = Row("Type:", Ui.Combo(new[] { "Function", "Vector/matrix", "Paired matrix" }));
            variable = Row("X variable:", Ui.Box("x"));
            style = Row("Style:", Ui.Combo(new[] { "Line", "Points", "Line with points", "Dots", "Boxes/bars", "Histogram", "Steps" }));
            smooth = Row("Smoothing:", Ui.Combo(new[] { "None", "Monotonic", "Natural cubic splines", "Bezier" }));
            axis = Row("Y-axis:", Ui.Combo(new[] { "Primary", "Secondary" }));
            imag = Row(null, Ui.Check("Show real and imaginary parts", false));
            var bRow = Ui.Row(new FlatButton("Add", (s, e) => AddFromForm(), true), new FlatButton("Apply", (s, e) => ApplyToSelected()), new FlatButton("Remove", (s, e) => RemoveSelected()));
            foreach (Control c in bRow.Controls) c.Margin = new Padding(0, 0, 6, 6);
            bRow.WrapContents = true;
            Row(null, bRow);
            seriesList = Row(null, new ListBox { Height = 96, BorderStyle = BorderStyle.None, BackColor = Theme.Raised, ForeColor = Theme.Text, IntegralHeight = false });
            seriesList.SelectedIndexChanged += (s, e) => LoadSelected();
            Section("Function Range");
            xmin = Row("Minimum x value:", Ui.Box("-10"));
            xmax = Row("Maximum x value:", Ui.Box("10"));
            samples = Row("Sampling rate:", Ui.Box("1001"));
            useStep = Row(null, Ui.Check("Use the sampling value as step size", false));
            polar = Row(null, Ui.Check("Polar plot (expression is r(θ), x variable is θ)", false));
            Section("Appearance");
            ptitle = Row("Title:", Ui.Box(""));
            xlabel = Row("X-axis label:", Ui.Box(""));
            ylabel = Row("Y-axis label:", Ui.Box(""));
            grid = Row(null, Ui.Check("Display grid", true));
            border = Row(null, Ui.Check("Display full border", true));
            ymin = Row("Minimum y value:", Ui.Box(""));
            ymax = Row("Maximum y value:", Ui.Box(""));
            logx = Row(null, Ui.Check("Logarithmic x scale", false));
            logy = Row(null, Ui.Check("Logarithmic y scale", false));
            logBase = Row("Log base:", Ui.Box("10"));
            legend = Row("Legend placement:", Ui.Combo(new[] { "None", "Top-left", "Top-right", "Bottom-left", "Bottom-right", "Below" }, 2));
            width = Row("Line width:", Ui.Number(2, 1, 10, 60));
            var actions = Ui.Row(new FlatButton("Plot", (s, e) => Replot(), true), new FlatButton("Reset view", (s, e) => canvas.FitView()), new FlatButton("Save PNG…", (s, e) => SavePng()), new FlatButton("Copy", (s, e) => CopyImage()));
            foreach (Control c in actions.Controls) c.Margin = new Padding(0, 6, 6, 0);
            actions.WrapContents = true;
            actions.Padding = new Padding(0, 4, 0, 0);
            Row(null, actions);
            side.Controls.Add(form);
            foreach (var cb in new[] { grid, border, logx, logy, polar }) cb.CheckedChanged += (s, e) => ApplyAppearance(cb == polar || cb == logx || cb == logy);
            foreach (var cb in new DarkCombo[] { legend }) cb.SelectedIndexChanged += (s, e) => ApplyAppearance(false);
            width.ValueChanged += (s, e) => ApplyAppearance(false);
            expr.KeyDown += (s, e) => { if (e.KeyCode == Keys.Enter) { e.SuppressKeyPress = true; if (seriesList.SelectedIndex >= 0) ApplyToSelected(); else AddFromForm(); } };

            var canvasWrap = new Card { Dock = DockStyle.Fill, Padding = new Padding(1) };
            canvasWrap.Controls.Add(canvas);
            var gap = new Panel { Dock = DockStyle.Left, Width = 12, BackColor = Theme.Window };
            Controls.Add(canvasWrap);
            Controls.Add(gap);
            Controls.Add(side);
            Controls.Add(header);
            canvas.ViewChanged += (s, e) =>
            {
                if (canvas.Polar || !canvas.Series.Any(x => x.Source == PlotSource.Function)) return;
                xmin.Text = canvas.ViewXMin.ToString("G6", CultureInfo.InvariantCulture);
                xmax.Text = canvas.ViewXMax.ToString("G6", CultureInfo.InvariantCulture);
                Recompute(false);
            };
        }

        static double ParseD(string s, double def)
        {
            s = s.Trim().Replace("−", "-");
            if (s.Length == 0) return def;
            if (double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out var v)) return v;
            try { var e = Calculator.Current.EvalText(s); if (e is NumE n && n.V.IsReal) return n.V.Re.ToDouble(); } catch (CalcException) { }
            return def;
        }

        PlotSeries FromForm(PlotSeries s = null)
        {
            s = s ?? new PlotSeries { Color = PlotCanvas.Palette[colorIndex++ % PlotCanvas.Palette.Length] };
            s.Expression = expr.Text.Trim();
            s.Title = title.Text.Trim();
            s.Source = (PlotSource)source.SelectedIndex;
            s.Variable = variable.Text.Trim().Length > 0 ? variable.Text.Trim() : "x";
            s.Style = (PlotStyle)style.SelectedIndex;
            s.Smooth = (Smoothing)smooth.SelectedIndex;
            s.Secondary = axis.SelectedIndex == 1;
            s.ImagParts = imag.Checked;
            return s;
        }

        void AddFromForm()
        {
            if (expr.Text.Trim().Length == 0) return;
            var s = FromForm();
            canvas.Series.Add(s);
            RefreshList();
            seriesList.SelectedIndex = -1;
            Recompute(true);
        }

        public void AddSeries(string e, double a, double b)
        {
            expr.Text = e;
            title.Text = "";
            source.SelectedIndex = 0;
            xmin.Text = a.ToString(CultureInfo.InvariantCulture);
            xmax.Text = b.ToString(CultureInfo.InvariantCulture);
            canvas.Series.Clear();
            colorIndex = 0;
            AddFromForm();
        }

        void ApplyToSelected()
        {
            int i = seriesList.SelectedIndex;
            if (i < 0 || i >= canvas.Series.Count) { AddFromForm(); return; }
            FromForm(canvas.Series[i]);
            RefreshList();
            seriesList.SelectedIndex = i;
            Recompute(true);
        }

        void RemoveSelected()
        {
            int i = seriesList.SelectedIndex;
            if (i < 0 || i >= canvas.Series.Count) return;
            canvas.Series.RemoveAt(i);
            RefreshList();
            canvas.FitView();
        }

        void LoadSelected()
        {
            int i = seriesList.SelectedIndex;
            if (i < 0 || i >= canvas.Series.Count) return;
            var s = canvas.Series[i];
            expr.Text = s.Expression; title.Text = s.Title; source.SelectedIndex = (int)s.Source; variable.Text = s.Variable;
            style.SelectedIndex = (int)s.Style; smooth.SelectedIndex = (int)s.Smooth; axis.SelectedIndex = s.Secondary ? 1 : 0; imag.Checked = s.ImagParts;
        }

        void RefreshList()
        {
            seriesList.Items.Clear();
            foreach (var s in canvas.Series) seriesList.Items.Add(s);
        }

        void ApplyAppearance(bool refit)
        {
            canvas.Title = ptitle.Text; canvas.XLabel = xlabel.Text; canvas.YLabel = ylabel.Text;
            canvas.Grid = grid.Checked; canvas.Border = border.Checked;
            canvas.LogX = logx.Checked; canvas.LogY = logy.Checked; canvas.Polar = polar.Checked;
            canvas.LogBase = Math.Max(1.0001, ParseD(logBase.Text, 10));
            canvas.Legend = (LegendPlace)legend.SelectedIndex;
            canvas.LineWidth = (float)width.Value;
            canvas.YMin = ymin.Text.Trim().Length > 0 ? ParseD(ymin.Text, double.NaN) : double.NaN;
            canvas.YMax = ymax.Text.Trim().Length > 0 ? ParseD(ymax.Text, double.NaN) : double.NaN;
            if (refit) Recompute(true); else canvas.Invalidate();
        }

        void Replot()
        {
            ApplyAppearance(false);
            if (canvas.Series.Count == 0) AddFromForm(); else Recompute(true);
        }

        void Recompute(bool fit)
        {
            ApplyAppearanceNoRefit();
            double a = ParseD(xmin.Text, -10), b = ParseD(xmax.Text, 10);
            if (b <= a) { var t = a; a = b; b = t; if (a == b) b = a + 1; }
            double sv = ParseD(samples.Text, 1001);
            int n = useStep.Checked ? (int)Math.Min(200000, Math.Max(2, (b - a) / Math.Max(1e-12, sv) + 1)) : (int)Math.Max(2, Math.Min(200000, sv));
            canvas.XMin = a; canvas.XMax = b;
            bool isPolar = polar.Checked, lx = logx.Checked;
            var list = canvas.Series.ToList();
            var c = main.Calc;
            main.Svc.Run(() => list.Select(s => Compute(c, s, a, b, n, isPolar, lx)).ToList()).ContinueWith(t => BeginInvoke((Action)(() =>
            {
                if (t.IsFaulted) return;
                for (int i = 0; i < list.Count; i++) { list[i].Points = t.Result[i].Item1; list[i].Imag = t.Result[i].Item2; list[i].Error = t.Result[i].Item3; }
                RefreshList();
                if (fit) canvas.FitView(); else canvas.Invalidate();
                var errs = list.Where(s => s.Error != null).Select(s => s.Expression + ": " + s.Error).ToList();
                if (errs.Count > 0 && fit) Ui.Message(main, string.Join("\n", errs), "Plot");
            })));
        }

        void ApplyAppearanceNoRefit()
        {
            canvas.Title = ptitle.Text; canvas.XLabel = xlabel.Text; canvas.YLabel = ylabel.Text;
            canvas.Grid = grid.Checked; canvas.Border = border.Checked; canvas.LogX = logx.Checked; canvas.LogY = logy.Checked; canvas.Polar = polar.Checked;
            canvas.Legend = (LegendPlace)legend.SelectedIndex; canvas.LineWidth = (float)width.Value;
        }

        static (List<PointF>, List<PointF>, string) Compute(Calculator c, PlotSeries src, double a, double b, int n, bool polar, bool logX)
        {
            var s = new PlotSeries { Expression = src.Expression, Variable = src.Variable, Source = src.Source, ImagParts = src.ImagParts };
            try
            {
                var parser = new Parser(s.Expression, c);
                parser.LocalSymbols.Add(s.Variable);
                var e = parser.ParseAll();
                if (e == null) return (s.Points, s.Imag, "empty expression");
                if (s.Source == PlotSource.Function)
                {
                    var saveExact = c.ExactPhase;
                    c.ExactPhase = true;
                    Expr pre;
                    try { pre = c.Ev.Eval(e); } finally { c.ExactPhase = saveExact; }
                    for (int i = 0; i < n; i++)
                    {
                        c.CheckAbort();
                        double x = logX && a > 0 ? Math.Exp(Math.Log(a) + (Math.Log(b) - Math.Log(a)) * i / (n - 1)) : a + (b - a) * i / (n - 1);
                        var z = c.Ev.EvalComplex(pre, s.Variable, x);
                        double y = Math.Abs(z.Imaginary) < 1e-12 * Math.Max(1, Math.Abs(z.Real)) || s.ImagParts ? z.Real : double.NaN;
                        if (polar) s.Points.Add(new PointF((float)(y * Math.Cos(x)), (float)(y * Math.Sin(x))));
                        else
                        {
                            s.Points.Add(new PointF((float)x, (float)y));
                            if (s.ImagParts) s.Imag.Add(new PointF((float)x, (float)z.Imaginary));
                        }
                    }
                }
                else
                {
                    var v = c.Run(e);
                    if (!(v is MatE m)) return (s.Points, s.Imag, "not a vector or matrix");
                    if (s.Source == PlotSource.PairedMatrix)
                    {
                        if (m.ColCount == 2 && m.RowCount >= 1 && !m.IsVector)
                            foreach (var row in m.Rows) s.Points.Add(new PointF((float)D(row[0]), (float)D(row[1])));
                        else if (m.RowCount == 2)
                            for (int j = 0; j < m.ColCount; j++) s.Points.Add(new PointF((float)D(m.Rows[0][j]), (float)D(m.Rows[1][j])));
                        else s.Error = "a paired matrix needs two rows or two columns";
                    }
                    else
                    {
                        var items = m.All.ToList();
                        for (int j = 0; j < items.Count; j++) s.Points.Add(new PointF(j + 1, (float)D(items[j])));
                    }
                }
            }
            catch (CalcException ex) { s.Error = ex.Message; }
            return (s.Points, s.Imag, s.Error);
        }

        static double D(Expr e)
        {
            if (Units.TryQuantity(e, out var v, out _, out var rest) && rest.IsOne && v.IsReal) return v.Re.ToDouble();
            return double.NaN;
        }

        void SavePng()
        {
            using (var d = new SaveFileDialog { Filter = "PNG image (*.png)|*.png", DefaultExt = "png", FileName = "plot.png" })
            {
                if (d.ShowDialog(main) != DialogResult.OK) return;
                using (var bmp = canvas.ToBitmap(Math.Max(800, canvas.Width), Math.Max(500, canvas.Height))) bmp.Save(d.FileName, ImageFormat.Png);
            }
        }

        void CopyImage()
        {
            using (var bmp = canvas.ToBitmap(canvas.Width, canvas.Height))
                try { Clipboard.SetImage(bmp); } catch (Exception) { }
        }

        public void Activated() { }
        public void DefinitionsChanged() { }
        public void OptionsChanged() { }
    }
}
