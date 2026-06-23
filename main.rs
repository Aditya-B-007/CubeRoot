use regex::Regex;

//================ Syntax Matcher (Regex Frontend) ============================
#[derive(Debug, Clone, PartialEq)]
pub enum MathExpr {
    Literal(i64),
    Binary { op: char, left: Box<MathExpr>, right: Box<MathExpr> },
    // Expandable math nodes go here
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticObject {
    // Action Verbs
    ActionCreate,
    ActionAdd,
    ActionDelete,
    ActionUpdate,
    
    // Connectors / Prepositions
    ConnectorTo,
    ConnectorFrom,
    ConnectorAt,
    
    // Abstract Wrapped Primitives
    Identifier(String),
    NumericValue(usize), 
    Expression(MathExpr),
    
    // Structural Data Types (For your extension requirement)
    TargetTypeArray,
    TargetTypeMap,
    TargetTypeStack,
    //Pratt terms
    Plus,
    Minus,
    Multiply,
    Divide,
    LeftParen,
    RightParen,
    EOF,
}

pub struct SemanticLexer;

impl SemanticLexer {
    pub fn tokenize(input: &str) -> Vec<SemanticObject> {
        let mut objects = Vec![];
        let re_word = Regex::new(r"[a-zA-Z_]\w*|\d+|[+\-*/]").unwrap();
        
        for mat in re_word.find_iter(input) {
            let txt = mat.as_str();
            let lower = txt.to_lowercase();
            
            let obj = match lower.as_str() {
                // Map actions
                "create" | "make" | "new" | "init" => SemanticObject::ActionCreate,
                "add" | "push" | "insert" | "append" => SemanticObject::ActionAdd,
                "delete" | "remove" | "drop" | "erase" => SemanticObject::ActionDelete,
                "update" | "set" | "change" => SemanticObject::ActionUpdate,
                
                // Map connectors
                "to" | "into" | "in" => SemanticObject::ConnectorTo,
                "from" | "out" => SemanticObject::ConnectorFrom,
                "at" | "on" | "index" | "key" => SemanticObject::ConnectorAt,
                
                // Map future structure targets
                "array" | "list" | "vector" => SemanticObject::TargetTypeArray,
                "map" | "dictionary" | "dict" => SemanticObject::TargetTypeMap,
                "stack" => SemanticObject::TargetTypeStack,
                
                // Map primitives wrapped inside semantic objects
                _ if txt.chars().next().unwrap().is_ascii_digit() => {
                    SemanticObject::NumericValue(txt.parse().unwrap())
                }
                _ => SemanticObject::Identifier(txt.to_string()),
            };
            objects.push(obj);
        }
        objects
    }
}


pub enum ExpandedCommand {
    Create { data_type: SemanticObject, name: String },
    Insert { payload: SemanticObject, target: String, key_or_idx: Option<SemanticObject> },
    Remove { target: String, key_or_idx: SemanticObject },
}

pub fn match_and_extract_flexible(input: &str) -> ASTStatement {
    let tokens = tokenize_unified(input);
    if tokens.is_empty() || tokens[0] == SemanticObject::EOF {
        panic!("Syntax Error: Received empty expression line string.");
    }

    match &tokens[0] {
        SemanticObject::ActionCreate => {
            // Expression: "Create my_array"
            let name = tokens.iter()
                .find_map(|t| if let SemanticObject::Identifier(id) = t { Some(id.clone()) } else { None })
                .expect("Parser Error: Missing target identifier name string for array initialization.");

            ASTStatement::CreateArray { name }
        }

        SemanticObject::ActionAdd => {
            // Expression: "Add 5 * (10 + 2) into user_list"
            let to_index = tokens.iter().position(|t| matches!(t, SemanticObject::ConnectorTo))
                .expect("Syntax Error: Missing structural vector directional destination keyword ('to' / 'into').");
            
            let array_name = match tokens.get(to_index + 1) {
                Some(SemanticObject::Identifier(name)) => name.clone(),
                _ => panic!("Syntax Error: Token directly following 'to' connector must be a valid destination identifier.")
            };

            // Slice out only the math symbols: sitting between "Add" and "into"
            let mut math_tokens = tokens[1..to_index].to_vec();
            math_tokens.push(SemanticObject::EOF); // Ensure Pratt parser has an EOF marker safely appended

            let mut pratt = PrattParser::new(math_tokens);
            let expr = pratt.parse_expression(0);

            ASTStatement::AddToArray { expr, array_name }
        }

        SemanticObject::ActionDelete => {
            // Expression: "Delete index 4 from user_list"
            let index = tokens.iter()
                .find_map(|t| if let SemanticObject::Number(num) = t { Some(*num as usize) } else { None })
                .expect("Syntax Error: Missing numerical lookup array target index for deletion operations.");

            let array_name = tokens.iter()
                .skip_while(|t| !matches!(t, SemanticObject::ConnectorFrom))
                .nth(1)
                .and_then(|t| if let SemanticObject::Identifier(id) = t { Some(id.clone()) } else { None })
                .expect("Syntax Error: Missing targeted source array identifier after 'from' keyword.");

            ASTStatement::DeleteFromArray { index, array_name }
        }

        // If the line doesn't start with an action keyword, fall back to parsing it as pure math code
        _ => {
            let mut pratt = PrattParser::new(tokens.clone());
            ASTStatement::EvaluateMath(pratt.parse_expression(0))
        }
    }
}


// Simple internal helper method for the enum
impl SemanticObject {
    fn is_identifier_equal(&self, text: &str) -> bool {
        if let SemanticObject::Identifier(id) = self { id == text } else { false }
    }
}


//================== Abstract Syntax Tree & Lexer ==========================

#[derive(Debug, PartialEq, Clone)]
pub enum MathOpType {
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MathExpr {
    Number(i32),
    BinaryOp {
        op: MathOpType,
        left: Box<MathExpr>,
        right: Box<MathExpr>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum ASTStatement {
    CreateArray { name: String },
    AddToArray { expr: MathExpr, array_name: String },
    DeleteFromArray { index: usize, array_name: String },
    EvaluateMath(MathExpr),
}

pub fn tokenize_unified(input: &str) -> Vec<SemanticObject> {
    let mut tokens = Vec::new();
    // This splits words, digits, and mathematical operator symbols
    let re_word = Regex::new(r"[a-zA-Z_]\w*|\d+|[+\-*/()]").unwrap();
    
    for mat in re_word.find_iter(input) {
        let txt = mat.as_str();
        let lower = txt.to_lowercase();
        
        let obj = match lower.as_str() {
            // Step A: Map actions
            "create" | "make" | "new" | "init" => SemanticObject::ActionCreate,
            "add" | "push" | "insert" | "append" => SemanticObject::ActionAdd,
            "delete" | "remove" | "drop" => SemanticObject::ActionDelete,
            
            // Step B: Map english text context anchors
            "to" | "into" | "in" => SemanticObject::ConnectorTo,
            "from" | "out" => SemanticObject::ConnectorFrom,
            
            // Step C: Map arithmetic operators
            "+" => SemanticObject::Plus,
            "-" => SemanticObject::Minus,
            "*" => SemanticObject::Multiply,
            "/" => SemanticObject::Divide,
            "(" => SemanticObject::LeftParen,
            ")" => SemanticObject::RightParen,
            
            // Step D: Parse primitives
            _ if txt.chars().next().unwrap().is_ascii_digit() => {
                SemanticObject::Number(txt.parse().unwrap())
            }
            _ => SemanticObject::Identifier(txt.to_string()),
        };
        tokens.push(obj);
    }
    tokens.push(SemanticObject::EOF);
    tokens
}


//========================= Pratt Parser Engine ============================
pub struct PrattParser {
    tokens: Vec<SemanticObject>,
    position: usize,
}

impl PrattParser {
    pub fn new(tokens: Vec<SemanticObject>) -> Self {
        Self { tokens, position: 0 }
    }

    fn peek(&self) -> &SemanticObject {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> SemanticObject {
        let tok = self.tokens[self.position].clone();
        if tok != SemanticObject::EOF {
            self.position += 1;
        }
        tok
    }

    fn get_binding_power(op: &SemanticObject) -> u8 {
        match op {
            SemanticObject::Plus | SemanticObject::Minus => 10,
            SemanticObject::Multiply | SemanticObject::Divide => 20,
            _ => 0,
        }
    }

    pub fn parse_expression(&mut self, min_bp: u8) -> MathExpr {
        let token = self.advance();
        
        // --- NUD Phase (Null Denotation) ---
        let mut left = match token {
            SemanticObject::Number(val) => MathExpr::Number(val),
            
            // Support for referencing variables inside expressions!
            SemanticObject::Identifier(name) => MathExpr::Variable(name),
            
            SemanticObject::LeftParen => {
                let inner_expr = self.parse_expression(0);
                match self.advance() {
                    SemanticObject::RightParen => inner_expr,
                    other => panic!("Parser Error: Unmatched opening parenthesis. Found {:?}", other),
                }
            }
            unsupported => panic!("Parser Error: Expected primary math expression, found {:?}", unsupported),
        };

        // --- LED Phase (Left Denotation) ---
        loop {
            let next_op = self.peek();
            let bp = Self::get_binding_power(next_op);
            
            if bp <= min_bp {
                break; 
            }

            let op_token = self.advance();
            let op_type = match op_token {
                SemanticObject::Plus => MathOpType::Add,
                SemanticObject::Minus => MathOpType::Subtract,
                SemanticObject::Multiply => MathOpType::Multiply,
                SemanticObject::Divide => MathOpType::Divide,
                _ => unreachable!(),
            };

            let right = self.parse_expression(bp);
            
            left = MathExpr::BinaryOp {
                op: op_type,
                left: Box::new(left),
                right: Box::new(right),
            };
        }

        left
    }
}


//=================== Data Transformation Binding Layer =======================

impl From<ParsedCommand> for ASTStatement {
    fn from(command: ParsedCommand) -> Self {
        match command {
            ParsedCommand::CreateArray { name } => {
                ASTStatement::CreateArray { name }
            }
            ParsedCommand::AddElement { expr, array_name } => {
                ASTStatement::AddToArray { expr, array_name }
            }
            ParsedCommand::DeleteElement { index, array_name } => {
                ASTStatement::DeleteFromArray { index, array_name }
            }
        }
    }
}

//================== LLVM IR SSA Compilation Backend =====================

pub struct LlvmIrCompiler {
    pub register_counter: usize,
}

impl LlvmIrCompiler {
    pub fn new() -> Self {
        Self { register_counter: 0 }
    }

    fn next_register(&mut self) -> String {
        self.register_counter += 1;
        format!("%t{}", self.register_counter)
    }
    pub fn compile_statement(&mut self, statement: ASTStatement) -> String {
        match statement {
            ASTStatement::CreateArray { name } => {
                // In a true LLVM system context, dynamic data structures represent global heap allocations.
                // We'll declare a comment and a standard basic tracking metadata variable link.
                format!("; --- Structural Mapping: Declaring Memory Instance '{}' ---\n", name)
            }
            ASTStatement::AddToArray { expr, array_name } => {
                let mut stream = format!("; --- Structural Mapping: Appending Calculation to List '{}' ---\n", array_name);
                let (expr_ir, final_reg) = self.compile_expression(expr);
                stream.push_str(&expr_ir);
                stream.push_str(&format!("; Target output sequence is resolved in register {}\n", final_reg));
                stream
            }
            ASTStatement::DeleteFromArray { index, array_name } => {
                format!("; --- Structural Mapping: Erasing Index Pointer ({}) from collection '{}' ---\n", index, array_name)
            }
            ASTStatement::EvaluateMath(expr) => {
                let (expr_ir, _) = self.compile_expression(expr);
                expr_ir
            }
        }
    }

    /// Compiles algebraic trees into linear static single assignment (SSA) registers
    pub fn compile_expression(&mut self, expr: MathExpr) -> (String, String) {
        match expr {
            MathExpr::Number(val) => {
                // Literals do not require an active assembly pipeline instruction line
                (String::new(), val.to_string())
            }
            MathExpr::BinaryOp { op, left, right } => {
                let (left_code, left_target) = self.compile_expression(*left);
                let (right_code, right_target) = self.compile_expression(*right);

                let my_register = self.next_register();
                let llvm_op = match op {
                    MathOpType::Add => "add",
                    MathOpType::Subtract => "sub",
                    MathOpType::Multiply => "mul",
                    MathOpType::Divide => "sdiv",
                };

                let my_instruction = format!(
                    "    {} = {} i32 {}, {}\n",
                    my_register, llvm_op, left_target, right_target
                );

                let total_accumulated_code = format!("{}{}{}", left_code, right_code, my_instruction);
                (total_accumulated_code, my_register)
            }
        }
    }
}
