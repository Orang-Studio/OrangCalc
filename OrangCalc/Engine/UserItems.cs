using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using System.Xml;

namespace OrangCalc.Engine
{
    // user defined functions, variables, units and data sets, and their xml persistence
    public static class UserItems
    {
        public static MathFunction MakeFunction(Calculator c, string name, string formula, IList<string> argNames = null)
        {
            int count = 0;
            foreach (Match m in Regex.Matches(formula, @"\\([a-z])"))
            {
                char ch = m.Groups[1].Value[0];
                int idx = ch >= 'x' ? ch - 'x' : ch - 'a' + 3;
                count = Math.Max(count, idx + 1);
            }
            var f = new MathFunction { Name = name, Names = { name }, Formula = formula, IsUser = true, Category = "User functions", Title = name };
            for (int i = 0; i < count; i++)
            {
                string an = argNames != null && i < argNames.Count ? argNames[i] : (i < 3 ? ((char)('x' + i)).ToString() : ((char)('a' + i - 3)).ToString());
                f.Args.Add(new ArgDef(an));
            }
            f.MinArgs = count;
            f.MaxArgs = count;
            f.Parsed = Parser.ParseText(formula, c);
            return f;
        }

        public static Variable MakeVariable(Calculator c, string name, string valueText, string title = "", string category = "User variables")
        {
            var v = new Variable { Name = name, Names = { name }, ValueText = valueText, IsUser = true, Category = category, Title = string.IsNullOrEmpty(title) ? name : title };
            v.Value = c.Run(Parser.ParseText(valueText, c));
            return v;
        }

        public static Unit MakeUnit(Calculator c, string name, IEnumerable<string> names, string plural, string definition, string title, string category, PrefixUse prefixes)
        {
            var u = new Unit
            {
                Name = name, Names = new[] { name }.Concat(names ?? new string[0]).Where(n => n.Length > 0).Distinct().ToList(), Plural = string.IsNullOrEmpty(plural) ? name : plural,
                Definition = definition, Title = string.IsNullOrEmpty(title) ? name : title, Category = string.IsNullOrEmpty(category) ? "User units" : category, IsUser = true, Prefixes = prefixes, System = "User"
            };
            Units.Resolve(c, u);
            return u;
        }

        public static void Save(Calculator c, XmlDocument doc, XmlElement root)
        {
            foreach (var f in c.Functions.Where(x => x.IsUser))
            {
                var e = doc.CreateElement("function");
                e.SetAttribute("name", f.Name);
                e.SetAttribute("names", string.Join(",", f.Names));
                e.SetAttribute("title", f.Title);
                e.SetAttribute("category", f.Category);
                e.SetAttribute("description", f.Description);
                e.SetAttribute("formula", f.Formula);
                e.SetAttribute("args", string.Join(",", f.Args.Select(a => a.Name)));
                e.SetAttribute("condition", f.Condition);
                e.SetAttribute("active", f.Active ? "1" : "0");
                root.AppendChild(e);
            }
            foreach (var v in c.Variables.Where(x => x.IsUser))
            {
                var e = doc.CreateElement("variable");
                e.SetAttribute("name", v.Name);
                e.SetAttribute("names", string.Join(",", v.Names));
                e.SetAttribute("title", v.Title);
                e.SetAttribute("category", v.Category);
                e.SetAttribute("description", v.Description);
                e.SetAttribute("value", v.IsUnknown ? "" : (string.IsNullOrEmpty(v.ValueText) ? Formatter.Ascii(v.Value, c.Opts) : v.ValueText));
                e.SetAttribute("unknown", v.IsUnknown ? "1" : "0");
                e.SetAttribute("type", v.AType.ToString());
                e.SetAttribute("sign", v.ASign.ToString());
                e.SetAttribute("active", v.Active ? "1" : "0");
                root.AppendChild(e);
            }
            foreach (var u in c.Units.Where(x => x.IsUser))
            {
                var e = doc.CreateElement("unit");
                e.SetAttribute("name", u.Name);
                e.SetAttribute("names", string.Join(",", u.Names));
                e.SetAttribute("plural", u.Plural);
                e.SetAttribute("title", u.Title);
                e.SetAttribute("category", u.Category);
                e.SetAttribute("definition", u.Definition);
                e.SetAttribute("prefixes", u.Prefixes.ToString());
                e.SetAttribute("active", u.Active ? "1" : "0");
                root.AppendChild(e);
            }
            foreach (var ds in c.DataSets.Where(x => x.IsUser))
            {
                var e = doc.CreateElement("dataset");
                e.SetAttribute("name", ds.Name);
                e.SetAttribute("title", ds.Title);
                e.SetAttribute("description", ds.Description);
                foreach (var p in ds.Properties)
                {
                    var pe = doc.CreateElement("property");
                    pe.SetAttribute("name", p.Name); pe.SetAttribute("title", p.Title); pe.SetAttribute("unit", p.Unit);
                    pe.SetAttribute("text", p.IsText ? "1" : "0"); pe.SetAttribute("key", p.IsKey ? "1" : "0"); pe.SetAttribute("approximate", p.Approximate ? "1" : "0");
                    e.AppendChild(pe);
                }
                foreach (var o in ds.Objects)
                {
                    var oe = doc.CreateElement("object");
                    foreach (var kv in o.Values) { var ve = doc.CreateElement("v"); ve.SetAttribute("k", kv.Key); ve.SetAttribute("v", kv.Value); oe.AppendChild(ve); }
                    e.AppendChild(oe);
                }
                root.AppendChild(e);
            }
        }

        public static List<string> Load(Calculator c, XmlElement root)
        {
            var errors = new List<string>();
            Calculator.Current = c;
            foreach (XmlElement e in root.SelectNodes("unit"))
            {
                try
                {
                    var names = e.GetAttribute("names").Split(',').Where(n => n.Length > 0).ToList();
                    Enum.TryParse(e.GetAttribute("prefixes"), out PrefixUse pu);
                    var u = MakeUnit(c, e.GetAttribute("name"), names, e.GetAttribute("plural"), e.GetAttribute("definition"), e.GetAttribute("title"), e.GetAttribute("category"), pu);
                    u.Active = e.GetAttribute("active") != "0";
                    c.AddUnit(u);
                }
                catch (CalcException ex) { errors.Add(e.GetAttribute("name") + ": " + ex.Message); }
            }
            foreach (XmlElement e in root.SelectNodes("variable"))
            {
                try
                {
                    var name = e.GetAttribute("name");
                    Variable v;
                    if (e.GetAttribute("unknown") == "1")
                    {
                        v = new Variable { Name = name, Names = { name }, IsUnknown = true, IsUser = true, Category = e.GetAttribute("category"), Title = e.GetAttribute("title") };
                        Enum.TryParse(e.GetAttribute("type"), out v.AType);
                        Enum.TryParse(e.GetAttribute("sign"), out v.ASign);
                    }
                    else v = MakeVariable(c, name, e.GetAttribute("value"), e.GetAttribute("title"), e.GetAttribute("category"));
                    var names = e.GetAttribute("names").Split(',').Where(n => n.Length > 0).ToList();
                    if (names.Count > 0) v.Names = names;
                    v.Description = e.GetAttribute("description");
                    v.Active = e.GetAttribute("active") != "0";
                    c.AddVariable(v);
                }
                catch (CalcException ex) { errors.Add(e.GetAttribute("name") + ": " + ex.Message); }
            }
            foreach (XmlElement e in root.SelectNodes("function"))
            {
                try
                {
                    var args = e.GetAttribute("args").Split(',').Where(n => n.Length > 0).ToList();
                    var f = MakeFunction(c, e.GetAttribute("name"), e.GetAttribute("formula"), args);
                    var names = e.GetAttribute("names").Split(',').Where(n => n.Length > 0).ToList();
                    if (names.Count > 0) f.Names = names;
                    f.Title = e.GetAttribute("title");
                    f.Category = e.GetAttribute("category");
                    f.Description = e.GetAttribute("description");
                    f.Condition = e.GetAttribute("condition");
                    f.Active = e.GetAttribute("active") != "0";
                    c.AddFunction(f);
                }
                catch (CalcException ex) { errors.Add(e.GetAttribute("name") + ": " + ex.Message); }
            }
            foreach (XmlElement e in root.SelectNodes("dataset"))
            {
                var ds = new DataSet { Name = e.GetAttribute("name"), Title = e.GetAttribute("title"), Description = e.GetAttribute("description"), IsUser = true, Category = "User data sets" };
                ds.Names.Add(ds.Name);
                foreach (XmlElement pe in e.SelectNodes("property"))
                    ds.Properties.Add(new DataProperty(pe.GetAttribute("name"), pe.GetAttribute("title"), pe.GetAttribute("unit"), pe.GetAttribute("text") == "1") { IsKey = pe.GetAttribute("key") == "1", Approximate = pe.GetAttribute("approximate") != "0" });
                foreach (XmlElement oe in e.SelectNodes("object"))
                {
                    var o = new DataObject();
                    foreach (XmlElement ve in oe.SelectNodes("v")) o.Values[ve.GetAttribute("k")] = ve.GetAttribute("v");
                    ds.Objects.Add(o);
                }
                c.DataSets.RemoveAll(x => x.Name == ds.Name && x.IsUser);
                c.DataSets.Add(ds);
            }
            return errors;
        }

        public static void ClearUser(Calculator c)
        {
            foreach (var f in c.Functions.Where(x => x.IsUser).ToList()) c.RemoveFunction(f);
            foreach (var v in c.Variables.Where(x => x.IsUser).ToList()) c.RemoveVariable(v);
            foreach (var u in c.Units.Where(x => x.IsUser).ToList()) c.RemoveUnit(u);
            c.DataSets.RemoveAll(d => d.IsUser);
        }
    }
}
