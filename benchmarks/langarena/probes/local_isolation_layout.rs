#[no_mangle]
pub extern "C" fn opcode_size() -> usize {
    std::mem::size_of::<kujo::bytecode::OpCode>()
}
#[no_mangle]
pub extern "C" fn value_size() -> usize {
    std::mem::size_of::<kujo::interpreter::Value>()
}
#[no_mangle]
pub extern "C" fn chunk_size() -> usize {
    std::mem::size_of::<kujo::bytecode::BytecodeChunk>()
}
