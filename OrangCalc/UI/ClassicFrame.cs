using System;
using System.Collections.Generic;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Runtime.InteropServices;
using System.Windows.Forms;

namespace OrangCalc.UI
{
    // classic unthemed window frame drawn by windows; only the caption gradient is repainted in the accent color
    public sealed class ClassicFrame : NativeWindow
    {
        const int WM_SETTEXT = 0x000C, WM_NCPAINT = 0x0085, WM_NCACTIVATE = 0x0086, WM_SETICON = 0x0080;
        const int WM_NCUAHDRAWCAPTION = 0x00AE, WM_NCUAHDRAWFRAME = 0x00AF, WM_EXITMENULOOP = 0x0212, WM_SYSCOMMAND = 0x0112;
        const int WM_NCLBUTTONUP = 0x00A2, WM_LBUTTONUP = 0x0202, WM_GETTITLEBARINFOEX = 0x033F;
        const int GWL_STYLE = -16, WS_VISIBLE = 0x10000000;

        static readonly Dictionary<IntPtr, ClassicFrame> live = new Dictionary<IntPtr, ClassicFrame>();

        readonly Form form;
        bool active = true;

        public static void Attach(Form f)
        {
            f.HandleCreated += (s, e) => Hook(f);
            if (f.IsHandleCreated) Hook(f);
        }

        static void Hook(Form f)
        {
            if (f.FormBorderStyle == FormBorderStyle.None || live.ContainsKey(f.Handle)) return;
            new ClassicFrame(f);
        }

        public static void Redraw(Form f)
        {
            if (f.IsHandleCreated && live.TryGetValue(f.Handle, out var cf)) cf.Paint();
        }

        ClassicFrame(Form f)
        {
            form = f;
            AssignHandle(f.Handle);
            live[Handle] = this;
            var h = Handle;
            f.HandleDestroyed += (s, e) => { live.Remove(h); ReleaseHandle(); };
            try
            {
                SetWindowTheme(h, "", "");
                int policy = 2, corners = 1;
                DwmSetWindowAttribute(h, 2, ref policy, 4);
                DwmSetWindowAttribute(h, 33, ref corners, 4);
            }
            catch (Exception) { }
            SetWindowPos(h, IntPtr.Zero, 0, 0, 0, 0, 0x0001 | 0x0002 | 0x0004 | 0x0010 | 0x0020);
        }

        protected override void WndProc(ref Message m)
        {
            switch (m.Msg)
            {
                case WM_NCACTIVATE:
                    active = m.WParam != IntPtr.Zero;
                    break;
                case WM_SETTEXT:
                case WM_SETICON:
                case WM_NCUAHDRAWCAPTION:
                    Hidden(ref m);
                    Paint();
                    return;
            }
            base.WndProc(ref m);
            switch (m.Msg)
            {
                case WM_NCPAINT:
                case WM_NCACTIVATE:
                case WM_NCUAHDRAWFRAME:
                case WM_EXITMENULOOP:
                case WM_SYSCOMMAND:
                case WM_NCLBUTTONUP:
                case WM_LBUTTONUP:
                    Paint();
                    break;
            }
        }

        // lets the default handler run without drawing its own caption
        void Hidden(ref Message m)
        {
            int style = GetWindowLong(Handle, GWL_STYLE);
            bool vis = (style & WS_VISIBLE) != 0;
            if (vis) SetWindowLong(Handle, GWL_STYLE, style & ~WS_VISIBLE);
            base.WndProc(ref m);
            if (vis) SetWindowLong(Handle, GWL_STYLE, style);
        }

        // window and caption rectangles in window coordinates
        bool Measure(out Rectangle win, out Rectangle caption)
        {
            GetWindowRect(Handle, out var wr);
            var o = new POINT();
            ClientToScreen(Handle, ref o);
            win = new Rectangle(wr.L, wr.T, wr.R - wr.L, wr.B - wr.T);
            int edge = o.X - wr.L;
            caption = new Rectangle(edge, edge, win.Width - 2 * edge, o.Y - wr.T - edge - 1);
            return edge > 0 && caption.Height > 6 && form.WindowState != FormWindowState.Minimized;
        }

        // the caption buttons windows drew, so they stay untouched
        List<Rectangle> Buttons(Rectangle win)
        {
            var list = new List<Rectangle>();
            var info = new TITLEBARINFOEX { cbSize = Marshal.SizeOf(typeof(TITLEBARINFOEX)), rgstate = new int[6], rgrect = new RECT[6] };
            SendMessage(Handle, WM_GETTITLEBARINFOEX, IntPtr.Zero, ref info);
            for (int i = 2; i < 6; i++)
            {
                var r = info.rgrect[i];
                if ((info.rgstate[i] & 0x8000) != 0 || r.R <= r.L) continue;
                list.Add(new Rectangle(r.L - win.Left, r.T - win.Top, r.R - r.L, r.B - r.T));
            }
            return list;
        }

        void Paint()
        {
            if (Handle == IntPtr.Zero || !Measure(out var win, out var cap)) return;
            var buttons = Buttons(win);
            IntPtr dc = GetWindowDC(Handle);
            if (dc == IntPtr.Zero) return;
            try
            {
                using (var g = Graphics.FromHdc(dc))
                using (var bmp = new Bitmap(cap.Width, cap.Height, System.Drawing.Imaging.PixelFormat.Format24bppRgb))
                {
                    using (var cg = Graphics.FromImage(bmp)) DrawCaption(cg, cap, buttons);
                    foreach (var b in buttons) g.ExcludeClip(b);
                    g.DrawImageUnscaled(bmp, cap.Location);
                }
            }
            finally { ReleaseDC(Handle, dc); }
        }

        void DrawCaption(Graphics g, Rectangle cap, List<Rectangle> buttons)
        {
            var r = new Rectangle(0, 0, cap.Width, cap.Height);
            var c1 = active ? Theme.CaptionStart : SystemColors.InactiveCaption;
            var c2 = active ? Theme.CaptionEnd : SystemColors.GradientInactiveCaption;
            using (var lg = new LinearGradientBrush(new Rectangle(-1, 0, r.Width + 2, r.Height), c1, c2, LinearGradientMode.Horizontal)) g.FillRectangle(lg, r);
            int x = 2;
            if (form.ShowIcon && form.Icon != null && form.FormBorderStyle != FormBorderStyle.FixedDialog)
            {
                int s = Math.Min(16, r.Height - 2);
                using (var ic = new Icon(form.Icon, s, s)) g.DrawIcon(ic, new Rectangle(x, (r.Height - s) / 2, s, s));
                x += s + 3;
            }
            int right = r.Width - 4;
            foreach (var b in buttons) right = Math.Min(right, b.Left - cap.Left - 4);
            using (var font = new Font(SystemFonts.CaptionFont, FontStyle.Bold))
                TextRenderer.DrawText(g, form.Text, font, new Rectangle(x, 0, Math.Max(0, right - x), r.Height), active ? Theme.CaptionText : SystemColors.InactiveCaptionText,
                    TextFormatFlags.Left | TextFormatFlags.VerticalCenter | TextFormatFlags.SingleLine | TextFormatFlags.EndEllipsis | TextFormatFlags.NoPrefix);
        }

        [StructLayout(LayoutKind.Sequential)] struct RECT { public int L, T, R, B; }
        [StructLayout(LayoutKind.Sequential)] struct POINT { public int X, Y; }

        [StructLayout(LayoutKind.Sequential)]
        struct TITLEBARINFOEX
        {
            public int cbSize;
            public RECT rcTitleBar;
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public int[] rgstate;
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public RECT[] rgrect;
        }

        [DllImport("uxtheme.dll", CharSet = CharSet.Unicode)] static extern int SetWindowTheme(IntPtr h, string app, string list);
        [DllImport("dwmapi.dll")] static extern int DwmSetWindowAttribute(IntPtr h, int attr, ref int value, int size);
        [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
        [DllImport("user32.dll")] static extern IntPtr GetWindowDC(IntPtr h);
        [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr h, IntPtr dc);
        [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h, out RECT r);
        [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr h, ref POINT p);
        [DllImport("user32.dll")] static extern int GetWindowLong(IntPtr h, int index);
        [DllImport("user32.dll")] static extern int SetWindowLong(IntPtr h, int index, int value);
        [DllImport("user32.dll")] static extern IntPtr SendMessage(IntPtr h, int msg, IntPtr w, ref TITLEBARINFOEX info);
}   }