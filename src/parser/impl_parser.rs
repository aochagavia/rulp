use super::*;
use builder::Relation;
use utils::read_file_contents;

#[derive(Debug, PartialEq)]
enum LineType {
	Variable,
	Constraint,
	Objective,
	Comment
}

#[derive(Debug, PartialEq)]
enum Component {
	Variable(Variable),
	Constraint(Constraint),
	Objective(Objective),
	Comment
}


impl ParserBase for Parser {
	/// Constructor for Components struct.
	///
	/// Takes a string input to be parsed.
	fn parse_components_from_text(text: &str) -> Components {
		let p = Parser::new();
		p.get_components(text)
	}

	/// Constructor for Components struct.
	/// 
	/// Takes a file input to be read.
	fn parse_components_from_file(file: &mut File) -> Components {
		Self::parse_components_from_text(&read_file_contents(file))
	}

	/// Constructor for Lp struct.
	///
	/// Takes a string input to be parsed and a Builder struct.
	///
	/// # Examples
	/// ```
	/// # #[macro_use]
	/// extern crate rulinalg;
	/// # extern crate rulp;
	/// use rulp::parser::*;
	/// use rulp::builder::*;
	/// use rulp::lp::Optimization;
	///
	/// # fn main() {
	///		let text_problem = "	
	///			var television;
	///			var newspaper;
	///			var radio;
	///
	///			maximize objective: 100000.*television + 40000.*newspaper + 18000.*radio;
	///
	///			subject to constraint_1: 20.*television + 6.*newspaper + 3.*radio <= 182.;
	///			subject to constraint_2: newspaper <= 10.;
	///			subject to constraint_3: -1.*television + -1.*newspaper + radio <= 0.;
	///			subject to constraint_4: -9.*television + newspaper + radio <= 0.;
	///
	///		";
	///		let builder = Builder::new();
	///		let lp = Parser::lp_from_text(text_problem, builder);
	///
	/// 	let expected_A = matrix![
	/// 			20.0,   6.0,   3.0, 1.0, 0., 0., 0.;
	///   			0.0,   1.0,   0.0, 0., 1.0, 0., 0.;
	///			   -1.0,  -1.0,   1.0, 0., 0., 1.0, 0.;
	///   		   -9.0,   1.0,   1.0, 0., 0., 0., 1.0
	/// 	];
	///
	/// 	let expected_b = vec![182., 10., 0., 0.];
	/// 	let expected_c = vec![100000., 40000., 18000., 0., 0., 0., 0.];
	///
	/// 	assert_matrix_eq!(lp.A, expected_A);
	/// 	assert_eq!(lp.b, expected_b);
	/// 	assert_eq!(lp.c, expected_c);
	/// 	assert_eq!(lp.optimization, Optimization::Max);
	/// # }
	/// ```
	fn lp_from_text<B: BuilderBase>(text: &str, mut builder: B) -> Lp {
		let components = Self::parse_components_from_text(text);

		for v in components.variables {
			builder.add_variable(v);
		}

		for c in components.constraints {
			builder.add_constraint(c);
		}

		builder.add_objective(components.objective);

		builder.build_lp()
	}

	/// Constructor for Lp struct.
	///
	/// Takes a file input to be read and a Builder struct.
	fn lp_from_file<B: BuilderBase>(file: &mut File, builder: B) -> Lp {
		Self::lp_from_text(&read_file_contents(file), builder)
	}
}

impl Parser {
	fn new() -> Self {
		Parser {
			variable_declaration_regex: Regex::new(r"var\s+(?P<name>\w+)\s*").unwrap(),
			variable_regex: Regex::new(r"((?:\s*(?P<sign>-)?\s*)(?P<coeff>\d+\.?\d*)\s*\*\s*)?(?P<name>\w+)").unwrap(),
			objective_regex: Regex::new(r"(?P<type>minimize|maximize)\s+(?P<name>\w+)\s*:\s*(?P<equation>[^;]*)").unwrap(),
			equation_component_regex: Regex::new(r"^(?P<vars>[\w\s\*\.\+-]*)\s*((?P<type>==|<=|>=)\s*(?P<constant>\d+\.?\d*)\s*)?$").unwrap(),
			constraint_regex: Regex::new(r"subject to (?P<name>\w*):\s*(?P<terms>[^=><]+?)\s*(?P<type>==|<=|>=)\s*?(?P<constant>\d+\.?\d*)\s*?").unwrap()
		}
	}

	fn get_components(&self, text: &str) -> Components {
		let components: Vec<Component> = text
			.split(';')
			.map(|line| line.trim())
			.filter(|line| line.len() > 0)
			.map(|line| self.component_from_line(line))
			.filter(|component| *component != Component::Comment)
			.collect();

		let mut variables = vec![];
		let mut constraints = vec![];
		let mut objective = None;

		for c in components {
			match c {
				Component::Variable(var) => {
					variables.push(var);
				},
				Component::Constraint(con) => {
					constraints.push(con);
				},
				Component::Objective(obj) => {
					objective = Some(obj);
				},
				Component::Comment => {}
			}
		}

		Components {
			variables: variables,
			constraints: constraints,
			objective: objective.expect("No objective function provided!")
		}
	}

	fn component_from_line(&self, line: &str) -> Component {
		match self.get_line_type(line) {
			LineType::Variable => {
				Component::Variable(self.parse_variable_declaration(line))
			},
			LineType::Constraint => {
				Component::Constraint(self.parse_constraint(line))
			},
			LineType::Objective => {
				Component::Objective(self.parse_objective(line))
			},
			LineType::Comment => Component::Comment,
		}
	}

	fn get_line_type(&self, line: &str) -> LineType {
		if line.contains("#") {
			return LineType::Comment;
		} else if line.contains("var") {
			return LineType::Variable;
		} else if line.contains("minimize") || line.contains("maximize") {
			return LineType::Objective;
		} else if line.contains("subject to") {
			return LineType::Constraint;
		} 

		panic!("Unknown line type for \"{:?}\"", line);
	}

	fn parse_variable_declaration(&self, data: &str) -> Variable {
		let caps = self.variable_declaration_regex.captures(data).unwrap();
		return Variable {
			name: caps["name"].to_string(),
			coefficient: 0.,
		}	
	}

	fn parse_constraint(&self, data: &str) -> Constraint {
		let caps = self.constraint_regex.captures(data).unwrap();
		let name = caps["name"].to_string();
		let relation = if caps["type"].contains("<") {
			Relation::LessThanOrEqual
		} else if caps["type"].contains(">") {
			Relation::GreaterThanOrEqual
		} else {
			Relation::Equal
		};
			
		let constant = caps["constant"].parse::<f64>().unwrap();
		let variables = self.parse_objective_vars(&caps["terms"]);

		Constraint {
			name: name,
			variables: variables,
			constant: constant,
			relation: relation
		}

	}

	fn parse_objective(&self, data: &str) -> Objective {
		let caps = self.objective_regex.captures(data).expect("Invalid objective!");

		Objective {
			name: caps["name"].to_string(),
			variables: self.parse_objective_vars(&caps["equation"]),
			maximize: caps["type"].contains("maximize")
		}
	}

	fn parse_objective_vars(&self, data: &str) -> Vec<Variable> {
		data.split('+').map(|s| s.trim()).map(|var| self.parse_variable(var)).collect()
	}

	fn parse_variable(&self, data: &str) -> Variable {
		let caps = self.variable_regex.captures(data).unwrap();
		let name = caps["name"].to_string();
		let sign = match caps.name("sign") {
			None => {
				1.
			},
			Some(_) => {
				-1.
			}
		};

		let coefficient = match caps.name("coeff") {
			None => {
				1.
			}, 
			Some(coeff) => {
				coeff.as_str().parse::<f64>().unwrap()
			}
		};

		Variable {
				name: name,
				coefficient: coefficient * sign,
		}
	}
}


#[cfg(test)]
mod LPParser_tests {
	use super::*;

	#[test]
	fn line_type_test() {
		let p = Parser::new();

		let comment = "# This is a comment;";
		let variable = "var a;";
		let min_objective= "minimize obj: 3*a;";
		let max_objective= "maximize obj: 3*a;";
		let constraint = "subject to foo_constraint: a == 10;";

		assert_eq!(p.get_line_type(comment), LineType::Comment);
		assert_eq!(p.get_line_type(variable), LineType::Variable);
		assert_eq!(p.get_line_type(min_objective), LineType::Objective);
		assert_eq!(p.get_line_type(max_objective), LineType::Objective);
		assert_eq!(p.get_line_type(constraint), LineType::Constraint);
	}


	#[test]
	fn parse_variable_declaration_test() {
		let p = Parser::new();

		let variable = "var a;";
		let expected = Variable {
			name: "a".to_string(),
			coefficient: 0.,
		};

		assert_eq!(p.parse_variable_declaration(variable), expected);
	}

	#[test]
	fn parse_vars_test() {
		let p = Parser::new();

		let data = "3.5*a + 1.5*b + -0.5*c";
		let expected = vec![
			generate_var("a".to_string(), 3.5),
			generate_var("b".to_string(), 1.5),
			generate_var("c".to_string(), -0.5),
		];

		assert_eq!(p.parse_objective_vars(data), expected);
	}

	fn generate_var(name: String, coeff: f64) -> Variable {
		Variable {
			name: name,
			coefficient: coeff,
		}
	}
}