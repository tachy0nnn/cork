use std::collections::HashSet;
use std::fs;
use std::path::Path;

const ELFMAG: [u8; 4] = [0x7f, b'E', b'L', b'F'];
const ELFCLASS64: u8 = 2;
const SHT_DYNSYM: u32 = 11;
const SHN_UNDEF: u16 = 0;

/// extracts all undefined dynamic symbol names from a 64-bit ELF binary
#[must_use]
pub fn get_undefined_symbols(path: &Path) -> HashSet<String> {
    let mut symbols = HashSet::new();
    let Ok(data) = fs::read(path) else {
        return symbols;
    };

    if data.len() < 64 || data[0..4] != ELFMAG || data[4] != ELFCLASS64 {
        return symbols;
    }

    let Ok(shoff_bytes) = data[0x28..0x30].try_into() else {
        return symbols;
    };
    let Ok(shentsize_bytes) = data[0x3a..0x3c].try_into() else {
        return symbols;
    };
    let Ok(shnum_bytes) = data[0x3c..0x3e].try_into() else {
        return symbols;
    };

    let shoff = u64::from_le_bytes(shoff_bytes) as usize;
    let shentsize = u16::from_le_bytes(shentsize_bytes) as usize;
    let shnum = u16::from_le_bytes(shnum_bytes) as usize;

    if shentsize < 64 || shoff.saturating_add(shnum.saturating_mul(shentsize)) > data.len() {
        return symbols;
    }

    // locate the SHT_DYNSYM section header
    let mut dynsym_hdr = None;
    for i in 0..shnum {
        let offset = shoff + i * shentsize;
        let Ok(sh_type_bytes) = data[offset + 4..offset + 8].try_into() else {
            continue;
        };
        let sh_type = u32::from_le_bytes(sh_type_bytes);
        if sh_type == SHT_DYNSYM {
            dynsym_hdr = Some(offset);
            break;
        }
    }

    let Some(sym_hdr_offset) = dynsym_hdr else {
        return symbols;
    };

    let Ok(sym_sh_link_bytes) = data[sym_hdr_offset + 0x28..sym_hdr_offset + 0x2c].try_into()
    else {
        return symbols;
    };
    let Ok(sym_table_offset_bytes) = data[sym_hdr_offset + 0x18..sym_hdr_offset + 0x20].try_into()
    else {
        return symbols;
    };
    let Ok(sym_table_size_bytes) = data[sym_hdr_offset + 0x20..sym_hdr_offset + 0x28].try_into()
    else {
        return symbols;
    };

    let sym_sh_link = u32::from_le_bytes(sym_sh_link_bytes) as usize;
    let sym_table_offset = u64::from_le_bytes(sym_table_offset_bytes) as usize;
    let sym_table_size = u64::from_le_bytes(sym_table_size_bytes) as usize;

    if sym_sh_link >= shnum {
        return symbols;
    }

    let strtab_hdr_offset = shoff + sym_sh_link * shentsize;
    let Ok(strtab_offset_bytes) =
        data[strtab_hdr_offset + 0x18..strtab_hdr_offset + 0x20].try_into()
    else {
        return symbols;
    };
    let Ok(strtab_size_bytes) = data[strtab_hdr_offset + 0x20..strtab_hdr_offset + 0x28].try_into()
    else {
        return symbols;
    };

    let strtab_offset = u64::from_le_bytes(strtab_offset_bytes) as usize;
    let strtab_size = u64::from_le_bytes(strtab_size_bytes) as usize;

    if sym_table_offset.saturating_add(sym_table_size) > data.len()
        || strtab_offset.saturating_add(strtab_size) > data.len()
    {
        return symbols;
    }

    let sym_entries = sym_table_size / 24; // Elf64_Sym is 24 bytes
    for i in 0..sym_entries {
        let entry_offset = sym_table_offset + i * 24;
        let Ok(st_name_bytes) = data[entry_offset..entry_offset + 4].try_into() else {
            continue;
        };
        let Ok(st_shndx_bytes) = data[entry_offset + 6..entry_offset + 8].try_into() else {
            continue;
        };

        let st_name = u32::from_le_bytes(st_name_bytes) as usize;
        let st_shndx = u16::from_le_bytes(st_shndx_bytes);

        if st_shndx == SHN_UNDEF && st_name != 0 && st_name < strtab_size {
            let str_start = strtab_offset + st_name;
            let str_slice = &data[str_start..strtab_offset + strtab_size];
            if let Some(null_pos) = str_slice.iter().position(|&b| b == 0)
                && null_pos > 0
                && let Ok(name) = std::str::from_utf8(&str_slice[..null_pos])
            {
                symbols.insert(name.to_string());
            }
        }
    }

    symbols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_elf() {
        let non_existent = Path::new("/non/existent/file.so");
        assert!(get_undefined_symbols(non_existent).is_empty());
    }

    #[test]
    fn test_real_elf() {
        let test_path = Path::new("/usr/lib/x86_64-linux-gnu/libm.so.6");
        if test_path.exists() {
            let syms = get_undefined_symbols(test_path);
            assert!(
                !syms.is_empty(),
                "libm.so.6 should have undefined dynamic symbols"
            );
        }
    }
}
