using System;
using System.Threading;
using System.Windows.Forms;
using OrangCalc.UI;

namespace OrangCalc
{
    static class Program
    {
        static Mutex instance;
        [STAThread]
        static void Main(string[] args)
        {
            Application.EnableVisualStyles();
            Application.SetCompatibleTextRenderingDefault(false);
            Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
            Application.ThreadException += (s, e) => Report(e.Exception);
            AppDomain.CurrentDomain.UnhandledException += (s, e) => Report(e.ExceptionObject as Exception);

            var probe = new AppSettings();
            Storage.LoadSettings(probe, null);
            instance = new Mutex(true, "OrangCalc.SingleInstance", out bool first);
            if (!first && !probe.MultipleInstances)
            {
                NativeActivate();
                return;
            }
            var form = new MainForm();
            if (args.Length > 0) form.Shown += (s, e) => form.InsertIntoExpression(string.Join(" ", args), true);
            Application.Run(form);
            GC.KeepAlive(instance);
        }

        static void NativeActivate()
        {
            foreach (var p in System.Diagnostics.Process.GetProcessesByName("OrangCalc"))
            {
                if (p.Id == System.Diagnostics.Process.GetCurrentProcess().Id || p.MainWindowHandle == IntPtr.Zero) continue;
                ShowWindow(p.MainWindowHandle, 9);
                SetForegroundWindow(p.MainWindowHandle);
                break;
            }
        }

        [System.Runtime.InteropServices.DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr h);
        [System.Runtime.InteropServices.DllImport("user32.dll")] static extern bool ShowWindow(IntPtr h, int cmd);

        static void Report(Exception ex)
        {
            if (ex == null) return;
            try { System.IO.File.AppendAllText(System.IO.Path.Combine(Storage.Dir, "error.log"), DateTime.Now.ToString("s") + " " + ex + Environment.NewLine); } catch (Exception) { }
            try { MessageBox.Show("Something went wrong:\n" + ex.Message + "\n\nDetails were written to error.log in the settings folder.", "OrangCalc", MessageBoxButtons.OK, MessageBoxIcon.Warning); } catch (Exception) { }
}   }   }