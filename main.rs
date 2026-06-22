//================ Syntax matcher (Regex)============================

use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum ParsedCommand {
    CreateArray { name: String },
    AddElement { expr: MathExpr, array_name: String }, // Swapped i32 for MathExpr
    DeleteElement { index: usize, array_name: String },
}//---> This is the output that comes from the function below.

pub fn match_and_extract_flexible(input: &str) -> ParsedCommand {
    let intent_create = Regex::new(r"(?i)\b(cr[ea]{1,2}t[e]?|make|new|init|generate)\b").unwrap();
    let intent_add    = Regex::new(r"(?i)\b(a[d]{1,2}|push|insert|append|put|place)\b").unwrap();
    let intent_delete = Regex::new(r"(?i)\b(d[el]{1,3}t[e]?|remove|drop|clear|pop|erase)\b").unwrap();
    let entity_number = Regex::new(r"\b\d+\b").unwrap();
    let entity_name   = Regex::new(r"\b[a-zA-Z_]\w*\b").unwrap();
    let reserved_keywords = vec![
        "create", "creete", "make", "new", "init", "generate",
        "add", "push", "insert", "append", "put", "place",
        "delete", "delte", "remove", "drop", "clear", "pop", "erase",
        "array", "aray", "list", "index", "idx", "ind", "from", "form", "to", "into"
    ];
    if intent_create.is_match(input) {
        let name = entity_name.find_iter(input)
            .map(|m| m.as_str().to_string())
            .find(|w| !reserved_keywords.contains(&w.to_lowercase().as_str()))
            .expect("LLM-Parser Error: Could not determine your target array name.");

        ParsedCommand::CreateArray { name }

    } else if intent_add.is_match(input) {
	let array_name = entity_name.find_iter(input)
	    .map(|m| m.as_str().to_string())
	    .find(|w| !reserved_keywords.contains(&w.to_lowercase().as_str()))
	    .expect("LLM-Parser Error: Missing destination array identifier.");
	let clean_math_target = input.replace(&array_name, "");
	let math_tokens = tokenize_math(&clean_math_target); 
	let mut parser = PrattParser::new(math_tokens);
	let expr = parser.parse_expression(0);

	ParsedCommand::AddElement { expr, array_name }

    } else if intent_delete.is_match(input) {
        let idx_str = entity_number.find(input)
            .expect("LLM-Parser Error: Missing structural target index for elimination.")
            .as_str();
        let index = idx_str.parse::<usize>().unwrap();

        let array_name = entity_name.find_iter(input)
            .map(|m| m.as_str().to_string())
            .find(|w| !reserved_keywords.contains(&w.to_lowercase().as_str()))
            .expect("LLM-Parser Error: Missing source array identifier.");

        ParsedCommand::DeleteElement { index, array_name }

    } else {
        panic!("LLM-Parser Error: Could not figure out what you want to do with '{}'", input);
    }
}


//=================================================================

//==================Abstract syntax tree==========================

#[derive(Debug, PartialEq, Clone)]
pub enum MathOpType {
    Add,
    Subtract,
    Multiply,
    Divide,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(i32),
    Plus,
    Minus,
    Multiply,
    Divide,
    EOF,
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
    CreateArray { 
        name: String 
    },
    AddToArray { 
        expr: MathExpr, 
        array_name: String 
    },
    DeleteFromArray { 
        index: usize, 
        array_name: String 
    },
    EvaluateMath(MathExpr),
}

pub fn tokenize_math(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            ' ' | '\t' | '\r' | '\n' => { chars.next(); } // Skip whitespace
            '+' => { tokens.push(Token::Plus); chars.next(); }
            '-' => { tokens.push(Token::Minus); chars.next(); }
            '*' => { tokens.push(Token::Multiply); chars.next(); }
            '/' => { tokens.push(Token::Divide); chars.next(); }
            '0'..='9' => {
                let mut num_str = String::new();
                while let Some(&digit) = chars.peek() {
                    if digit.is_ascii_digit() {
                        num_str.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num_str.parse::<i32>().unwrap()));
            }
            _ => { chars.next(); } // Ignore non-math characters (or handle gracefully)
        }
    }
    tokens.push(Token::EOF);
    tokens
}
//----------Pratt Parser--------------------
pub struct PrattParser {
    tokens: Vec<Token>,
    position: usize,
}

impl PrattParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens[self.position].clone();
        if tok != Token::EOF {
            self.position += 1;
        }
        tok
    }

    fn get_binding_power(op: &Token) -> u8 {
        match op {
            Token::Plus | Token::Minus => 10,
            Token::Multiply | Token::Divide => 20,
            _ => 0,
        }
    }

    pub fn parse_expression(&mut self, min_bp: u8) -> MathExpr {
        let token = self.advance();
        let mut left = match token {
            Token::Number(val) => MathExpr::Number(val),
            unsupported => panic!("Parser Error: Expected a number, found {:?}", unsupported),
        };

        loop {
            let next_op = self.peek();
            let bp = Self::get_binding_power(next_op);
            
            if bp <= min_bp {
                break; 
            }

            let op_token = self.advance();
            let op_type = match op_token {
                Token::Plus => MathOpType::Add,
                Token::Minus => MathOpType::Subtract,
                Token::Multiply => MathOpType::Multiply,
                Token::Divide => MathOpType::Divide,
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

//=============================================================

//==========Byte code serialization engine=====================

pub struct BytecodeCompiler {
    pub stream: Vec<u8>,
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        Self { stream: Vec::new() }
    }

    pub fn compile_statement(&mut self, statement: ASTStatement) {
        match statement {
            ASTStatement::CreateArray { name } => {
                self.stream.push(0x06); // Opcode
                self.write_string(&name);
            }
            ASTStatement::AddToArray { expr, array_name } => {
                self.compile_expression(expr);
                self.stream.push(0x07); // Opcode
                self.write_string(&array_name);
            }
            ASTStatement::DeleteFromArray { index, array_name } => {
                self.stream.push(0x08); // Opcode
                self.stream.extend_from_slice(&(index as u64).to_be_bytes());
                self.write_string(&array_name);
            }
            ASTStatement::EvaluateMath(expr) => {
                self.compile_expression(expr);
            }
        }
    }

    fn compile_expression(&mut self, expr: MathExpr) {
        match expr {
            MathExpr::Number(val) => {
                self.stream.push(0x01); // OpPush Opcode
                self.stream.extend_from_slice(&val.to_be_bytes()); // 4-byte payload
            }
            MathExpr::BinaryOp { op, left, right } => {
                self.compile_expression(*left);
                self.compile_expression(*right);
                
                let opcode = match op {
                    MathOpType::Add => 0x02,
                    MathOpType::Subtract => 0x03,
                    MathOpType::Multiply => 0x04,
                    MathOpType::Divide => 0x05,
                };
                self.stream.push(opcode);
            }
        }
    }

    fn write_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let length = bytes.len() as u32; 
        self.stream.extend_from_slice(&length.to_be_bytes()); 
        self.stream.extend_from_slice(bytes); 
    }
}

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
//=================================================================

//============ Virtual Machine ===================================

//============ Virtual Machine ===================================

use std::collections::HashMap;

pub struct VirtualMachine {
    stack: Vec<i32>,
    heap: HashMap<String, Vec<i32>>,
}

impl VirtualMachine {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            heap: HashMap::new(),
        }
    }

    pub fn run(&mut self, bytecode: &[u8]) {
        let mut pc = 0; // Program Counter pointer

        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            pc += 1;

            match opcode {
                0x01 => {
                    if pc + 4 > bytecode.len() {
                        panic!("VM Execution Error: Unexpected EOF reading OpPush payload.");
                    }
                    let val = i32::from_be_bytes(bytecode[pc..pc + 4].try_into().unwrap());
                    pc += 4;
                    self.stack.push(val);
                }
                0x02 => {
                    let right = self.stack.pop().expect("VM Error: Math Stack Underflow during Addition.");
                    let left = self.stack.pop().expect("VM Error: Math Stack Underflow during Addition.");
                    self.stack.push(left + right);
                }
                0x03 => {
                    let right = self.stack.pop().expect("VM Error: Math Stack Underflow during Subtraction.");
                    let left = self.stack.pop().expect("VM Error: Math Stack Underflow during Subtraction.");
                    self.stack.push(left - right);
                }
                0x04 => {
                    let right = self.stack.pop().expect("VM Error: Math Stack Underflow during Multiplication.");
                    let left = self.stack.pop().expect("VM Error: Math Stack Underflow during Multiplication.");
                    self.stack.push(left * right);
                }
                0x05 => {
                    let right = self.stack.pop().expect("VM Error: Math Stack Underflow during Division.");
                    let left = self.stack.pop().expect("VM Error: Math Stack Underflow during Division.");
                    if right == 0 {
                        panic!("VM Runtime Panic: Division by zero exception encountered.");
                    }
                    self.stack.push(left / right);
                }
                0x06 => {
                    let array_name = self.read_string_metadata(bytecode, &mut pc);
                    self.heap.insert(array_name, Vec::new());
                }
                0x07 => {
                    let array_name = self.read_string_metadata(bytecode, &mut pc);
                    let val = self.stack.pop().expect("VM Error: Calculation stack empty; nothing to add to array.");
                    
                    let array_instance = self.heap.get_mut(&array_name)
                        .expect("VM Runtime Panic: Target reference array not found on heap map.");
                    array_instance.push(val);
                }
                0x08 => {
                    if pc + 8 > bytecode.len() {
                        panic!("VM Execution Error: Unexpected EOF reading OpDeleteFromArray index payload.");
                    }
                    let index = u64::from_be_bytes(bytecode[pc..pc + 8].try_into().unwrap()) as usize;
                    pc += 8;

                    let array_name = self.read_string_metadata(bytecode, &mut pc);
                    let array_instance = self.heap.get_mut(&array_name)
                        .expect("VM Runtime Panic: Target reference array not found on heap map.");
                    
                    if index >= array_instance.len() {
                        panic!("VM Runtime Panic: Array index {} out of bounds for current size ({}).", index, array_instance.len());
                    }
                    array_instance.remove(index);
                }

                unsupported => {
                    panic!("VM Fatal Error: Invalid or unrecognized Machine Opcode code raw byte '0x{:X?}' detected.", unsupported);
                }
            }
        }
    }
    fn read_string_metadata(&self, bytecode: &[u8], pc: &mut usize) -> String {
        if *pc + 4 > bytecode.len() {
            panic!("VM Execution Error: Unexpected EOF while fetching string size prefix bytes.");
        }
        let length = u32::from_be_bytes(bytecode[*pc..*pc + 4].try_into().unwrap()) as usize;
        *pc += 4;

        if *pc + length > bytecode.len() {
            panic!("VM Execution Error: Unexpected EOF while fetching string characters.");
        }
        let string_bytes = &bytecode[*pc..*pc + length];
        *pc += length;

        String::from_utf8(string_bytes.to_vec())
            .expect("VM Execution Error: Failed parsing identifier name into dynamic UTF-8 string layout.")
    }
    pub fn print_state(&self) {
        println!("Working Evaluation Stack: {:?}", self.stack);
        println!("Heap Arrays Status:");
        for (name, contents) in &self.heap {
            println!("  -> Array '{}': {:?}", name, contents);
        }
    }
}
//==========================================================





