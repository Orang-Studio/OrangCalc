using System;
using System.Collections.Generic;
using System.Linq;

namespace OrangCalc.Engine
{
    public static class Constants
    {
        static Variable Add(Calculator c, string names, string title, string cat, string value, bool approx = true, bool symbolic = false, bool unit = false, string desc = "")
        {
            var v = new Variable
            {
                Names = names.Split(',').ToList(), Title = title, Category = cat, ValueText = value, IsConstant = true,
                IsApproximate = approx, KeepSymbolic = symbolic, HasUnit = unit, Description = desc
            };
            v.Name = v.Names[0];
            c.AddVariable(v);
            return v;
        }

        public static void Register(Calculator c)
        {
            const string M = "Mathematical Constants", P = "Physical Constants", A = "Astronomical", Q = "Quantum & Atomic", EM = "Electromagnetic", U = "Universal", PL = "Planck Units";

            var pi = Add(c, "pi,π", "Archimedes' Constant (pi)", M, "", true, true);
            pi.Dynamic = cc => new NumE(new Cx(RMath.Pi()));
            var e = Add(c, "e", "Euler's Number", M, "", true, true);
            e.Dynamic = cc => new NumE(new Cx(RMath.E()));
            var i = Add(c, "i", "Imaginary Unit", M, "", false);
            i.Dynamic = cc => new NumE(Cx.I);
            i.IsApproximate = false;
            Add(c, "golden,φ,phi", "Golden Ratio", M, "(1+sqrt(5))/2", false, true);
            Add(c, "supergolden,ψ", "Supergolden Ratio", M, "1.4655712318767680266567312252199391080256");
            Add(c, "euler,γ,EulerGamma", "Euler–Mascheroni Constant", M, "0.5772156649015328606065120900824024310422", true, true).Dynamic = cc => DecimalConst("0.5772156649015328606065120900824024310422");
            Add(c, "catalan", "Catalan's Constant", M, "", true, true).Dynamic = cc => DecimalConst("0.9159655941772190150546035149323841107741");
            Add(c, "apery,ζ3", "Apéry's Constant", M, "", true, true).Dynamic = cc => DecimalConst("1.2020569031595942853997381615114499907650");
            Add(c, "omega_constant,omega", "Omega Constant", M, "0.5671432904097838729999686622103555497538");
            Add(c, "plastic,ρ_plastic", "Plastic Number", M, "1.3247179572447460259609088544780973407344");
            Add(c, "sqrt2,pythagoras_constant", "Pythagoras' Constant", M, "sqrt(2)", false);
            var inf = Add(c, "infinity,inf,∞", "Infinity", M, "", false);
            inf.Dynamic = cc => new NumE(new Cx(Real.PosInf));
            Add(c, "true", "True", "Logical", "1", false);
            Add(c, "false", "False", "Logical", "0", false);

            for (int k = 1; k <= 5; k++)
            {
                int idx = k - 1;
                var ans = Add(c, k == 1 ? "ans,answer" : "ans" + k, k == 1 ? "Last Answer" : "Answer " + k, "Temporary", "", false);
                ans.Dynamic = cc =>
                {
                    if (cc.Answers.Count <= idx) throw new CalcException("No previous answer");
                    return cc.Answers[idx];
                };
                ans.IsConstant = false;
            }
            var mem = Add(c, "memory,MR", "Memory", "Temporary", "", false);
            mem.Dynamic = cc => cc.Memory ?? Expr.Zero;
            mem.IsConstant = false;

            foreach (var x in new[] { "x", "y", "z" })
            {
                var u = new Variable { Name = x, Names = { x }, IsUnknown = true, Title = "Unknown " + x, Category = "Unknowns", AType = AssumeType.Real };
                c.AddVariable(u);
            }

            Add(c, "c,speed_of_light,c0", "Speed of Light", U, "299792458 m/s", false, false, true);
            Add(c, "planck,ℎ,h_planck", "Planck Constant", U, "6.62607015e-34 J*s", false, false, true);
            Add(c, "hbar,ħ,reduced_planck", "Reduced Planck Constant", U, "6.62607015e-34 J*s/(2*pi)", false, false, true);
            Add(c, "G,gravitational_constant,newton_constant", "Newtonian Constant of Gravitation", U, "6.67430e-11 m^3/(kg*s^2)", true, false, true);
            Add(c, "g0,g_n,standard_gravity", "Standard Acceleration of Gravity", U, "9.80665 m/s^2", false, false, true);
            Add(c, "epsilon0,ε0,ε_0,electric_constant", "Vacuum Electric Permittivity", EM, "8.8541878128e-12 F/m", true, false, true);
            Add(c, "mu0,µ0,μ0,magnetic_constant", "Vacuum Magnetic Permeability", EM, "1.25663706212e-6 N/A^2", true, false, true);
            Add(c, "Z0,Z_0,impedance_of_vacuum", "Characteristic Impedance of Vacuum", EM, "376.730313668 Ω", true, false, true);
            Add(c, "q_e,elementary_charge,e_charge", "Elementary Charge", EM, "1.602176634e-19 C", false, false, true);
            Add(c, "k_B,boltzmann,k_b", "Boltzmann Constant", P, "1.380649e-23 J/K", false, false, true);
            Add(c, "N_A,avogadro,NA", "Avogadro Constant", P, "6.02214076e23 mol^-1", false, false, true);
            Add(c, "R,gas_constant,molar_gas_constant", "Molar Gas Constant", P, "8.314462618 J/(mol*K)", true, false, true);
            Add(c, "faraday,F_faraday", "Faraday Constant", P, "96485.33212 C/mol", true, false, true);
            Add(c, "stefan_boltzmann,σ_SB,sigma_SB", "Stefan–Boltzmann Constant", P, "5.670374419e-8 W/(m^2*K^4)", true, false, true);
            Add(c, "wien,b_wien", "Wien Wavelength Displacement Constant", P, "2.897771955e-3 m*K", true, false, true);
            Add(c, "V_m,molar_volume", "Molar Volume of Ideal Gas (273.15 K, 101.325 kPa)", P, "22.41396954e-3 m^3/mol", true, false, true);
            Add(c, "loschmidt,n0", "Loschmidt Constant", P, "2.686780111e25 m^-3", true, false, true);
            Add(c, "speed_of_sound,c_sound", "Speed of Sound (20 °C, dry air)", P, "343 m/s", true, false, true);
            Add(c, "atm_pressure,standard_atmosphere", "Standard Atmosphere", P, "101325 Pa", false, false, true);

            Add(c, "m_e,electron_mass", "Electron Mass", Q, "9.1093837015e-31 kg", true, false, true);
            Add(c, "m_p,proton_mass", "Proton Mass", Q, "1.67262192369e-27 kg", true, false, true);
            Add(c, "m_n,neutron_mass", "Neutron Mass", Q, "1.67492749804e-27 kg", true, false, true);
            Add(c, "m_u,atomic_mass_constant", "Atomic Mass Constant", Q, "1.66053906660e-27 kg", true, false, true);
            Add(c, "m_mu,muon_mass", "Muon Mass", Q, "1.883531627e-28 kg", true, false, true);
            Add(c, "alpha,α,fine_structure", "Fine-Structure Constant", Q, "7.2973525693e-3", true);
            Add(c, "rydberg,R_inf,R∞", "Rydberg Constant", Q, "10973731.568160 m^-1", true, false, true);
            Add(c, "a0,a_0,bohr_radius", "Bohr Radius", Q, "5.29177210903e-11 m", true, false, true);
            Add(c, "mu_B,µB,μB,bohr_magneton", "Bohr Magneton", Q, "9.2740100783e-24 J/T", true, false, true);
            Add(c, "mu_N,µN,μN,nuclear_magneton", "Nuclear Magneton", Q, "5.0507837461e-27 J/T", true, false, true);
            Add(c, "Phi0,Φ0,flux_quantum", "Magnetic Flux Quantum", Q, "2.067833848e-15 Wb", true, false, true);
            Add(c, "G0,G_0,conductance_quantum", "Conductance Quantum", Q, "7.748091729e-5 S", true, false, true);
            Add(c, "K_J,josephson", "Josephson Constant", Q, "483597.8484e9 Hz/V", true, false, true);
            Add(c, "R_K,von_klitzing", "von Klitzing Constant", Q, "25812.80745 Ω", true, false, true);
            Add(c, "lambda_C,compton_wavelength", "Compton Wavelength", Q, "2.42631023867e-12 m", true, false, true);
            Add(c, "r_e,electron_radius", "Classical Electron Radius", Q, "2.8179403262e-15 m", true, false, true);
            Add(c, "sigma_e,thomson_cross_section", "Thomson Cross Section", Q, "6.6524587321e-29 m^2", true, false, true);
            Add(c, "E_h,hartree_energy", "Hartree Energy", Q, "4.3597447222071e-18 J", true, false, true);

            Add(c, "planck_length,l_P", "Planck Length", PL, "1.616255e-35 m", true, false, true);
            Add(c, "planck_mass,m_P", "Planck Mass", PL, "2.176434e-8 kg", true, false, true);
            Add(c, "planck_time,t_P", "Planck Time", PL, "5.391247e-44 s", true, false, true);
            Add(c, "planck_temperature,T_P", "Planck Temperature", PL, "1.416784e32 K", true, false, true);

            Add(c, "earth_mass,M_earth", "Earth Mass", A, "5.9722e24 kg", true, false, true);
            Add(c, "earth_radius,R_earth", "Earth Equatorial Radius", A, "6378137 m", true, false, true);
            Add(c, "solar_mass,M_sun", "Solar Mass", A, "1.98847e30 kg", true, false, true);
            Add(c, "solar_radius,R_sun", "Solar Radius", A, "6.957e8 m", true, false, true);
            Add(c, "solar_luminosity,L_sun", "Solar Luminosity", A, "3.828e26 W", true, false, true);
            Add(c, "moon_mass,M_moon", "Moon Mass", A, "7.342e22 kg", true, false, true);
            Add(c, "hubble,H0", "Hubble Constant", A, "67.4 km/(s*Mpc)", true, false, true);
        }

        static Expr DecimalConst(string s)
        {
            var r = Real.Parse(s.Replace("0.", "0.").Trim());
            return new NumE(new Cx(r.AsApprox()));
        }
    }
}
