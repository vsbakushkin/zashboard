#[derive(Debug, PartialEq, Eq)]
pub enum LuaInit<'a> {
    File(&'a [u8]),
    Code(&'a [u8]),
}

#[derive(Debug, PartialEq, Eq)]
pub struct NfqwsConfig<'a> {
    pub queue_number: Option<u16>,
    pub fwmark: Option<u32>,
    pub lua_inits: Vec<LuaInit<'a>>,
}
