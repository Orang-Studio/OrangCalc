use super::*;

#[allow(clippy::too_many_arguments)]
fn var(names: &str, title: &str, cat: &str, value: &str, approx: bool, symbolic: bool, unit: bool) -> Variable {
    let names: Vec<String> = names.split(',').map(|s| s.to_string()).collect();
    Variable {
        item: Item { name: names[0].clone(), names, title: title.into(), category: cat.into(), ..Default::default() },
        value_text: value.into(),
        is_constant: true,
        is_approximate: approx,
        keep_symbolic: symbolic,
        has_unit: unit,
        ..Default::default()
    }
}

fn add(c: &Calculator, names: &str, title: &str, cat: &str, value: &str, approx: bool, symbolic: bool, unit: bool) {
    c.add_variable(var(names, title, cat, value, approx, symbolic, unit));
}

fn dynamic(c: &Calculator, mut v: Variable, f: DynFn) {
    v.dynamic = Some(f);
    c.add_variable(v);
}

fn decimal_const(s: &str) -> Expr {
    num_r(Real::parse(s, 10).as_approx())
}

pub fn register(c: &Calculator) {
    const M: &str = "Mathematical Constants";
    const P: &str = "Physical Constants";
    const A: &str = "Astronomical";
    const Q: &str = "Quantum & Atomic";
    const EM: &str = "Electromagnetic";
    const U: &str = "Universal";
    const PL: &str = "Planck Units";

    dynamic(c, var("pi,π", "Archimedes' Constant (pi)", M, "", true, true, false), |_| num_r(rmath::pi()));
    dynamic(c, var("e", "Euler's Number", M, "", true, true, false), |_| num_r(rmath::e()));
    dynamic(c, var("i", "Imaginary Unit", M, "", false, false, false), |_| num(Cx::i()));
    add(c, "golden,φ,phi", "Golden Ratio", M, "(1+sqrt(5))/2", false, true, false);
    add(c, "supergolden,ψ", "Supergolden Ratio", M, "1.4655712318767680266567312252199391080256", true, false, false);
    dynamic(c, var("euler,γ,EulerGamma", "Euler–Mascheroni Constant", M, "0.5772156649015328606065120900824024310422", true, true, false), |_| decimal_const("0.5772156649015328606065120900824024310422"));
    dynamic(c, var("catalan", "Catalan's Constant", M, "", true, true, false), |_| decimal_const("0.9159655941772190150546035149323841107741"));
    dynamic(c, var("apery,ζ3", "Apéry's Constant", M, "", true, true, false), |_| decimal_const("1.2020569031595942853997381615114499907650"));
    add(c, "omega_constant,omega", "Omega Constant", M, "0.5671432904097838729999686622103555497538", true, false, false);
    add(c, "plastic,ρ_plastic", "Plastic Number", M, "1.3247179572447460259609088544780973407344", true, false, false);
    add(c, "sqrt2,pythagoras_constant", "Pythagoras' Constant", M, "sqrt(2)", false, false, false);
    dynamic(c, var("infinity,inf,∞", "Infinity", M, "", false, false, false), |_| num_r(Real::pos_inf()));
    add(c, "true", "True", "Logical", "1", false, false, false);
    add(c, "false", "False", "Logical", "0", false, false, false);

    const ANS: [DynFn; 5] = [|c| answer(c, 0), |c| answer(c, 1), |c| answer(c, 2), |c| answer(c, 3), |c| answer(c, 4)];
    for k in 1..=5usize {
        let names = if k == 1 { "ans,answer".to_string() } else { format!("ans{}", k) };
        let title = if k == 1 { "Last Answer".to_string() } else { format!("Answer {}", k) };
        let mut v = var(&names, &title, "Temporary", "", false, false, false);
        v.is_constant = false;
        dynamic(c, v, ANS[k - 1]);
    }
    let mut mem = var("memory,MR", "Memory", "Temporary", "", false, false, false);
    mem.is_constant = false;
    dynamic(c, mem, |c| c.memory.borrow().clone().unwrap_or_else(zero));

    for x in ["x", "y", "z"] {
        c.add_variable(Variable {
            item: Item { name: x.into(), names: vec![x.into()], title: format!("Unknown {}", x), category: "Unknowns".into(), ..Default::default() },
            is_unknown: true,
            atype: AssumeType::Real,
            ..Default::default()
        });
    }

    let t = |c: &Calculator, n: &str, title: &str, cat: &str, v: &str, approx: bool| add(c, n, title, cat, v, approx, false, true);
    t(c, "c,speed_of_light,c0", "Speed of Light", U, "299792458 m/s", false);
    t(c, "planck,ℎ,h_planck", "Planck Constant", U, "6.62607015e-34 J*s", false);
    t(c, "hbar,ħ,reduced_planck", "Reduced Planck Constant", U, "6.62607015e-34 J*s/(2*pi)", false);
    t(c, "G,gravitational_constant,newton_constant", "Newtonian Constant of Gravitation", U, "6.67430e-11 m^3/(kg*s^2)", true);
    t(c, "g0,g_n,standard_gravity", "Standard Acceleration of Gravity", U, "9.80665 m/s^2", false);
    t(c, "epsilon0,ε0,ε_0,electric_constant", "Vacuum Electric Permittivity", EM, "8.8541878128e-12 F/m", true);
    t(c, "mu0,µ0,μ0,magnetic_constant", "Vacuum Magnetic Permeability", EM, "1.25663706212e-6 N/A^2", true);
    t(c, "Z0,Z_0,impedance_of_vacuum", "Characteristic Impedance of Vacuum", EM, "376.730313668 Ω", true);
    t(c, "q_e,elementary_charge,e_charge", "Elementary Charge", EM, "1.602176634e-19 C", false);
    t(c, "k_B,boltzmann,k_b", "Boltzmann Constant", P, "1.380649e-23 J/K", false);
    t(c, "N_A,avogadro,NA", "Avogadro Constant", P, "6.02214076e23 mol^-1", false);
    t(c, "R,gas_constant,molar_gas_constant", "Molar Gas Constant", P, "8.314462618 J/(mol*K)", true);
    t(c, "faraday,F_faraday", "Faraday Constant", P, "96485.33212 C/mol", true);
    t(c, "stefan_boltzmann,σ_SB,sigma_SB", "Stefan–Boltzmann Constant", P, "5.670374419e-8 W/(m^2*K^4)", true);
    t(c, "wien,b_wien", "Wien Wavelength Displacement Constant", P, "2.897771955e-3 m*K", true);
    t(c, "V_m,molar_volume", "Molar Volume of Ideal Gas (273.15 K, 101.325 kPa)", P, "22.41396954e-3 m^3/mol", true);
    t(c, "loschmidt,n0", "Loschmidt Constant", P, "2.686780111e25 m^-3", true);
    t(c, "speed_of_sound,c_sound", "Speed of Sound (20 °C, dry air)", P, "343 m/s", true);
    t(c, "atm_pressure,standard_atmosphere", "Standard Atmosphere", P, "101325 Pa", false);

    t(c, "m_e,electron_mass", "Electron Mass", Q, "9.1093837015e-31 kg", true);
    t(c, "m_p,proton_mass", "Proton Mass", Q, "1.67262192369e-27 kg", true);
    t(c, "m_n,neutron_mass", "Neutron Mass", Q, "1.67492749804e-27 kg", true);
    t(c, "m_u,atomic_mass_constant", "Atomic Mass Constant", Q, "1.66053906660e-27 kg", true);
    t(c, "m_mu,muon_mass", "Muon Mass", Q, "1.883531627e-28 kg", true);
    add(c, "alpha,α,fine_structure", "Fine-Structure Constant", Q, "7.2973525693e-3", true, false, false);
    t(c, "rydberg,R_inf,R∞", "Rydberg Constant", Q, "10973731.568160 m^-1", true);
    t(c, "a0,a_0,bohr_radius", "Bohr Radius", Q, "5.29177210903e-11 m", true);
    t(c, "mu_B,µB,μB,bohr_magneton", "Bohr Magneton", Q, "9.2740100783e-24 J/T", true);
    t(c, "mu_N,µN,μN,nuclear_magneton", "Nuclear Magneton", Q, "5.0507837461e-27 J/T", true);
    t(c, "Phi0,Φ0,flux_quantum", "Magnetic Flux Quantum", Q, "2.067833848e-15 Wb", true);
    t(c, "G0,G_0,conductance_quantum", "Conductance Quantum", Q, "7.748091729e-5 S", true);
    t(c, "K_J,josephson", "Josephson Constant", Q, "483597.8484e9 Hz/V", true);
    t(c, "R_K,von_klitzing", "von Klitzing Constant", Q, "25812.80745 Ω", true);
    t(c, "lambda_C,compton_wavelength", "Compton Wavelength", Q, "2.42631023867e-12 m", true);
    t(c, "r_e,electron_radius", "Classical Electron Radius", Q, "2.8179403262e-15 m", true);
    t(c, "sigma_e,thomson_cross_section", "Thomson Cross Section", Q, "6.6524587321e-29 m^2", true);
    t(c, "E_h,hartree_energy", "Hartree Energy", Q, "4.3597447222071e-18 J", true);

    t(c, "planck_length,l_P", "Planck Length", PL, "1.616255e-35 m", true);
    t(c, "planck_mass,m_P", "Planck Mass", PL, "2.176434e-8 kg", true);
    t(c, "planck_time,t_P", "Planck Time", PL, "5.391247e-44 s", true);
    t(c, "planck_temperature,T_P", "Planck Temperature", PL, "1.416784e32 K", true);

    t(c, "earth_mass,M_earth", "Earth Mass", A, "5.9722e24 kg", true);
    t(c, "earth_radius,R_earth", "Earth Equatorial Radius", A, "6378137 m", true);
    t(c, "solar_mass,M_sun", "Solar Mass", A, "1.98847e30 kg", true);
    t(c, "solar_radius,R_sun", "Solar Radius", A, "6.957e8 m", true);
    t(c, "solar_luminosity,L_sun", "Solar Luminosity", A, "3.828e26 W", true);
    t(c, "moon_mass,M_moon", "Moon Mass", A, "7.342e22 kg", true);
    t(c, "hubble,H0", "Hubble Constant", A, "67.4 km/(s*Mpc)", true);
}

fn answer(c: &Calculator, idx: usize) -> Expr {
    c.answers.borrow().get(idx).cloned().unwrap_or_else(|| throw("No previous answer"))
}