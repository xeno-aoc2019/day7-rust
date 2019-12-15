use std::fmt;
use std::fs::File;
use std::path::Path;
use std::io::{BufRead, BufReader, Result, Lines};
use std::fmt::Formatter;

struct Instruction {
    opcode: i32,
    steps_next: usize,
}

const I_ADD: Instruction = Instruction { opcode: 1, steps_next: 4 };
const I_MUL: Instruction = Instruction { opcode: 2, steps_next: 4 };
const I_IN: Instruction = Instruction { opcode: 3, steps_next: 2 };
const I_OUT: Instruction = Instruction { opcode: 4, steps_next: 2 };
const I_JT: Instruction = Instruction { opcode: 5, steps_next: 3 };
const I_JF: Instruction = Instruction { opcode: 6, steps_next: 3 };
const I_LT: Instruction = Instruction { opcode: 7, steps_next: 4 };
const I_EQ: Instruction = Instruction { opcode: 8, steps_next: 4 };
const I_HALT: Instruction = Instruction { opcode: 99, steps_next: 0 };

const MODE_REF: i32 = 0;
const MODE_VAL: i32 = 1;

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.opcode {
            1 => write!(f, "I_ADD({})", self.opcode),
            2 => write!(f, "I_MUL({})", self.opcode),
            3 => write!(f, "I_IN({})", self.opcode),
            4 => write!(f, "I_OUT({})", self.opcode),
            5 => write!(f, "I_JT({})", self.opcode),
            6 => write!(f, "I_JF({})", self.opcode),
            7 => write!(f, "I_LT({})", self.opcode),
            8 => write!(f, "I_EQ({})", self.opcode),
            _ => write!(f, "UNKNOWN({}", self.opcode)
        }
    }
}

struct Param {
    value: i32,
    mode: i32,
}

impl Param {
    fn new(value: i32, mode: i32) -> Param {
        Param {
            value,
            mode,
        }
    }

    fn is_valid(&self) -> bool {
        if !(self.mode == 0 || self.mode == 1) { return false; }
        if self.mode == 0 && self.value < 0 { return false; }
        true
    }

    fn is_reference(&self) -> bool {
        return self.mode == MODE_REF;
    }

    fn is_value(&self) -> bool {
        return self.mode == MODE_VAL;
    }
}

struct ParaModes {
    modes: [i32; 3]
}

impl ParaModes {
    fn param_modes(instr: i32) -> [i32; 3] {
        let mut params: [i32; 3] = [0; 3];
        let param_part = (instr - instr % 100) / 100;
        params[0] = param_part % 10;
        params[1] = ((param_part - param_part % 10) / 10) % 10;
        params[2] = ((param_part - (param_part % 100)) / 100) % 10;
//        println!("MODES: instr={} : {} => {},{},{}", instr, param_part, params[0], params[1], params[2]);
        params
    }

    fn new(instr: i32) -> ParaModes {
        ParaModes {
            modes: ParaModes::param_modes(instr)
        }
    }
    fn mode(&self, n: i32) -> i32 {
        match n {
            1 => self.modes[0],
            2 => self.modes[1],
            3 => self.modes[2],
            _ => panic!("Unsupported parameter mode number")
        }
    }
}

impl fmt::Display for ParaModes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Modes({} {} {})", self.modes[0], self.modes[1], self.modes[2])
    }
}

impl fmt::Display for VM {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "VM(ip={} input=", self.ip);
        let mut inp_ind = 0;
        for value in self.inputs.iter() {
            if inp_ind == self.in_p {
                write!(f, "[{}] ", value);
            } else {
                write!(f, "{} ", value);
            }
            inp_ind += 1;
        }
        write!(f, "output=");
        for value in self.outputs.iter() {
            write!(f, "{} ", value);
        }
        write!(f, "program=");
        for value in self.program.iter() {
            write!(f, "{} ", value);
        }
        write!(f, ")")
    }
}

struct VM {
    program: Vec<i32>,
    ip: usize,
    in_p: i32,
    out_p: i32,
    halted: bool,
    inputs: Vec<i32>,
    outputs: Vec<i32>,
}

impl VM {
    fn new(program: Vec<i32>, inputs: Vec<i32>) -> VM {
        VM {
            program,
            ip: 0,
            in_p: 0,
            out_p: 0,
            halted: false,
            inputs,
            outputs: vec!(),
        }
    }

    fn read_mem(&self, addr: i32) -> i32 {
        if addr < 0 {
            println!("Tried to read a negative memory address: {}", addr);
            panic!("Illegal memory access");
        }
        let value = self.program[addr as usize];
        println!("Reading [{}] = {}", addr, value);
        value
    }

    fn write_mem(&mut self, addr: i32, value: i32) {
        if addr < 0 {
            println!("Tried to write to a negative memory address: {}", addr);
            panic!("Illegal memory access");
        }
        println!("Writing [{}] = {}", addr, value);
        self.program[addr as usize] = value;
    }


    fn fetch_instr(&self) -> (Instruction, ParaModes) {
        let instruction = self.program[self.ip];
        let para_modes = ParaModes::new(instruction);
//        println!("Fetching instruction at [{}] = {}", self.ip, instruction);
        let opcode = instruction % 100;
        let instr = match opcode {
            1 => I_ADD,
            2 => I_MUL,
            3 => I_IN,
            4 => I_OUT,
            5 => I_JT,
            6 => I_JF,
            7 => I_LT,
            8 => I_EQ,
            99 => I_HALT,
            _ => {
                println!("Unknown opcode at ip={}: {}", self.ip, opcode);
                panic!("Uknown opcode")
            }
        };
        (instr, para_modes)
    }

    fn fetch_arg(&self, n: usize) -> i32 {
        self.program[self.ip + n]
    }

    fn fetch_arg_value(&self, n: usize, mode: i32) -> i32 {
        let arg = self.program[self.ip + n];
        if mode == MODE_VAL {
            return arg;
        }
        if mode == MODE_REF {
            return self.read_mem(arg);
        }
        panic!("Unknown param mode");
    }

    fn step(&mut self, n: usize) {
        self.ip += n;
    }

    fn goto(&mut self, dest: i32) {
        println!("Goto {}", dest);
        if dest < 0 {
            panic!("Trying to jump out of the program");
        }
        self.ip = dest as usize;
    }

    fn read_input(&mut self) -> i32 {
        let input = self.inputs[self.in_p as usize];
        self.in_p += 1;
        input
    }

    fn i_add(&mut self, modes: &ParaModes) {
        let param1 = self.fetch_arg_value(1, modes.mode(1));
        let param2 = self.fetch_arg_value(2, modes.mode(2));
        let dest = self.fetch_arg(3);
        println!("I_ADD [{}] = {}+{}", dest, param1, param2);
        self.write_mem(dest, param1 + param2);
        self.step(I_ADD.steps_next);
    }

    fn i_mul(&mut self, modes: &ParaModes) {
        let adr1 = self.fetch_arg(1);
        let adr2 = self.fetch_arg(2);
        let param1 = self.fetch_arg_value(1, modes.mode(1));
        let param2 = self.fetch_arg_value(2, modes.mode(2));
        let dest = self.fetch_arg(3);
        println!("I_MUL [{}] = [{}]+[{}]", dest, adr1, adr2);
        println!("I_MUL [{}] = [{}]={}+[{}]={}", dest, adr1, param1, adr2, param2);
        let value = param1 * param2;
        self.write_mem(dest, value);
        self.step(I_MUL.steps_next);
    }

    fn i_input(&mut self) {
        let adr = self.fetch_arg(1);
        let input = self.read_input();
        self.write_mem(adr, input);
        println!("I_INPUT [{}] input:{}", adr, input);
        self.ip = self.ip + I_IN.steps_next;
    }

    fn i_output(&mut self) {
        let adr = self.program[(self.ip + 1) as usize] as usize;
        let output = self.program[adr];
        self.outputs.push(output);
        self.out_p += 1;
        println!("I_OUTPUT: outputting [{}] = {}", adr, output);
        self.ip = self.ip + I_OUT.steps_next;
    }

    fn i_jt(&mut self, modes: &ParaModes) {
        let param = self.fetch_arg_value(1, modes.mode(1));
        let dest = self.fetch_arg_value(2, modes.mode(2));
        let jump = param != 0;
        println!("I_JT {} ->{}:{}", dest, dest, jump);
        if jump {
            self.goto(dest);
        } else {
            self.step(I_JT.steps_next);
        }
    }

    fn i_jf(&mut self, modes: &ParaModes) {
        let param = self.fetch_arg_value(1, modes.mode(1));
        let dest = self.fetch_arg_value(2, modes.mode(2));
        let jump = param == 0;
        println!("I_JF {} ->{}:{}", param, dest, jump);
        if jump {
            self.goto(dest);
        } else {
            self.step(I_JT.steps_next);
        }
    }

    fn i_lt(&mut self, modes: &ParaModes) {
        let param1 = self.fetch_arg_value(1, modes.mode(1));
        let param2 = self.fetch_arg_value(2, modes.mode(2));
        let dest = self.fetch_arg(3);
        let res = if param1 < param2 { 1 } else { 0 };
        println!("I_LT [{}]={} = {}=={}", dest, res, param1, param2);
        self.write_mem(dest, res);
        self.step(I_LT.steps_next);
    }

    fn i_eq(&mut self, modes: &ParaModes) {
        let param1 = self.fetch_arg_value(1, modes.mode(1));
        let param2 = self.fetch_arg_value(2, modes.mode(2));
        let dest = self.fetch_arg(3);
        let res = if param1 == param2 { 1 } else { 0 };
        println!("I_EQ [{}]={} = {}=={}", dest, res, param1, param2);
        self.write_mem(dest, res);
        self.step(I_EQ.steps_next);
    }

    fn i_halt(&mut self) {
        println!("I_HALT");
        self.halted = true;
    }

    fn exec_inst(&mut self) {
        let (instr, modes) = self.fetch_instr();
        let opcode = instr.opcode;
        println!("Executing: {} ip={} {}", opcode, self.ip, modes);
        if opcode == 99 { return self.i_halt(); };
        if opcode == 1 { return self.i_add(&modes); };
        if opcode == 2 { return self.i_mul(&modes); };
        if opcode == 3 { return self.i_input(); };
        if opcode == 4 { return self.i_output(); };
        if opcode == 5 { return self.i_jt(&modes); };
        if opcode == 6 { return self.i_jf(&modes); };
        if opcode == 7 { return self.i_lt(&modes); };
        if opcode == 8 { return self.i_eq(&modes); };
        println!("Unknown instruction: {}, halting", opcode);
        self.i_halt();
    }

    fn is_halted(&self) -> bool {
        self.halted
    }

    fn run(&mut self) {
        println!("start vm={}", self);
        self.ip = 0;
        while !self.is_halted() {
            self.exec_inst();
        }
        println!("end vm={}", self);
    }
}

fn main() {
    let program = read_program();
//    let program = vec!(3, 0, 4, 0, 99);
//    let program = vec!(1, 0, 0, 0, 99);
//    let program = vec!(3, 12, 6, 12, 15, 1, 13, 14, 13, 4, 13, 99, -1, 0, 1, 9);
//    let program = vec!(3, 3, 1105, -1, 9, 1101, 0, 0, 12, 4, 12, 99, 1);
    let mut vm: VM = VM::new(program, vec!(5));
    vm.run();
}

fn read_program() -> Vec<i32> {
    if let Ok(lines) = getLines("input.txt") {
        for maybe_line in lines {
            if let Ok(line) = maybe_line {
                let mut result: Vec<i32> = vec!();
                for item in line.split(",") {
                    let byte: i32 = item.parse().unwrap();
                    result.push(byte);
                }
                return result;
            }
        }
    }
    panic!("no input");
}

fn getLines<P>(file_name: P) -> Result<Lines<BufReader<File>>>
    where P: AsRef<Path>, {
    let file = File::open(file_name)?;
    Ok(BufReader::new(file).lines())
}