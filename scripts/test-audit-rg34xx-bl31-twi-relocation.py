#!/usr/bin/env python3
"""Host tests for TOC1 TWI selector derivation and copied-code relocation."""
from __future__ import annotations
import hashlib, importlib.util, struct
from pathlib import Path
SCRIPT=Path(__file__).with_name("audit-rg34xx-bl31-twi-relocation.py")
SPEC=importlib.util.spec_from_file_location("twi_relocation_audit",SCRIPT)
if SPEC is None or SPEC.loader is None: raise RuntimeError("cannot import audit module")
M=importlib.util.module_from_spec(SPEC); SPEC.loader.exec_module(M)

def synthetic_dtb():
    strings=b"pmu0\0device_type\0compatible\0"
    offsets={"pmu0":0,"device_type":5,"compatible":17}
    s=bytearray()
    def token(x): s.extend(struct.pack(">I",x))
    def begin(name):
        token(1); s.extend(name.encode("ascii")+b"\0")
        while len(s)&3: s.append(0)
    def prop(name,value):
        token(3); s.extend(struct.pack(">II",len(value),offsets[name])); s.extend(value)
        while len(s)&3: s.append(0)
    def end(): token(2)
    begin("")
    begin("aliases"); prop("pmu0",b"/soc@03000000/twi@0x07081400/pmu\0"); end()
    begin("soc@03000000"); begin("twi@0x07081400")
    prop("device_type",b"twi5\0")
    begin("pmu"); prop("compatible",b"x-powers,axp2202\0"); end()
    end(); end(); end(); token(9)
    off_struct=56; off_strings=off_struct+len(s); total=off_strings+len(strings)
    header=struct.pack(">10I",0xD00DFEED,total,off_struct,off_strings,40,17,16,0,len(strings),len(s))
    return header+b"\0"*16+bytes(s)+strings

def main():
    dtb=synthetic_dtb(); f=M.derive_selector(dtb)
    assert f["pmu_path"]=="/soc@03000000/twi@0x07081400/pmu"
    assert f["controller_path"]=="/soc@03000000/twi@0x07081400"
    assert f["pmu_compatible"]=="x-powers,axp2202"
    assert f["controller_device_type"]=="twi5" and f["selector"]==5
    assert M.decode_adrp(0xB0000001,0x48013AC8)==(1,0x48014000)
    assert M.decode_add(0x9118A021,0x48013AD4)==(1,1,0x628)
    assert M.bl_target(0x97FFFB7E,0x48002998)==0x48001790
    lookup=M.relocated(0x48013AC8); table=(lookup&~0xFFF)+0x1000+0x628
    assert lookup==0x278A8 and table==0x28628
    assert M.source_equivalent(table)==0x48014848
    assert M.source_equivalent(table+5*8)==0x48014870
    ctx=M.relocated(0x4800C430); slot=(ctx&~0xFFF)+0x8000+0x668
    assert ctx==0x20210 and slot==0x28668
    assert M.source_equivalent(slot)==0x48014888
    # Full integration test against pinned monitor with synthetic FDT; CI uses real TOC1 DTB.
    mon_path=Path("/mnt/data/monitor101/toc1-extracted/items/monitor.bin")
    if mon_path.exists():
        M.DTB_SIZE=len(dtb); M.DTB_SHA256=hashlib.sha256(dtb).hexdigest()
        spl=Path("/mnt/data/baseos-native-boot-contract.c")
        spl.write_text("memcpy(monitor + 0x30, parameters, 128);\nmemcpy(monitor + 0x24, board, 8);\n*(u32 *)(monitor + 0xb0) = 0x4a200000;\n",encoding="utf-8")
        r=M.audit(mon_path.read_bytes(),dtb,spl)
        assert r["selected_controller"]["selector"]==5
        assert r["selected_controller"]["controller_base"]=="0x7081400"
        assert r["relocation"]["source_table_base"]=="0x48014848"
        assert r["relocation"]["selector_source"]=="0x4803cbb4"
        assert r["relocation"]["selector_destination"]=="0x200a4"
    print("PASS: pmu0 -> parent twi5 -> selector 5; relocated lookup selects R-TWI0")
    return 0
if __name__=="__main__": raise SystemExit(main())
