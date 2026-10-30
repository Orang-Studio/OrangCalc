using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Numerics;

namespace OrangCalc.Engine
{
    public static class Units
    {
        static Calculator C => Calculator.Current;

        public static readonly List<Prefix> Prefixes = new List<Prefix>
        {
            new Prefix("quecto","q",-30,false,false), new Prefix("ronto","r",-27,false,false),
            new Prefix("yocto","y",-24), new Prefix("zepto","z",-21), new Prefix("atto","a",-18), new Prefix("femto","f",-15),
            new Prefix("pico","p",-12), new Prefix("nano","n",-9), new Prefix("micro","µ",-6), new Prefix("milli","m",-3),
            new Prefix("centi","c",-2,false,false), new Prefix("deci","d",-1,false,false), new Prefix("deca","da",1,false,false), new Prefix("hecto","h",2,false,false),
            new Prefix("kilo","k",3), new Prefix("mega","M",6), new Prefix("giga","G",9), new Prefix("tera","T",12),
            new Prefix("peta","P",15), new Prefix("exa","E",18), new Prefix("zetta","Z",21), new Prefix("yotta","Y",24),
            new Prefix("ronna","R",27,false,false), new Prefix("quetta","Q",30,false,false),
            new Prefix("kibi","Ki",10,true), new Prefix("mebi","Mi",20,true), new Prefix("gibi","Gi",30,true), new Prefix("tebi","Ti",40,true),
            new Prefix("pebi","Pi",50,true), new Prefix("exbi","Ei",60,true), new Prefix("zebi","Zi",70,true), new Prefix("yobi","Yi",80,true)
        };

        public static Prefix PrefixByName(string n) => Prefixes.FirstOrDefault(p => p.Name == n || p.Symbol == n);

        // name list (abbreviation first), title, category, definition or base index, prefixes, system
        static readonly string[][] Table =
        {
            new[]{"m,meter,meters,metre,metres","Meter","Length","#0","all","SI"},
            new[]{"g,gram,grams,gramme","Gram","Mass","#1","all","SI"},
            new[]{"s,second,seconds,sec,secs","Second","Time","#2","all","SI"},
            new[]{"A,ampere,amperes,amp,amps","Ampere","Current","#3","all","SI"},
            new[]{"K,kelvin,kelvins","Kelvin","Temperature","#4","all","SI"},
            new[]{"mol,mole,moles","Mole","Amount of Substance","#5","all","SI"},
            new[]{"cd,candela,candelas","Candela","Luminous Intensity","#6","all","SI"},
            new[]{"bit,bits","Bit","Information","#7","binary","SI"},
            new[]{"rad,radian,radians","Radian","Angle","#9","some","SI"},

            new[]{"in,inch,inches","Inch","Length","0.0254 m","none","Imperial"},
            new[]{"ft,foot,feet","Foot","Length","0.3048 m","none","Imperial"},
            new[]{"yd,yard,yards","Yard","Length","0.9144 m","none","Imperial"},
            new[]{"mi,mile,miles","Mile","Length","1609.344 m","none","Imperial"},
            new[]{"nmi,nauticalmile,nauticalmiles,NM","Nautical Mile","Length","1852 m","none","Other"},
            new[]{"au,astronomicalunit,AU","Astronomical Unit","Length","149597870700 m","none","Astronomical"},
            new[]{"ly,lightyear,lightyears","Light Year","Length","9460730472580800 m","some","Astronomical"},
            new[]{"pc,parsec,parsecs","Parsec","Length","648000/pi au","some","Astronomical"},
            new[]{"Å,angstrom,angstroms,Ang","Ångström","Length","1e-10 m","none","Other"},
            new[]{"micron,microns","Micron","Length","1e-6 m","none","Other"},
            new[]{"mil,thou","Mil","Length","0.0000254 m","none","Imperial"},
            new[]{"pt,point,points","Point (typography)","Length","1/72 in","none","Other"},
            new[]{"pica,picas","Pica","Length","1/6 in","none","Other"},
            new[]{"hand,hands","Hand","Length","4 in","none","Imperial"},
            new[]{"ftm,fathom,fathoms","Fathom","Length","6 ft","none","Imperial"},
            new[]{"ch,chain,chains","Chain","Length","66 ft","none","Imperial"},
            new[]{"fur,furlong,furlongs","Furlong","Length","660 ft","none","Imperial"},
            new[]{"rd,rod,rods","Rod","Length","16.5 ft","none","Imperial"},
            new[]{"lea,league,leagues","League","Length","3 mi","none","Imperial"},
            new[]{"cubit,cubits","Cubit","Length","18 in","none","Other"},
            new[]{"lP,planck_length","Planck Length","Length","1.616255e-35 m","none","Natural"},

            new[]{"ha,hectare,hectares","Hectare","Area","10000 m^2","none","SI"},
            new[]{"are,ares","Are","Area","100 m^2","none","SI"},
            new[]{"ac,acre,acres","Acre","Area","4046.8564224 m^2","none","Imperial"},
            new[]{"barn,barns","Barn","Area","1e-28 m^2","all","Other"},

            new[]{"L,l,liter,liters,litre,litres","Liter","Volume","0.001 m^3","all","SI"},
            new[]{"gal,gallon,gallons","Gallon (US)","Volume","3.785411784 L","none","US"},
            new[]{"galUK,impgal,imperialgallon","Gallon (imperial)","Volume","4.54609 L","none","Imperial"},
            new[]{"qt,quart,quarts","Quart (US)","Volume","1/4 gal","none","US"},
            new[]{"pint,pints","Pint (US)","Volume","1/8 gal","none","US"},
            new[]{"pintUK,imppint","Pint (imperial)","Volume","1/8 galUK","none","Imperial"},
            new[]{"cup,cups","Cup (US)","Volume","1/16 gal","none","US"},
            new[]{"floz,fluidounce,fluidounces","Fluid Ounce (US)","Volume","1/128 gal","none","US"},
            new[]{"tbsp,tablespoon,tablespoons","Tablespoon","Volume","1/2 floz","none","US"},
            new[]{"tsp,teaspoon,teaspoons","Teaspoon","Volume","1/6 floz","none","US"},
            new[]{"bbl,barrel,barrels","Barrel (oil)","Volume","42 gal","none","US"},
            new[]{"cc","Cubic Centimeter","Volume","1e-6 m^3","none","Other"},
            new[]{"bu,bushel,bushels","Bushel (US)","Volume","35.23907016688 L","none","US"},

            new[]{"t,tonne,tonnes,metricton","Tonne","Mass","1000000 g","some","SI"},
            new[]{"lb,lbs,pound,pounds","Pound","Mass","453.59237 g","none","Imperial"},
            new[]{"oz,ounce,ounces","Ounce","Mass","1/16 lb","none","Imperial"},
            new[]{"st,stone,stones","Stone","Mass","14 lb","none","Imperial"},
            new[]{"gr,grain,grains","Grain","Mass","1/7000 lb","none","Imperial"},
            new[]{"ct,carat,carats","Carat","Mass","0.2 g","none","Other"},
            new[]{"ozt,troyounce","Troy Ounce","Mass","31.1034768 g","none","Other"},
            new[]{"ton,tons,shortton","Short Ton","Mass","2000 lb","none","US"},
            new[]{"longton,longtons","Long Ton","Mass","2240 lb","none","Imperial"},
            new[]{"slug,slugs","Slug","Mass","14593.90294 g","none","Imperial"},
            new[]{"u,Da,dalton,daltons,amu","Atomic Mass Unit","Mass","1.66053906660e-24 g","all","Other"},

            new[]{"min,minute,minutes","Minute","Time","60 s","none","Other"},
            new[]{"h,hr,hour,hours","Hour","Time","60 min","none","Other"},
            new[]{"d,day,days","Day","Time","24 h","none","Other"},
            new[]{"wk,week,weeks","Week","Time","7 d","none","Other"},
            new[]{"fortnight,fortnights","Fortnight","Time","14 d","none","Other"},
            new[]{"yr,year,years","Year","Time","365.2425 d","some","Other"},
            new[]{"mo,month,months","Month","Time","1/12 yr","none","Other"},
            new[]{"decade,decades","Decade","Time","10 yr","none","Other"},
            new[]{"century,centuries","Century","Time","100 yr","none","Other"},
            new[]{"millennium,millennia","Millennium","Time","1000 yr","none","Other"},
            new[]{"tP,planck_time","Planck Time","Time","5.391247e-44 s","none","Natural"},

            new[]{"kph,kmh","Kilometers per Hour","Speed","1000 m/h","none","Other"},
            new[]{"mph","Miles per Hour","Speed","1 mi/h","none","Imperial"},
            new[]{"kn,knot,knots,kt","Knot","Speed","1 nmi/h","none","Other"},
            new[]{"fps","Feet per Second","Speed","1 ft/s","none","Imperial"},

            new[]{"Gal,galileo","Gal","Acceleration","0.01 m/s^2","all","CGS"},
            new[]{"gn,gravity,gee","Standard Gravity","Acceleration","9.80665 m/s^2","none","Other"},

            new[]{"N,newton,newtons","Newton","Force","1000 g*m/s^2","all","SI"},
            new[]{"dyn,dyne,dynes","Dyne","Force","1e-5 N","all","CGS"},
            new[]{"lbf,poundforce","Pound-force","Force","4.4482216152605 N","none","Imperial"},
            new[]{"kgf,kilopond,kp","Kilogram-force","Force","9.80665 N","none","Other"},

            new[]{"J,joule,joules","Joule","Energy","1 N*m","all","SI"},
            new[]{"Wh,watthour,watthours","Watt Hour","Energy","3600 J","all","Other"},
            new[]{"cal,calorie,calories","Calorie","Energy","4.184 J","all","Other"},
            new[]{"Cal,kcal_food,foodcalorie","Calorie (food)","Energy","4184 J","none","Other"},
            new[]{"eV,electronvolt,electronvolts","Electronvolt","Energy","1.602176634e-19 J","all","Other"},
            new[]{"erg,ergs","Erg","Energy","1e-7 J","all","CGS"},
            new[]{"BTU,btu","British Thermal Unit","Energy","1055.05585262 J","none","Imperial"},
            new[]{"thm,therm,therms","Therm","Energy","105480400 J","none","US"},
            new[]{"Eh,hartree","Hartree","Energy","4.3597447222071e-18 J","none","Natural"},
            new[]{"tTNT,tonTNT","Ton of TNT","Energy","4.184e9 J","some","Other"},

            new[]{"W,watt,watts","Watt","Power","1 J/s","all","SI"},
            new[]{"hp,horsepower","Horsepower","Power","745.69987158227022 W","none","Imperial"},
            new[]{"PS,metrichorsepower","Metric Horsepower","Power","735.49875 W","none","Other"},

            new[]{"Pa,pascal,pascals","Pascal","Pressure","1 N/m^2","all","SI"},
            new[]{"bar,bars","Bar","Pressure","100000 Pa","all","Other"},
            new[]{"atm,atmosphere,atmospheres","Atmosphere","Pressure","101325 Pa","none","Other"},
            new[]{"Torr,torr","Torr","Pressure","101325/760 Pa","some","Other"},
            new[]{"mmHg","Millimeter of Mercury","Pressure","133.322387415 Pa","none","Other"},
            new[]{"inHg","Inch of Mercury","Pressure","3386.389 Pa","none","Imperial"},
            new[]{"psi","Pound per Square Inch","Pressure","1 lbf/in^2","some","Imperial"},

            new[]{"C,coulomb,coulombs","Coulomb","Electric Charge","1 A*s","all","SI"},
            new[]{"Ah,amperehour","Ampere Hour","Electric Charge","3600 C","all","Other"},
            new[]{"V,volt,volts","Volt","Electric Potential","1 W/A","all","SI"},
            new[]{"Ω,ohm,ohms","Ohm","Electric Resistance","1 V/A","all","SI"},
            new[]{"S,siemens,mho","Siemens","Electric Conductance","1 A/V","all","SI"},
            new[]{"F,farad,farads","Farad","Capacitance","1 C/V","all","SI"},
            new[]{"Wb,weber,webers","Weber","Magnetic Flux","1 V*s","all","SI"},
            new[]{"T,tesla,teslas","Tesla","Magnetic Flux Density","1 Wb/m^2","all","SI"},
            new[]{"H,henry,henries,henrys","Henry","Inductance","1 Wb/A","all","SI"},
            new[]{"G,gauss","Gauss","Magnetic Flux Density","1e-4 T","all","CGS"},
            new[]{"Mx,maxwell","Maxwell","Magnetic Flux","1e-8 Wb","all","CGS"},

            new[]{"°C,oC,celsius,degC,℃","Degree Celsius","Temperature","@C","none","SI"},
            new[]{"°F,oF,fahrenheit,degF,℉","Degree Fahrenheit","Temperature","@F","none","Imperial"},
            new[]{"°R,oR,rankine,degR","Degree Rankine","Temperature","5/9 K","none","Imperial"},

            new[]{"Hz,hertz","Hertz","Frequency","1 s^-1","all","SI"},
            new[]{"rpm","Revolutions per Minute","Frequency","1 min^-1","none","Other"},
            new[]{"Bq,becquerel,becquerels","Becquerel","Radioactivity","1 s^-1","all","SI"},
            new[]{"Ci,curie,curies","Curie","Radioactivity","3.7e10 Bq","all","Other"},
            new[]{"Gy,gray,grays","Gray","Absorbed Dose","1 J/kg","all","SI"},
            new[]{"Sv,sievert,sieverts","Sievert","Equivalent Dose","1 J/kg","all","SI"},
            new[]{"R_roentgen,roentgen","Roentgen","Exposure","2.58e-4 C/kg","none","Other"},
            new[]{"kat,katal","Katal","Catalytic Activity","1 mol/s","all","SI"},

            new[]{"lm,lumen,lumens","Lumen","Luminous Flux","1 cd","all","SI"},
            new[]{"lx,lux","Lux","Illuminance","1 lm/m^2","all","SI"},

            new[]{"°,deg,degree,degrees","Degree","Angle","pi/180 rad","none","Other"},
            new[]{"arcmin,arcminute,arcminutes","Arcminute","Angle","1/60 deg","none","Other"},
            new[]{"arcsec,arcsecond,arcseconds","Arcsecond","Angle","1/60 arcmin","some","Other"},
            new[]{"gon,grad,gradian,gradians","Gradian","Angle","pi/200 rad","none","Other"},
            new[]{"tr,turn,turns,rev,revolution","Turn","Angle","2*pi rad","none","Other"},
            new[]{"sr,steradian,steradians","Steradian","Solid Angle","=1","none","SI"},

            new[]{"B,byte,bytes,octet","Byte","Information","8 bit","binary","Other"},
            new[]{"nibble,nibbles","Nibble","Information","4 bit","none","Other"},
            new[]{"bps","Bits per Second","Data Rate","1 bit/s","all","Other"},

            new[]{"P,poise","Poise","Dynamic Viscosity","0.1 Pa*s","all","CGS"},
            new[]{"St,stokes","Stokes","Kinematic Viscosity","1e-4 m^2/s","all","CGS"},
            new[]{"M_molar,molar","Molar","Concentration","1000 mol/m^3","all","Other"},
        };

        public static readonly string[][] Currencies =
        {
            new[]{"EUR","Euro","€"}, new[]{"USD","US Dollar","$"}, new[]{"JPY","Japanese Yen","¥"}, new[]{"BGN","Bulgarian Lev",""},
            new[]{"CZK","Czech Koruna",""}, new[]{"DKK","Danish Krone",""}, new[]{"GBP","British Pound","£"}, new[]{"HUF","Hungarian Forint",""},
            new[]{"PLN","Polish Złoty","zł"}, new[]{"RON","Romanian Leu",""}, new[]{"SEK","Swedish Krona",""}, new[]{"CHF","Swiss Franc",""},
            new[]{"ISK","Icelandic Króna",""}, new[]{"NOK","Norwegian Krone",""}, new[]{"TRY","Turkish Lira","₺"},
            new[]{"AUD","Australian Dollar",""}, new[]{"BRL","Brazilian Real",""}, new[]{"CAD","Canadian Dollar",""}, new[]{"CNY","Chinese Yuan",""},
            new[]{"HKD","Hong Kong Dollar",""}, new[]{"IDR","Indonesian Rupiah",""}, new[]{"ILS","Israeli New Shekel","₪"}, new[]{"INR","Indian Rupee","₹"},
            new[]{"KRW","South Korean Won","₩"}, new[]{"MXN","Mexican Peso",""}, new[]{"MYR","Malaysian Ringgit",""}, new[]{"NZD","New Zealand Dollar",""},
            new[]{"PHP","Philippine Peso","₱"}, new[]{"SGD","Singapore Dollar",""}, new[]{"THB","Thai Baht","฿"}, new[]{"ZAR","South African Rand",""},
            new[]{"BTC","Bitcoin","₿"}
        };

        // fallback rates per euro, used until a download succeeds
        public static readonly Dictionary<string, double> DefaultRates = new Dictionary<string, double>
        {
            {"USD",1.08},{"JPY",162.0},{"BGN",1.9558},{"CZK",25.2},{"DKK",7.46},{"GBP",0.85},{"HUF",395.0},{"PLN",4.30},{"RON",4.97},
            {"SEK",11.4},{"CHF",0.95},{"ISK",150.0},{"NOK",11.6},{"TRY",36.0},{"AUD",1.65},{"BRL",6.0},{"CAD",1.48},{"CNY",7.8},
            {"HKD",8.4},{"IDR",17000.0},{"ILS",4.0},{"INR",90.0},{"KRW",1480.0},{"MXN",20.0},{"MYR",4.9},{"NZD",1.8},{"PHP",61.0},
            {"SGD",1.45},{"THB",38.0},{"ZAR",19.5},{"BTC",0.000016}
        };

        public static readonly List<string[]> MixedChains = new List<string[]>
        {
            new[]{"yr","d","h","min","s"}, new[]{"mi","yd","ft","in"}, new[]{"lb","oz"}, new[]{"st","lb"}, new[]{"°","arcmin","arcsec"}
        };

        public static void Load(Calculator c)
        {
            foreach (var row in Table)
            {
                var names = row[0].Split(',').ToList();
                var u = new Unit
                {
                    Name = names[0], Names = names, Title = row[1], Category = row[2], Definition = row[3],
                    System = row[5]
                };
                u.Plural = names.Count > 2 ? names[2] : names.Count > 1 ? names[1] : names[0];
                switch (row[4])
                {
                    case "all": u.Prefixes = PrefixUse.All; break;
                    case "some": u.Prefixes = PrefixUse.Some; break;
                    case "binary": u.Prefixes = PrefixUse.Binary; break;
                    default: u.Prefixes = PrefixUse.None; break;
                }
                c.AddUnit(u);
                Resolve(c, u);
            }
            foreach (var cur in Currencies)
            {
                var u = new Unit { Name = cur[0], Names = new List<string> { cur[0] }, Title = cur[1], Category = "Currency", IsCurrency = true, System = "Currency", Plural = cur[0] };
                if (cur[2].Length > 0) u.Names.Add(cur[2]);
                u.Dim = new Dim(8);
                if (cur[0] == "EUR") { u.Factor = Real.One; u.IsBase = true; u.Definition = "#8"; }
                else
                {
                    double rate = DefaultRates.TryGetValue(cur[0], out var r) ? r : 1;
                    u.Factor = Real.One / Real.FromDouble(rate);
                    u.ApproxFactor = true;
                    u.Definition = "1/" + rate.ToString(CultureInfo.InvariantCulture) + " EUR";
                }
                c.AddUnit(u);
            }
        }

        public static void SetRates(Calculator c, Dictionary<string, double> rates)
        {
            foreach (var kv in rates)
            {
                var u = c.Units.FirstOrDefault(x => x.IsCurrency && x.Name == kv.Key);
                if (u == null)
                {
                    u = new Unit { Name = kv.Key, Names = new List<string> { kv.Key }, Title = kv.Key, Category = "Currency", IsCurrency = true, System = "Currency", Plural = kv.Key, Dim = new Dim(8) };
                    c.AddUnit(u);
                }
                if (kv.Value <= 0) continue;
                u.Factor = Real.One / Real.FromDouble(kv.Value);
                u.ApproxFactor = true;
                u.Definition = "1/" + kv.Value.ToString(CultureInfo.InvariantCulture) + " EUR";
            }
        }

        public static void Resolve(Calculator c, Unit u)
        {
            string def = u.Definition;
            if (def.StartsWith("#"))
            {
                u.Dim = new Dim(int.Parse(def.Substring(1)));
                u.Factor = u.Name == "g" ? Real.Frac(1, 1000) : Real.One;
                u.IsBase = true;
                return;
            }
            if (def == "@C") { u.Dim = new Dim(4); u.Factor = Real.One; u.Offset = Real.Frac(27315, 100); return; }
            if (def == "@F") { u.Dim = new Dim(4); u.Factor = Real.Frac(5, 9); u.Offset = Real.Frac(45967, 180); return; }
            if (def == "=1") { u.Dim = new Dim(); u.Factor = Real.One; return; }
            var saveExact = c.ExactPhase;
            try
            {
                c.ExactPhase = false;
                var e = c.Ev.Eval(Parser.ParseText(def, c));
                if (!TryQuantity(e, out var v, out var d, out var rest) || !rest.IsOne || !v.IsReal)
                    throw new CalcException("Invalid unit definition for " + u.Name + ": " + def);
                u.Factor = v.Re;
                u.Dim = d;
                u.ApproxFactor = v.Re.Approx;
            }
            finally { c.ExactPhase = saveExact; }
        }

        public static bool TryDims(Expr units, out Dim dim, out Real factor)
        {
            dim = new Dim(); factor = Real.One;
            IEnumerable<Expr> fs = units is MulE m ? m.Factors : new[] { units };
            foreach (var f in fs)
            {
                if (f.IsOne) continue;
                if (f is UnitE u) { dim = dim.Mul(u.U.Dim); factor *= u.Factor; continue; }
                if (f is PowE p && p.B is UnitE pu && p.X is NumE pn && pn.V.IsReal && pn.V.Re.IsInt)
                {
                    int k = pn.V.Re.ToInt();
                    dim = dim.Mul(pu.U.Dim.Pow(k));
                    factor *= pu.Factor.PowInt(k);
                    continue;
                }
                return false;
            }
            return true;
        }

        // value * units * rest; units given as dimension and SI factor folded into value
        public static bool TryQuantity(Expr e, out Cx value, out Dim dim, out Expr rest)
        {
            value = Cx.One; dim = new Dim(); rest = Expr.One;
            IEnumerable<Expr> fs = e is MulE m ? m.Factors : new[] { e };
            var others = new List<Expr>();
            foreach (var f in fs)
            {
                if (f is NumE n) { value *= n.V; continue; }
                if (Simp.IsUnitFactor(f))
                {
                    if (!TryDims(f, out var d, out var fac)) return false;
                    dim = dim.Mul(d); value *= new Cx(fac);
                    continue;
                }
                if (f.HasUnits) return false;
                others.Add(f);
            }
            rest = others.Count == 0 ? Expr.One : others.Count == 1 ? others[0] : new MulE(others);
            return true;
        }

        public static bool ToSeconds(Expr t, out Real secs)
        {
            secs = null;
            if (!TryQuantity(t, out var v, out var d, out var rest) || !rest.IsOne || !v.IsReal) return false;
            var sd = new Dim(2);
            if (!d.SameAs(sd)) return false;
            secs = v.Re;
            return true;
        }

        public static Expr BaseUnits(Dim d)
        {
            var fs = new List<Expr>();
            for (int i = 0; i < Dim.Count; i++)
            {
                if (d.E[i] == 0) continue;
                Expr u;
                if (i == 1) u = new UnitE(C.FindUnit("g", out _), PrefixByName("kilo"));
                else if (i == 8) u = new UnitE(C.FindUnit("EUR", out _));
                else u = new UnitE(C.FindUnit(Dim.Symbols[i], out _));
                fs.Add(d.E[i] == 1 ? u : new PowE(u, Expr.N(d.E[i])));
            }
            if (fs.Count == 0) return Expr.One;
            return fs.Count == 1 ? fs[0] : new MulE(fs);
        }

        static Real UnitsFactor(Expr units)
        {
            TryDims(units, out _, out var f);
            return f;
        }

        public static int Complexity(Expr units)
        {
            int c = 0;
            IEnumerable<Expr> fs = units is MulE m ? m.Factors : new[] { units };
            foreach (var f in fs)
            {
                if (f is UnitE) c += 1;
                else if (f is PowE p && p.X is NumE n && n.V.IsReal) c += Math.Max(1, (int)Math.Abs(n.V.Re.ToDouble()));
            }
            return c;
        }

        static bool IsTemperature(Expr units, out Unit u)
        {
            u = null;
            if (units is UnitE ue && ue.U.Dim.SameAs(new Dim(4)) && ue.P == null) { u = ue.U; return true; }
            return false;
        }

        // converts e to the units of target
        public static Expr Convert(Expr e, Expr target)
        {
            if (e is AddE ae) return Simp.Add(ae.Terms.Select(t => Convert(t, target)).ToList());
            if (e is MatE me) return Matrices.MapElements(me, x => Convert(x, target));
            if (e is CmpE ce) return new CmpE(ce.Op, Convert(ce.L, target), Convert(ce.R, target));
            if (!TryQuantity(target, out var tv, out var td, out var trest)) throw new CalcException("Invalid conversion target");
            Simp.SplitCoef(target, out _, out var tUnits);
            Simp.SplitUnits(tUnits, out _, out tUnits);
            if (!TryQuantity(e, out var ev, out var ed, out var erest)) return e;
            Simp.SplitCoef(e, out var ecoef, out var eAll);
            Simp.SplitUnits(eAll, out _, out var eUnits);

            if (C.Opts.Temperature != TemperatureMode.Relative && IsTemperature(eUnits, out var fromT) && IsTemperature(tUnits, out var toT) && ecoef.IsReal)
            {
                var kelvin = ecoef.Re * fromT.Factor + fromT.Offset;
                var outv = (kelvin - toT.Offset) / toT.Factor;
                return Simp.Mul(Simp.Mul(new NumE(new Cx(outv)), erest), tUnits);
            }

            if (ed.SameAs(td))
            {
                var val = ev / tv;
                return Simp.Mul(new List<Expr> { new NumE(val), erest, tUnits });
            }
            if (ed.SameAs(td.Pow(-1)) && !ed.IsNone)
            {
                var val = Cx.One / (ev * tv);
                var restInv = Simp.Pow(erest, Expr.MinusOne);
                return Simp.Mul(new List<Expr> { new NumE(val), restInv, tUnits });
            }
            // partial conversion only when the target is part of the value, like J to N
            for (int i = 0; i < Dim.Count; i++)
                if (td.E[i] != 0 && (Math.Sign(td.E[i]) != Math.Sign(ed.E[i]) || Math.Abs(td.E[i]) > Math.Abs(ed.E[i]))) return e;
            var remDim = ed.Mul(td.Pow(-1));
            var remUnits = BestUnits(remDim) ?? BaseUnits(remDim);
            var remFactor = UnitsFactor(remUnits);
            var value = ev / (tv * new Cx(remFactor));
            return Simp.Mul(new List<Expr> { new NumE(value), erest, tUnits, remUnits });
        }

        public static Expr ToBase(Expr e)
        {
            if (e is AddE ae) return Simp.Add(ae.Terms.Select(ToBase).ToList());
            if (e is MatE me) return Matrices.MapElements(me, ToBase);
            if (!e.HasUnits || !TryQuantity(e, out var v, out var d, out var rest)) return e;
            Simp.SplitCoef(e, out var coef, out var all);
            Simp.SplitUnits(all, out _, out var units);
            if (IsTemperature(units, out var tu) && tu.HasOffset && C.Opts.Temperature != TemperatureMode.Relative && coef.IsReal)
                return Simp.Mul(new NumE(new Cx(coef.Re * tu.Factor + tu.Offset)), rest, BaseUnits(d));
            return Simp.Mul(new List<Expr> { new NumE(v), rest, BaseUnits(d) });
        }

        static readonly string[] DerivedPreferred = { "N", "J", "W", "Pa", "C", "V", "Ω", "F", "H", "Wb", "T", "S", "Gy", "kat", "lx" };

        // finds the simplest unit expression for a dimension
        public static Expr BestUnits(Dim d, bool siOnly = true)
        {
            if (d.IsNone) return Expr.One;
            Expr best = null;
            int bestC = int.MaxValue;
            void Consider(Expr cand)
            {
                if (!TryDims(cand, out var cd, out _) || !cd.SameAs(d)) return;
                int cc = Complexity(cand);
                if (cc < bestC) { best = cand; bestC = cc; }
            }
            var bu = BaseUnits(d);
            Consider(bu);
            for (int i = 0; i < Dim.Count; i++)
            {
                if (d.E[i] == 0 || d.Complexity != Math.Abs(d.E[i])) continue;
            }
            foreach (var n in DerivedPreferred)
            {
                var u = C.FindUnit(n, out _, false);
                if (u == null || u.Name == "Gy" && !d.SameAs(u.Dim)) continue;
                Expr ue = new UnitE(u);
                Consider(ue);
                for (int b = 0; b < Dim.Count; b++)
                {
                    if (b == 9 || b == 8) continue;
                    for (int k = -3; k <= 3; k++)
                    {
                        if (k == 0) continue;
                        var bdim = new Dim(b).Pow(k);
                        var bexpr = BaseUnits(bdim);
                        Consider(Simp.Mul(ue, bexpr));
                    }
                }
            }
            return best;
        }

        public static Expr Optimal(Expr e)
        {
            if (e is AddE ae) return Simp.Add(ae.Terms.Select(Optimal).ToList());
            if (e is MatE me) return Matrices.MapElements(me, Optimal);
            if (e is CmpE ce) return new CmpE(ce.Op, Optimal(ce.L), Optimal(ce.R));
            if (!e.HasUnits || !TryQuantity(e, out var v, out var d, out var rest)) return e;
            Simp.SplitCoef(e, out _, out var all);
            Simp.SplitUnits(all, out _, out var units);
            if (units.IsOne) return e;
            int cur = Complexity(units);
            if (d.IsNone) return Simp.Mul(new NumE(v), rest);
            if (IsCurrencyOnly(units)) return e;
            var best = BestUnits(d);
            if (best == null) return e;
            int bc = Complexity(best);
            bool cancels = SumOfParts(units) > d.Complexity;
            if (bc < cur && (cur >= 3 || cancels))
            {
                var f = UnitsFactor(best);
                return Simp.Mul(new List<Expr> { new NumE(v / new Cx(f)), rest, best });
            }
            return e;
        }

        static bool IsCurrencyOnly(Expr units)
        {
            IEnumerable<Expr> fs = units is MulE m ? m.Factors : new[] { units };
            return fs.All(f => (f is UnitE u && u.U.IsCurrency) || (f is PowE p && p.B is UnitE pu && pu.U.IsCurrency));
        }

        static int SumOfParts(Expr units)
        {
            int s = 0;
            IEnumerable<Expr> fs = units is MulE m ? m.Factors : new[] { units };
            foreach (var f in fs)
            {
                if (f is UnitE u) s += u.U.Dim.Complexity;
                else if (f is PowE p && p.B is UnitE pu && p.X is NumE n && n.V.IsReal) s += pu.U.Dim.Complexity * (int)Math.Abs(n.V.Re.ToDouble());
            }
            return s;
        }

        // picks a prefix so that the coefficient lands in [1, 1000)
        public static Expr OptimizePrefix(Expr e, bool force)
        {
            if (e is AddE ae) return Simp.Add(ae.Terms.Select(t => OptimizePrefix(t, force)).ToList());
            if (!(e is MulE m) || m.Factors.Count < 2 || !(m.Factors[0] is NumE n) || !n.V.IsReal || n.V.Re.IsZero) return e;
            int ui = -1;
            for (int i = 1; i < m.Factors.Count; i++) if (Simp.IsUnitFactor(m.Factors[i])) { ui = i; break; }
            if (ui < 0) return e;
            int unitCount = m.Factors.Count(Simp.IsUnitFactor);
            var f = m.Factors[ui];
            UnitE u = f as UnitE ?? (UnitE)((PowE)f).B;
            int pw = f is PowE pp ? (int)Math.Round(((NumE)pp.X).V.Re.ToDouble()) : 1;
            if (pw <= 0 && !C.Opts.DenominatorPrefixes) return e;
            if (u.U.Prefixes == PrefixUse.None || u.U.HasOffset) return e;
            if (u.U.IsCurrency && !C.Opts.CurrencyPrefixes) return e;
            if (!force && C.Opts.Prefixes == PrefixMode.None) return e;
            if (!force && C.Opts.Prefixes == PrefixMode.Some && u.P == null && u.U.Prefixes != PrefixUse.All && u.U.Prefixes != PrefixUse.Binary) return e;
            int minExp = -30, maxExp = 30;
            if (!force && C.Opts.Prefixes == PrefixMode.Some)
            {
                if (unitCount != 1 || pw != 1) return e;
                switch (u.U.Name)
                {
                    case "m": minExp = -12; maxExp = 3; break;
                    case "g": minExp = -12; maxExp = 3; break;
                    case "s": minExp = -15; maxExp = 0; break;
                    case "L": minExp = -6; maxExp = 0; break;
                    case "B": case "bit": minExp = 0; maxExp = 24; break;
                    case "W": case "J": case "V": case "Hz": case "N": case "Pa": case "A": case "Ω": case "F": case "C": case "H": case "T": case "Wh": case "eV": case "S": case "Wb": case "bps": minExp = -12; maxExp = 12; break;
                    default: if (u.P == null) return e; break;
                }
            }
            double v = Math.Abs(n.V.Re.ToDouble());
            if (double.IsInfinity(v) || v == 0) return e;
            Real baseFactor = u.P == null ? Real.One : u.P.Value;
            double total = v * Math.Pow(baseFactor.ToDouble(), pw);
            bool binary = u.U.Prefixes == PrefixUse.Binary && (C.Opts.BinaryPrefixes || (u.P != null && u.P.Binary));
            Prefix best = null;
            double bestScore = double.MaxValue;
            var cands = Prefixes.Where(p => binary ? p.Binary : !p.Binary).Where(p => p.Common || C.Opts.AllSiPrefixes || p == u.P).Where(p => p.Binary || (p.Exp >= minExp && p.Exp <= maxExp)).ToList();
            if (u.U.Name == "g" && !binary) { }
            foreach (var p in cands.Concat(new Prefix[] { null }))
            {
                double pv = p == null ? 1 : p.Value.ToDouble();
                double val = total / Math.Pow(pv, pw);
                double limit = binary ? 1024 : 1000;
                double score = val >= 1 && val < limit ? 0 : val < 1 ? 1 / val : val / limit;
                if (u.U.Name == "g" && p == null && total >= 1000) score += 1e-9;
                if (score < bestScore - 1e-12) { bestScore = score; best = p; }
            }
            if (u.U.Name == "s" && best != null && best.Exp > 0) return e;
            if (u.U.Name == "L" && best != null && best.Exp > 0) best = null;
            if ((best?.Name) == (u.P?.Name)) return e;
            var nu = new UnitE(u.U, best);
            var ratio = RMath.Pow(baseFactor / (best == null ? Real.One : best.Value), Real.Int(pw));
            var fs = m.Factors.ToList();
            fs[0] = new NumE(new Cx(n.V.Re * ratio));
            fs[ui] = pw == 1 ? (Expr)nu : new PowE(nu, Expr.N(pw));
            return new MulE(fs);
        }

        // 1.5 h -> 1 h + 30 min
        public static Expr ToMixed(Expr e, bool force)
        {
            if (!(e is MulE m) || m.Factors.Count != 2 || !(m.Factors[0] is NumE n) || !n.V.IsReal || !(m.Factors[1] is UnitE u) || u.P != null) return e;
            var chain = MixedChains.FirstOrDefault(ch => ch.Contains(u.U.Name));
            if (chain == null) return e;
            var val = n.V.Re;
            if (val.IsInt && !force) return e;
            bool neg = val.Sign < 0;
            if (neg) val = -val;
            if (!force && val < Real.One) return e;
            int start = Array.IndexOf(chain, u.U.Name);
            var parts = new List<Expr>();
            Real remaining = val * u.U.Factor;
            bool approx = val.Approx;
            for (int i = start; i < chain.Length; i++)
            {
                var cu = C.FindUnit(chain[i], out _, false);
                var amount = remaining / cu.Factor;
                if (i == chain.Length - 1)
                {
                    if (approx && Math.Abs(amount.ToDouble()) < 1e-9) break;
                    if (!amount.IsZero) parts.Add(Simp.Mul(new NumE(new Cx(amount)), new UnitE(cu)));
                    break;
                }
                var whole = amount.Floor();
                var fracPart = amount - whole;
                if (approx && Math.Abs(1 - fracPart.ToDouble()) < 1e-9) { whole = whole + Real.One; fracPart = Real.Zero; }
                if (!whole.IsZero) parts.Add(Simp.Mul(new NumE(new Cx(Real.Int(whole.ToBigInt()))), new UnitE(cu)));
                remaining = fracPart * cu.Factor;
                if (remaining.IsZero || (approx && Math.Abs(fracPart.ToDouble()) < 1e-9)) break;
            }
            if (parts.Count <= 1) return e;
            if (neg) parts = parts.Select(Simp.Neg).ToList();
            return new AddE(parts);
        }
    }
}
