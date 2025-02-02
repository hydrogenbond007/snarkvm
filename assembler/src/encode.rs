use crate::error::AssemblerError;
use core::program::instruction::Opcode;
use core::program::instruction::{
    IMM_FLAG_FIELD_BIT_POSITION, IMM_INSTRUCTION_LEN, NO_IMM_INSTRUCTION_LEN,
    REG0_FIELD_BIT_POSITION, REG1_FIELD_BIT_POSITION, REG2_FIELD_BIT_POSITION,
};
use log::debug;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use log::Level::Debug;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum ImmediateFlag {
    NoUsed,
    Used,
}

#[derive(Debug, Default)]
pub struct Encoder {
    pub labels: HashMap<String, u64>,
    pub asm_code: Vec<String>,
    pub pc: u64,
}

impl Encoder {
    pub fn get_reg_index(&self, reg_str: &str) -> Result<usize, AssemblerError> {
        let first = reg_str.chars().nth(0).unwrap();
        assert!(first == 'r', "wrong reg name");
        let res = reg_str[1..].parse();
        if let Ok(reg_index) = res {
            return Ok(reg_index);
        }
        Err(AssemblerError::ParseIntError)
    }

    pub fn get_index_value(&self, op_str: &str) -> Result<(ImmediateFlag, u64), AssemblerError> {
        let re = Regex::new(r"^r\d$").unwrap();

        let src;

        if !op_str.contains("0x") {
            src = op_str.parse();
        } else {
            src = u64::from_str_radix(&op_str[2..].to_string(), 16);
        }

        if src.is_ok() {
            let data: u64 = src.unwrap();
            return Ok((ImmediateFlag::Used, data));
        } else if re.is_match(op_str) {
            let reg_index = self.get_reg_index(op_str)?;
            return Ok((ImmediateFlag::NoUsed, reg_index as u64));
        } else {
            let res = self.labels.get(op_str);
            if res.is_none() {
                return Ok((ImmediateFlag::Used, 0));
            } else {
                return Ok((ImmediateFlag::Used, *res.unwrap()));
            }
        }
    }

    pub fn encode_instruction(&self, raw_inst: &str) -> Result<Vec<String>, AssemblerError> {
        let ops: Vec<_> = raw_inst.trim().split(' ').collect();
        let opcode = ops.first().unwrap().to_lowercase();
        let mut raw_instruction: u64 = 0;
        let mut instuction = Vec::new();

        match opcode.as_str() {
            "mov" | "assert" | "eq" | "neq" | "not" | "gte" | "mload" => {
                debug!("opcode: mov");
                assert!(
                    ops.len() == 3,
                    "{}",
                    format!("{} params len is 2", opcode.as_str())
                );
                let dst_index = self.get_reg_index(ops[1])? as u64;
                let value = self.get_index_value(ops[2])?;
                if value.0 as u8 == ImmediateFlag::Used as u8 {
                    raw_instruction |= 1 << IMM_FLAG_FIELD_BIT_POSITION;
                    instuction.push(format!("{:#x}", value.1));
                } else {
                    raw_instruction |= 1 << (value.1 + REG1_FIELD_BIT_POSITION);
                }

                match opcode.as_str() {
                    "mov" => {
                        raw_instruction |=
                            1 << Opcode::MOV as u8 | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                    }
                    "assert" => {
                        raw_instruction |=
                            1 << Opcode::ASSERT as u8 | 1 << (dst_index + REG2_FIELD_BIT_POSITION)
                    }
                    "eq" => {
                        raw_instruction |=
                            1 << Opcode::EQ as u8 | 1 << (dst_index + REG2_FIELD_BIT_POSITION)
                    }
                    "neq" => {
                        raw_instruction |=
                            1 << Opcode::NEQ as u8 | 1 << (dst_index + REG2_FIELD_BIT_POSITION)
                    }
                    "not" => {
                        raw_instruction |=
                            1 << Opcode::NOT as u8 | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                    }
                    "gte" => {
                        raw_instruction |=
                            1 << Opcode::GTE as u8 | 1 << (dst_index + REG2_FIELD_BIT_POSITION)
                    }
                    "mload" => {
                        raw_instruction |=
                            1 << Opcode::MLOAD as u8 | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                    }
                    _ => panic!("not match opcode:{}", opcode),
                }
            }
            "jmp" | "cjmp" | "call" | "range" => {
                debug!("opcode: cjmp");
                assert!(
                    ops.len() == 2,
                    "{}",
                    format!("{} params len is 1", opcode.as_str())
                );

                let value = self.get_index_value(ops[1])?;
                if value.0 as u8 == ImmediateFlag::Used as u8 {
                    raw_instruction |= 1 << IMM_FLAG_FIELD_BIT_POSITION;
                    instuction.push(format!("{:#x}", value.1));
                } else {
                    raw_instruction |= 1 << (value.1 + REG1_FIELD_BIT_POSITION);
                }
                match opcode.as_str() {
                    "cjmp" => raw_instruction |= 1 << Opcode::CJMP as u8,
                    "jmp" => {
                        raw_instruction |= 1 << Opcode::JMP as u8;
                    }
                    "call" => {
                        raw_instruction |= 1 << Opcode::CALL as u8;
                    }
                    "range" => {
                        raw_instruction |= 1 << Opcode::RC as u8;
                    }
                    _ => panic!("not match opcode:{}", opcode),
                }
            }
            "add" | "mul" | "and" | "or" | "xor" => {
                debug!("opcode: arithmatic");
                assert!(
                    ops.len() == 4,
                    "{}",
                    format!("{} params len is 3", opcode.as_str())
                );
                let dst_index = self.get_reg_index(ops[1])? as u64;
                let op1_index = self.get_reg_index(ops[2])? as u64;
                let op2_value = self.get_index_value(ops[3])?;

                if op2_value.0 as u8 == ImmediateFlag::Used as u8 {
                    raw_instruction |= 1 << IMM_FLAG_FIELD_BIT_POSITION;
                    instuction.push(format!("{:#x}", op2_value.1));
                } else {
                    raw_instruction |= 1 << (op2_value.1 + REG1_FIELD_BIT_POSITION);
                }

                match opcode.as_str() {
                    "add" => {
                        raw_instruction |= 1 << (Opcode::ADD as u8)
                            | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                            | 1 << (op1_index + REG2_FIELD_BIT_POSITION)
                    }
                    "mul" => {
                        raw_instruction |= 1 << (Opcode::MUL as u8)
                            | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                            | 1 << (op1_index + REG2_FIELD_BIT_POSITION)
                    }
                    "and" => {
                        raw_instruction |= 1 << (Opcode::AND as u8)
                            | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                            | 1 << (op1_index + REG2_FIELD_BIT_POSITION)
                    }
                    "or" => {
                        raw_instruction |= 1 << (Opcode::OR as u8)
                            | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                            | 1 << (op1_index + REG2_FIELD_BIT_POSITION)
                    }
                    "xor" => {
                        raw_instruction |= 1 << (Opcode::XOR as u8)
                            | 1 << (dst_index + REG0_FIELD_BIT_POSITION)
                            | 1 << (op1_index + REG2_FIELD_BIT_POSITION)
                    }
                    _ => panic!("not match opcode:{}", opcode),
                }
            }
            "ret" => {
                debug!("opcode: ret");
                assert!(ops.len() == 1, "ret params len is 0");
                raw_instruction |= 1 << Opcode::RET as u8;
            }
            "mstore" => {
                debug!("opcode: mstore");
                assert!(
                    ops.len() == 3,
                    "{}",
                    format!("{} params len is 2", opcode.as_str())
                );
                let op1_value = self.get_index_value(ops[1])?;
                let op2_index = self.get_reg_index(ops[2])? as u64;

                if op1_value.0 as u8 == ImmediateFlag::Used as u8 {
                    raw_instruction |= 1 << IMM_FLAG_FIELD_BIT_POSITION;
                    instuction.push(format!("{:#x}", op1_value.1));
                } else {
                    raw_instruction |= 1 << (op1_value.1 + REG1_FIELD_BIT_POSITION);
                }
                raw_instruction |=
                    1 << Opcode::MSTORE as u8 | 1 << (op2_index + REG2_FIELD_BIT_POSITION);
            }
            "end" => {
                debug!("opcode: end");
                assert!(ops.len() == 1, "end params len is 0");
                raw_instruction |= 1 << Opcode::END as u8;
            }
            _ => panic!("not match opcode:{}", opcode),
        };
        instuction.insert(0, format!("0x{:0>16x}", raw_instruction));
        Ok(instuction)
    }

    pub fn get_inst_len(&self, raw_inst: &str) -> Result<u64, AssemblerError> {
        let ops: Vec<_> = raw_inst.trim().split(' ').collect();
        let opcode = ops.first().unwrap().to_lowercase();

        match opcode.as_str() {
            "mov" | "assert" | "eq" | "neq" | "not" | "gte" | "mload" => {
                debug!("opcode: mov");
                assert!(
                    ops.len() == 3,
                    "{}",
                    format!("{} params len is 2", opcode.as_str())
                );
                let value = self.get_index_value(ops[2])?;
                if value.0 as u8 == ImmediateFlag::Used as u8 {
                    return Ok(IMM_INSTRUCTION_LEN);
                }
            }
            "jmp" | "cjmp" | "call" | "range" => {
                debug!("opcode: cjmp");
                assert!(
                    ops.len() == 2,
                    "{}",
                    format!("{} params len is 1", opcode.as_str())
                );

                let value = self.get_index_value(ops[1])?;
                if value.0 as u8 == ImmediateFlag::Used as u8 {
                    return Ok(IMM_INSTRUCTION_LEN);
                }
            }
            "add" | "mul" | "and" | "or" | "xor" => {
                debug!("opcode: arithmatic");
                assert!(
                    ops.len() == 4,
                    "{}",
                    format!("{} params len is 3", opcode.as_str())
                );
                let op2_value = self.get_index_value(ops[3])?;

                if op2_value.0 as u8 == ImmediateFlag::Used as u8 {
                    return Ok(IMM_INSTRUCTION_LEN);
                }
            }
            "mstore" => {
                debug!("opcode: mstore");
                assert!(
                    ops.len() == 3,
                    "{}",
                    format!("{} params len is 2", opcode.as_str())
                );
                let op1_value = self.get_index_value(ops[1])?;

                if op1_value.0 as u8 == ImmediateFlag::Used as u8 {
                    return Ok(IMM_INSTRUCTION_LEN);
                }
            }
            _ => return Ok(NO_IMM_INSTRUCTION_LEN),
        };
        Ok(NO_IMM_INSTRUCTION_LEN)
    }

    pub fn relocate(&mut self) {
        let init_asm_len = self.asm_code.len();
        let mut cur_asm_len = init_asm_len;
        let mut index = 0;
        loop {
            if index == cur_asm_len {
                break;
            }
            let item = self.asm_code.get(index).unwrap();
            debug!("item:{:?}", item);
            if item.contains(":") {
                let mut label = item.trim().to_string();
                label.remove(label.len() - 1);
                self.labels.insert(label, self.pc);
                self.asm_code.remove(index);
                cur_asm_len -= 1;
                continue;
            } else if  item.contains("//"){
                self.asm_code.remove(index);
                cur_asm_len -= 1;
                continue;
            } else if item.contains("[") {
                // not r5 3
                // add r5 r5 1
                // add r5 r8 r5
                // mstore r5, r4
                //mstore [r8,-3] r4
                let inst = item.trim();
                let ops: Vec<&str> = inst.split(" ").collect();
                let mut offset_asm = Default::default();
                let mut reg_name = Default::default();
                if ops[0].eq("mload") {
                    let mut fp_offset = ops.get(2).unwrap().to_string();
                    fp_offset = fp_offset.replace("[","").replace("]","");
                    let mut base_offset: Vec<&str> = fp_offset.split(",").collect();
                    let mut offset = 0;
                    if (*base_offset.get(0).unwrap()).eq("r8") {
                        offset = i32::from_str_radix(base_offset.get(1).unwrap(), 10).unwrap();
                    }
                    offset_asm = format!("not r6 {}", offset.abs());
                    reg_name = format!("mload {} r6", ops[1].clone().to_string());
                } else if ops[0].eq("mstore") {
                    let mut fp_offset = ops.get(1).unwrap().to_string();
                    fp_offset = fp_offset.replace("[","").replace("]","");
                    let mut base_offset: Vec<&str> = fp_offset.split(",").collect();
                    let mut offset = 0;
                    if (*base_offset.get(0).unwrap()).eq("r8") {
                        offset = i32::from_str_radix(base_offset.get(1).unwrap(), 10).unwrap();
                    }
                    offset_asm = format!("not r6 {}", offset.abs());
                    reg_name = format!("mstore r6 {}", ops[2].clone().to_string());
                } else {
                    panic!("unknown instruction")
                }

                self.asm_code.insert(index+1, offset_asm);
                cur_asm_len += 1;
                self.asm_code.insert(index+2, format!("add r6 r6 1"));
                cur_asm_len += 1;
                self.asm_code.insert(index+3, format!("add r6 r8 r6"));
                cur_asm_len += 1;
                self.asm_code.insert(index+4, reg_name);
                cur_asm_len += 1;
                self.asm_code.remove(index);
                cur_asm_len -= 1;
                continue;
            }
            let len = self.get_inst_len(&item).unwrap();
            self.pc += len;
            index += 1;
        }
    }

    pub fn assemble_link(&mut self, asm_codes: Vec<String>) -> Vec<String> {
        let mut raw_insts = Vec::new();

        self.asm_code = asm_codes;
        self.relocate();
        for item in &self.asm_code {
            println!("{}", item);
        }
        for raw_code in self.asm_code.clone().into_iter() {
            let raw_inst = self.encode_instruction(&raw_code).unwrap();
            raw_insts.extend(raw_inst);
        }
        raw_insts
    }
}

#[allow(unused_imports)]
mod tests {
    use crate::encode::Encoder;
    use log::{debug, error, LevelFilter};
    #[test]
    fn encode_test() {
        let encode_code = "0x4000000840000000
        0x1
        0x4000002040000000
        0x1
        0x4020000001000000
        0x80
        0x4020000001000000
        0x87
        0x4000000840000000
        0x8
        0x4000004040000000
        0x0
        0x0020800100000000
        0x4000000010000000
        0x1e
        0x4000001002000000
        0x80
        0x0040400080000000
        0x4000002002000000
        0x87
        0x0040408400000000
        0x4080000001000000
        0x80
        0x4200000001000000
        0x87
        0x4000008040000000
        0x1
        0x0101004400000000
        0x4000000020000000
        0xc
        0x0000800000400000
        0x0000000000800000";

        let asm_codes = "mov r0 1
         mov r2 1
         mstore 128 r0
         mstore 135 r0
         mov r0 8
         mov r3 0
         EQ r0 r3
         cjmp 30
         mload r1 128
         assert r1 r2
         mload r2 135
         add r4 r1 r2
         mstore 128 r2
         mstore 135 r4
         mov r4 1
         add r3 r3 r4
         jmp 12
         range r3
         end";

        let encoder: Encoder = Default::default();
        let mut raw_insts = Vec::new();
        let raw_codes = asm_codes.split('\n');
        for raw_code in raw_codes.into_iter() {
            let raw_inst = encoder.encode_instruction(raw_code).unwrap();
            raw_insts.extend(raw_inst);
        }

        let encode_codes = encode_code.split('\n');
        for (index, encode_code) in encode_codes.into_iter().enumerate() {
            let raw_inst = raw_insts.get(index).unwrap().clone();
            if raw_inst.eq(encode_code.trim()) {
                debug!("raw_inst: {:?}", raw_inst);
            } else {
                panic!("err raw_inst: {:?}", raw_inst);
            }
        }
    }

    #[test]
    fn call_test() {
        let encode_code = "0x4000000020000000
                             0x7
                            0x4020008200000000
                            0xa
                            0x0200208400000000
                            0x0001000840000000
                            0x0000000004000000
                            0x4000000840000000
                            0x8
                            0x4000001040000000
                            0x2
                            0x4000080040000000
                            0x100010000
                            0x6000040400000000
                            0xfffffffeffffffff
                            0x4000020040000000
                            0x100000000
                            0x0808000001000000
                            0x4000000008000000
                            0x2
                            0x0020200c00000000
                            0x0000000000800000";

        let asm_codes = "JMP 7
    MUL r4 r0 10
    ADD r4 r4 r1
    MOV r0 r4
    RET
    MOV r0 8
    MOV r1 2
    mov r8 0x100010000
    add r7 r8 0xfffffffeffffffff
    mov r6 0x100000000
    mstore r7 r6
    CALL 2
    ADD r0 r0 r1
    END";

        let encoder: Encoder = Default::default();
        let mut raw_insts = Vec::new();
        let raw_codes = asm_codes.split('\n');
        for raw_code in raw_codes.into_iter() {
            let raw_inst = encoder.encode_instruction(raw_code).unwrap();
            raw_insts.extend(raw_inst);
        }

        let encode_codes = encode_code.split('\n');
        for (index, encode_code) in encode_codes.into_iter().enumerate() {
            let raw_inst = raw_insts.get(index).unwrap().clone();
            if raw_inst.eq(encode_code.trim()) {
                debug!("raw_inst: {:?}", raw_inst);
            } else {
                panic!("err raw_inst: {:?}", raw_inst);
            }
        }
    }

    #[test]
    fn label_test() {
        let _ = env_logger::builder()
            .filter_level(LevelFilter::Debug)
            .try_init();
        let encode_code = "0x4000000840000000
        0x1
        0x4000002040000000
        0x1
        0x4020000001000000
        0x80
        0x4020000001000000
        0x87
        0x4000000840000000
        0x8
        0x4000004040000000
        0x0
        0x0020800100000000
        0x4000000010000000
        0x1e
        0x4000001002000000
        0x80
        0x0040400080000000
        0x4000002002000000
        0x87
        0x0040408400000000
        0x4080000001000000
        0x80
        0x4200000001000000
        0x87
        0x4000008040000000
        0x1
        0x0101004400000000
        0x4000000020000000
        0xc
        0x0000800000400000
        0x0000000000800000";

        let asm_codes = "mov r0 1
         mov r2 1
         mstore 128 r0
         mstore 135 r0
         mov r0 8
         mov r3 0
         .LBL_0_0:
         EQ r0 r3
         cjmp .LBL_0_1
         mload r1 128
         assert r1 r2
         mload r2 135
         add r4 r1 r2
         mstore 128 r2
         mstore 135 r4
         mov r4 1
         add r3 r3 r4
         jmp .LBL_0_0
         .LBL_0_1:
         range r3
         end";

        let mut encoder: Encoder = Default::default();
        let asm_codes: Vec<String> = asm_codes.split('\n').map(|e| e.to_string()).collect();

        let raw_insts = encoder.assemble_link(asm_codes);
        let encode_codes = encode_code.split('\n');
        for (index, encode_code) in encode_codes.into_iter().enumerate() {
            let raw_inst = raw_insts.get(index).unwrap().clone();
            if raw_inst.eq(encode_code.trim()) {
                debug!("raw_inst: {:?}", raw_inst);
            } else {
                panic!("err raw_inst: {:?}", raw_inst);
            }
        }
    }

    #[test]
    fn main_test() {
        let _ = env_logger::builder()
            .filter_level(LevelFilter::Debug)
            .try_init();

        let encode_code = "0x6000080400000000
                             0x4
                             0x4000008040000000
                             0x64
                             0x4000010000040000
                             0x3
                             0x4400010400000000
                             0x1
                             0x2002010400000000
                             0x0202000001000000
                             0x4000008040000000
                             0x1
                             0x4000020000040000
                             0x2
                             0x4800020400000000
                             0x1
                             0x2004020400000000
                             0x0204000001000000
                             0x4000008040000000
                             0x2
                             0x4000040000040000
                             0x1
                             0x5000040400000000
                             0x1
                             0x2008040400000000
                             0x0208000001000000
                             0x0004008002000000
                             0x0008001002000000
                             0x0002000802000000
                             0x0200208400000000
                             0x0200108200000000
                             0x0202000001000000
                             0x0002000802000000
                             0x4000008000040000
                             0x4
                             0x4200008400000000
                             0x1
                             0x2001080400000000
                             0x0000000000800000";

        let asm_codes = "main:
                             .LBL_0_0:
                               add r8 r8 4
                               mov r4 100
                               not r5 3
                               add r5 r5 1
                               add r5 r8 r5
                               mstore r5 r4
                               mov r4 1
                               not r6 2
                               add r6 r6 1
                               add r6 r8 r6
                               mstore r6 r4
                               mov r4 2
                               not r7 1
                               add r7 r7 1
                               add r7 r8 r7
                               mstore r7 r4
                               mload r4 r6
                               mload r1 r7
                               mload r0 r5
                               add r4 r4 r1
                               mul r4 r4 r0
                               mstore r5 r4
                               mload r0 r5
                               not r4 4
                               add r4 r4 1
                               add r8 r8 r4
                               end ";

        let mut encoder: Encoder = Default::default();
        let asm_codes: Vec<String> = asm_codes.split('\n').map(|e| e.to_string()).collect();

        let raw_insts = encoder.assemble_link(asm_codes);
        let encode_codes = encode_code.split('\n');
        for (index, encode_code) in encode_codes.into_iter().enumerate() {
            let raw_inst = raw_insts.get(index).unwrap().clone();
            if raw_inst.eq(encode_code.trim()) {
                debug!("raw_inst: {:?}", raw_inst);
            } else {
                panic!("err raw_inst: {:?}", raw_inst);
            }
        }
    }

    #[test]
    fn mload_mstore_expansion() {
        let asm_codes = "main:
                             .LBL_0_0:
                               add r8 r8 4
                               mov r4 100
                               // not r5 3
                               // add r5 r5 1
                               // add r5 r8 r5
                               // mstore r5, r4
                               mstore [r8,-3] r4
                               mov r4 1
                               // not r6 2
                               // add r6 r6 1
                               // add r6 r8 r6
                               // mstore r6, r4
                               mstore [r8,-2] r4
                               mov r4 2
                               // not r7 1
                               // add r7 r7 1
                               // add r7 r8 r7
                               // mstore r7 r4
                               mstore [r8,-1] r4
                               // mload r4 r6
                               // mload r1 r7
                               // mload r0 r5
                               mload r0 [r8,-3]
                               mload r1 [r8,-2]
                               mload r4 [r8,-1]
                               add r4 r4 r1
                               mul r4 r4 r0
                               mstore r5 r4
                               mload r0 r5
                               not r4 4
                               add r4 r4 1
                               add r8 r8 r4
                               end ";

        let mut encoder: Encoder = Default::default();
        let asm_codes: Vec<String> = asm_codes.split('\n').map(|e| e.to_string()).collect();

        let raw_insts = encoder.assemble_link(asm_codes);
        for item in raw_insts {
            println!("{}", item);
        }
    }

    #[test]
    fn fibo_recursive_encode() {
        let asm_codes = "main:
                            .LBL0_0:
                              add r8 r8 4
                              mstore [r8,-2] r8
                              mov r1 10
                              call fib_recursive
                              not r7 4
                              add r7 r7 1
                              add r8 r8 r7
                              end
                            fib_recursive:
                            .LBL1_0:
                              add r8 r8 9
                              mstore [r8,-2] r8
                              mov r0 r1
                              mstore [r8,-7] r0
                              mload r0 [r8,-7]
                              eq r0 1
                              cjmp .LBL1_1
                              jmp .LBL1_2
                            .LBL1_1:
                              mov r0 1
                              not r7 9
                              add r7 r7 1
                              add r8 r8 r7
                              ret
                            .LBL1_2:
                              mload r0 [r8,-7]
                              eq r0 2
                              cjmp .LBL1_3
                              jmp .LBL1_4
                            .LBL1_3:
                              mov r0 1
                              not r7 9
                              add r7 r7 1
                              add r8 r8 r7
                              ret
                            .LBL1_4:
                              not r7 1
                              add r7 r7 1
                              mload r0 [r8,-7]
                              add r1 r0 r7
                              call fib_recursive
                              mstore [r8,-3] r0
                              not r7 2
                              add r7 r7 1
                              mload r0 [r8,-7]
                              add r0 r0 r7
                              mstore [r8,-5] r0
                              mload r1 [r8,-5]
                              call fib_recursive
                              mload r1 [r8,-3]
                              add r0 r1 r0
                              mstore [r8,-6] r0
                              mload r0 [r8,-6]
                              not r7 9
                              add r7 r7 1
                              add r8 r8 r7
                              ret";

        let mut encoder: Encoder = Default::default();
        let asm_codes: Vec<String> = asm_codes.split('\n').map(|e| e.to_string()).collect();

        let raw_insts = encoder.assemble_link(asm_codes);
        for item in raw_insts {
            println!("{}", item);
        }
    }
}
