//================ Syntax matcher (Regex)============================

use regex::Regex;

#[derive(Debug, PartialEq)]
pub enum ParsedCommand {
    CreateArray { name: String },
    AddElement { value: i32, array_name: String },
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
        let val_str = entity_number.find(input)
            .expect("LLM-Parser Error: Missing a number value to insert.")
            .as_str();
        let value = val_str.parse::<i32>().unwrap();

        let array_name = entity_name.find_iter(input)
            .map(|m| m.as_str().to_string())
            .find(|w| !reserved_keywords.contains(&w.to_lowercase().as_str()))
            .expect("LLM-Parser Error: Missing destination array identifier.");

        ParsedCommand::AddElement { value, array_name }

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
                self.stream.extend_from_slice(&index.to_be_bytes()); // 8-byte index
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
//=================================================================

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
                    let index = usize::from_be_bytes(bytecode[pc..pc + 8].try_into().unwrap());
                    pc += 8;

                    let array_name = self.read_string_metadata(bytecode, &mut pc);
                    let array_instance = self.heap.get_mut(&array_name)
                        .expect("VM Runtime Panic: Target reference array not found on heap map.");
                    
                    if index >= array_instance.len() {
                        panic!("VM Runtime Panic: Array index {} out of bounds for layout.", index);
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
}
//==========================================================





