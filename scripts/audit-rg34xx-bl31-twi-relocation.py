#!/usr/bin/env python3
"""Read-only audit of the RG34XX BL31 RTC TWI selector and SRAM relocation."""
from __future__ import annotations
import argparse, hashlib, json, re, struct
from pathlib import Path

MONITOR_BASE=0x48000000
MONITOR_SIZE=107216
MONITOR_SHA256="676bc0206e3c9a7f229d93ea7f3286095f669940ad91f3c4f3d37bdb18696c31"
DTB_SIZE=144384
DTB_SHA256="c28c87380ffab00148bf4dc457e70b8c6c576470dcfbc53df64addddf89603a1"
COPY_SOURCE_START=0x4800C220
COPY_SOURCE_END=0x48015630
SRAM_DEST=0x20000
SRAM_COPY_BYTES=COPY_SOURCE_END-COPY_SOURCE_START
TWI_TABLE_SOURCE=0x48014848
CONTEXT_POINTER_SOURCE=0x48014888
EXTERNAL_CONTEXT=0x4803CB90
STAGED_CONTEXT=0x20080
TWI_BASES={0:0x05002000,1:0x05002400,2:0x05002800,3:0x05002C00,4:0x05003000,5:0x07081400}
TWI_NAMES={0:"TWI0",1:"TWI1",2:"TWI2",3:"TWI3",4:"TWI4",5:"R-TWI0"}

WORDS={
0x48002998:0x97FFFB7E,0x48001794:0xF00001C0,0x480017A4:0xF945E013,0x480017B8:0x97FFFEC5,
0x480012D0:0xB00000A1,0x480012D4:0x910FB821,0x480012EC:0x97FFFE2B,0x480012FC:0x97FFFE6F,
0x48001304:0xB00000A2,0x48001310:0x910FCC42,0x48001318:0xF00001D6,0x4800131C:0x97FFFD96,
0x48001324:0xB00000A1,0x4800132C:0x910FFC21,0x48001330:0x9400297E,0x48001338:0x912E42C1,
0x48001348:0xB98047A0,0x4800134C:0x8B000280,0x48001350:0x385FE000,0x48001354:0x5100C000,
0x48001358:0xB9002420,
0x48001A08:0xF0000041,0x48001A0C:0x900000A2,0x48001A10:0x91088021,0x48001A14:0x9118C042,
0x48001A1C:0xCB010042,0x48001A20:0xD2A00040,0x48001A2C:0x94002415,0x48001A34:0xF00001C1,
0x48001A38:0xD2801000,0x48001A3C:0x912E4021,0x48001A40:0xD2800602,0x48001A44:0xF2A00040,
0x48001A48:0x9400240E,0x4800C430:0x90000054,0x4800C434:0xF9433693,
0x48013AC8:0xB0000001,0x48013AD4:0x9118A021,0x48013AF0:0xF860D821,
}

def u32(b,o):
    if o<0 or o+4>len(b): raise ValueError("u32 offset out of range")
    return struct.unpack_from("<I",b,o)[0]
def u64(b,o):
    if o<0 or o+8>len(b): raise ValueError("u64 offset out of range")
    return struct.unpack_from("<Q",b,o)[0]
def word(b,a): return u32(b,a-MONITOR_BASE)
def qword(b,a): return u64(b,a-MONITOR_BASE)
def sx(v,n):
    sign=1<<(n-1)
    return (v^sign)-sign
def adrp(w,pc):
    if w&0x9F000000!=0x90000000: raise ValueError(f"not ADRP at {pc:#x}")
    imm=sx(((((w>>5)&0x7FFFF)<<2)|((w>>29)&3)),21)
    return w&31,(pc&~0xFFF)+(imm<<12)
def add_imm(w,pc):
    if w&0xFFC00000!=0x91000000: raise ValueError(f"not ADD X immediate at {pc:#x}")
    imm=(w>>10)&0xFFF
    if (w>>22)&1: imm<<=12
    return w&31,(w>>5)&31,imm
def ldr_x(w,pc):
    if w&0xFFC00000!=0xF9400000: raise ValueError(f"not LDR X at {pc:#x}")
    return w&31,(w>>5)&31,((w>>10)&0xFFF)*8
def bl_target(w,pc):
    if w&0xFC000000!=0x94000000: raise ValueError(f"not BL at {pc:#x}")
    return pc+(sx(w&0x03FFFFFF,26)<<2)
def relocated(a):
    if not COPY_SOURCE_START<=a<COPY_SOURCE_END: raise ValueError("source outside copied standby image")
    return SRAM_DEST+a-COPY_SOURCE_START
def source_equivalent(a):
    if not SRAM_DEST<=a<SRAM_DEST+SRAM_COPY_BYTES: raise ValueError("runtime address outside SRAM image")
    return COPY_SOURCE_START+a-SRAM_DEST

def parse_fdt(blob):
    if len(blob)<40: raise ValueError("FDT header truncated")
    magic,total,os,ot,_rsv,_ver,_last,_boot,ss,sz=struct.unpack_from(">10I",blob,0)
    if magic!=0xD00DFEED or total>len(blob) or total<40: raise ValueError("invalid FDT header")
    if os+sz>total or ot+ss>total: raise ValueError("FDT block outside total size")
    strings=blob[ot:ot+ss]
    def str_at(off):
        if not 0<=off<len(strings): raise ValueError("FDT string offset out of range")
        end=strings.find(b"\0",off)
        if end<0: raise ValueError("unterminated FDT property name")
        return strings[off:end].decode("ascii")
    nodes={}; stack=[]; p=os; stop=os+sz; ended=False
    while p+4<=stop:
        token=struct.unpack_from(">I",blob,p)[0]; p+=4
        if token==1:
            end=blob.find(b"\0",p,stop)
            if end<0: raise ValueError("unterminated FDT node")
            stack.append(blob[p:end].decode("ascii")); p=(end+4)&~3
            path="/"+"/".join(stack[1:]) if len(stack)>1 else "/"
            if path in nodes: raise ValueError(f"duplicate FDT node {path}")
            nodes[path]={}
        elif token==2:
            if not stack: raise ValueError("unbalanced FDT node end")
            stack.pop()
        elif token==3:
            if not stack or p+8>stop: raise ValueError("invalid FDT property")
            n,off=struct.unpack_from(">II",blob,p); p+=8; name=str_at(off)
            if p+n>stop: raise ValueError("FDT property out of range")
            value=blob[p:p+n]; p=(p+n+3)&~3
            path="/"+"/".join(stack[1:]) if len(stack)>1 else "/"
            nodes[path][name]=value
        elif token==4: continue
        elif token==9: ended=True; break
        else: raise ValueError(f"unknown FDT token {token} at {p-4:#x}")
    if not ended or stack: raise ValueError("FDT structure did not terminate cleanly")
    return nodes
def fdt_string(nodes,path,name):
    try: raw=nodes[path][name]
    except KeyError as e: raise ValueError(f"missing FDT string {path}:{name}") from e
    if not raw.endswith(b"\0"): raise ValueError("FDT string is not NUL-terminated")
    return raw.split(b"\0",1)[0].decode("ascii")
def derive_selector(dtb):
    nodes=parse_fdt(dtb)
    pmu=fdt_string(nodes,"/aliases","pmu0")
    if not pmu.startswith("/") or pmu.endswith("/"): raise ValueError("invalid pmu0 alias")
    parent=pmu.rsplit("/",1)[0] or "/"
    compat=fdt_string(nodes,pmu,"compatible")
    devtype=fdt_string(nodes,parent,"device_type")
    if compat!="x-powers,axp2202": raise ValueError(f"unexpected PMU compatible {compat}")
    match=re.fullmatch(r"twi([0-9])",devtype)
    if not match: raise ValueError(f"unexpected controller device_type {devtype!r}")
    return {"pmu_path":pmu,"pmu_compatible":compat,"controller_path":parent,
            "controller_device_type":devtype,"selector":int(match.group(1))}

def audit(monitor,dtb,baseos_source):
    msha=hashlib.sha256(monitor).hexdigest(); dsha=hashlib.sha256(dtb).hexdigest()
    if len(monitor)!=MONITOR_SIZE or msha!=MONITOR_SHA256: raise ValueError("monitor size/SHA mismatch")
    if len(dtb)!=DTB_SIZE or dsha!=DTB_SHA256: raise ValueError("original TOC1 DTB size/SHA mismatch")
    for pc,expected in WORDS.items():
        if word(monitor,pc)!=expected: raise ValueError(f"monitor instruction changed at {pc:#x}")
    calls={0x48002998:0x48001790,0x480017B8:0x480012CC,0x480012EC:0x48000B98,
           0x480012FC:0x48000CB8,0x4800131C:0x48000974,0x48001330:0x4800B928}
    for pc,target in calls.items():
        if bl_target(word(monitor,pc),pc)!=target: raise ValueError(f"BL target changed at {pc:#x}")
    for a,s in ((0x480163EE,b"pmu0\0"),(0x480163F3,b"device_type\0"),(0x480163FF,b"twi\0")):
        if monitor[a-MONITOR_BASE:a-MONITOR_BASE+len(s)]!=s: raise ValueError(f"FDT key/prefix changed at {a:#x}")

    sr,sp=adrp(word(monitor,0x48001A08),0x48001A08); sd,sn,si=add_imm(word(monitor,0x48001A10),0x48001A10)
    er,ep=adrp(word(monitor,0x48001A0C),0x48001A0C); ed,en,ei=add_imm(word(monitor,0x48001A14),0x48001A14)
    if (sr,sp,sd,sn,si)!=(1,0x4800C000,1,1,0x220): raise ValueError("standby copy source changed")
    if (er,ep,ed,en,ei)!=(2,0x48015000,2,2,0x630): raise ValueError("standby copy end changed")
    if ep+ei-sp-si!=0x9410: raise ValueError("standby copy length changed")

    # Rebase code ADRP by its source-page delta after the standby image is copied to SRAM.
    rtpc=relocated(0x48013AC8); ar,ap=adrp(word(monitor,0x48013AC8),0x48013AC8)
    rd,rn,imm=add_imm(word(monitor,0x48013AD4),0x48013AD4)
    if (ar,rd,rn,imm)!=(1,1,1,0x628): raise ValueError("TWI lookup ADRP/ADD changed")
    rt_table=(rtpc&~0xFFF)+(ap-(0x48013AC8&~0xFFF))+imm
    src_table=source_equivalent(rt_table)
    if src_table!=TWI_TABLE_SOURCE: raise ValueError(f"relocated table base is {src_table:#x}")
    selector=derive_selector(dtb)
    idx=selector["selector"]
    if idx not in TWI_BASES: raise ValueError("FDT selector has no pinned TWI table entry")
    entries={}
    for i,base in TWI_BASES.items():
        a=TWI_TABLE_SOURCE+i*8; actual=qword(monitor,a)
        if actual!=base: raise ValueError(f"TWI base-map qword changed at {a:#x}")
        entries[str(i)]={"name":TWI_NAMES[i],"source_address":hex(a),
                         "runtime_sram_address":hex(rt_table+i*8),"base":hex(actual)}
    selected_rt=rt_table+idx*8; selected_src=source_equivalent(selected_rt)
    selected_base=qword(monitor,selected_src)

    # Context pointer loaded by the relocated code resolves to the copied SRAM context base.
    ctxpc=relocated(0x4800C430); cr,cp=adrp(word(monitor,0x4800C430),0x4800C430)
    lr,ln,lo=ldr_x(word(monitor,0x4800C434),0x4800C434)
    if (cr,lr,ln,lo)!=(20,19,20,0x668): raise ValueError("relocated context pointer LDR changed")
    rt_ctxslot=(ctxpc&~0xFFF)+(cp-(0x4800C430&~0xFFF))+lo
    src_ctxslot=source_equivalent(rt_ctxslot)
    if src_ctxslot!=CONTEXT_POINTER_SOURCE: raise ValueError("context pointer slot relocation changed")
    ctxbase=qword(monitor,src_ctxslot)
    if ctxbase!=SRAM_DEST: raise ValueError(f"context base changed: {ctxbase:#x}")
    rt_selector=ctxbase+0xA4

    # Selector producer writes external context +0x24; the 0x30-byte stage copy
    # at 0x48001a34 maps that field to the copied structure at 0x20080 + 0x24.
    pr,pp=adrp(word(monitor,0x48001318),0x48001318)
    p_rd,p_rn,p_imm=add_imm(word(monitor,0x48001338),0x48001338)
    if (pr,pp,p_rd,p_rn,p_imm)!=(22,0x4803C000,1,22,0xB90): raise ValueError("selector producer context address changed")
    st_r,st_p=adrp(word(monitor,0x48001A34),0x48001A34)
    st_rd,st_rn,st_imm=add_imm(word(monitor,0x48001A3C),0x48001A3C)
    if (st_r,st_p,st_rd,st_rn,st_imm)!=(1,0x4803C000,1,1,0xB90): raise ValueError("context staging source changed")
    if (word(monitor,0x48001A38),word(monitor,0x48001A40),word(monitor,0x48001A44))!=(0xD2801000,0xD2800602,0xF2A00040):
        raise ValueError("context staging length/destination changed")
    ext_selector=EXTERNAL_CONTEXT+0x24; staged_selector=STAGED_CONTEXT+0x24
    if ext_selector!=EXTERNAL_CONTEXT+0x24 or staged_selector!=rt_selector:
        raise ValueError("external selector does not map to SRAM context +0xa4")

    source=baseos_source.read_text(encoding="utf-8")
    patches=("memcpy(monitor + 0x30, parameters, 128);",
             "memcpy(monitor + 0x24, board, 8);",
             "*(u32 *)(monitor + 0xb0) = 0x4a200000;")
    if any(x not in source for x in patches): raise ValueError("pinned BaseOS monitor patch contract changed")

    return {
      "schema":"rg34xx-bl31-rtc-twi-selector-relocation-v1","monitor_sha256":msha,"toc1_dtb_sha256":dsha,
      "call_chain":{"0x48002998":"0x48001790","0x480017b8":"0x480012cc"},
      "fdt_selector":{**selector,"producer_store_pc":"0x48001358","external_context_base":hex(EXTERNAL_CONTEXT),
                      "external_selector_address":hex(ext_selector)},
      "relocation":{"source_start":hex(COPY_SOURCE_START),"source_end":hex(COPY_SOURCE_END),
                    "bytes":hex(SRAM_COPY_BYTES),"runtime_lookup_pc":hex(rtpc),
                    "runtime_table_base":hex(rt_table),"source_table_base":hex(src_table),
                    "runtime_context_pointer_slot":hex(rt_ctxslot),"source_context_pointer_slot":hex(src_ctxslot),
                    "context_base":hex(ctxbase),"runtime_context_plus_a4":hex(rt_selector),
                    "staged_context_source":hex(EXTERNAL_CONTEXT),"staged_context_destination":hex(STAGED_CONTEXT),
                    "staged_context_bytes":"0x30","selector_source":hex(ext_selector),
                    "selector_destination":hex(staged_selector)},
      "twi_table":entries,
      "selected_controller":{"selector":idx,"name":TWI_NAMES[idx],"runtime_entry":hex(selected_rt),
                             "source_entry":hex(selected_src),"controller_base":hex(selected_base)},
      "baseos_monitor_patch_contract":list(patches),
      "limitations":[
        "Exact matched-BL31/original-TOC1-DTB host/static data flow only; no device suspend trace or live memory readback.",
        "Selector 5 is derived from the original TOC1 FDT and matched producer/staging path; the active runtime transaction was not captured.",
        "No PMIC slave address, command/register sequence, GP_DATA2 consumer, or final power action is established.",
        "No SMC/MMIO/TWI transaction, DRAM content access, suspend/wake, storage write, flashing, or payload execution was performed."
      ]
    }

def render(r):
    f=r["fdt_selector"]; x=r["relocation"]; c=r["selected_controller"]
    lines=[
      "# RG34XX BL31 RTC TWI selector provenance and SRAM relocation","",
      f"Status: **static selector dataflow resolves index {c['selector']} to {c['name']} at {c['controller_base']}; no PMIC transaction is claimed.**","",
      f"- Monitor SHA-256: {r['monitor_sha256']}",
      f"- Original TOC1 DTB SHA-256: {r['toc1_dtb_sha256']}",
      "- PSCI suspend call chain: 0x48002998 -> 0x48001790 -> 0x480012cc.","",
      "## FDT-derived selector","",
      f"- /aliases/pmu0: {f['pmu_path']} ({f['pmu_compatible']}).",
      f"- Parent controller: {f['controller_path']}, device_type={f['controller_device_type']}.",
      f"- BL31 converts the final digit of the twi-prefixed device_type to selector {c['selector']} and stores it at {f['external_selector_address']}.","",
      "## Copy/relocation","",
      f"- Standby image: {x['source_start']}..{x['source_end']} ({x['bytes']} bytes) -> SRAM 0x20000.",
      f"- Relocated lookup PC {x['runtime_lookup_pc']} resolves runtime table {x['runtime_table_base']}, source-equivalent {x['source_table_base']}.",
      f"- Relocated context pointer slot {x['runtime_context_pointer_slot']} maps to source qword {x['source_context_pointer_slot']}, containing context base {x['context_base']}.",
      f"- External selector {x['selector_source']} is staged to SRAM context+0xa4 at {x['selector_destination']}.","",
      "## Selected controller","",
      f"- Selector: {c['selector']}",
      f"- Runtime table entry: {c['runtime_entry']} (source-equivalent {c['source_entry']})",
      f"- Controller base: {c['controller_base']} ({c['name']})",
      "- This corrects the earlier unrelocated interpretation of source addresses 0x48014628/0x48014668; the active source-equivalent locations are 0x48014848/0x48014888.","",
      "## Limitations",""
    ]
    lines.extend("- "+x for x in r["limitations"])
    return "\n".join(lines)+"\n"

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument("--monitor",type=Path,required=True)
    p.add_argument("--dtb",type=Path,required=True)
    p.add_argument("--baseos-source",type=Path,required=True)
    p.add_argument("--out-dir",type=Path,required=True)
    a=p.parse_args()
    r=audit(a.monitor.read_bytes(),a.dtb.read_bytes(),a.baseos_source)
    a.out_dir.mkdir(parents=True,exist_ok=True)
    (a.out_dir/"twi-selector-relocation-report.json").write_text(json.dumps(r,indent=2,sort_keys=True)+"\n",encoding="utf-8")
    md=render(r); (a.out_dir/"TWI-SELECTOR-RELOCATION.md").write_text(md,encoding="utf-8")
    print(md); return 0
if __name__=="__main__": raise SystemExit(main())
