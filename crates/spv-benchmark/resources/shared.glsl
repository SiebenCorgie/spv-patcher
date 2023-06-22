
struct ResHandle{
    uint hdl;
};

uint get_index(ResHandle res){
    return (res.hdl >> 8);
}

uint get_type(ResHandle res){
    return res.hdl & 0xff;
}

bool handle_invalid(ResHandle res){
  //checks that the handle is somewhere in the *undefined* area
  return get_type(res) > 0x4;
}
