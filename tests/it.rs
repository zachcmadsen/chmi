#[test]
fn parse_un880_capabilities() {
    let capabilities_string = "(prot(monitor)type(lcd)UN880cmds(01 02 03 0C E3 F3)vcp(02 04 05 08 10 12 14(05 08 0B ) 16 18 1A 52 60( 11 12 0F 00) AC AE B2 B6 C0 C6 C8 C9 D6(01 04) DF 62 8D F4 F5(00 01 02) F6(00 01 02) 4D 4E 4F 15(01 06 11 13 14 15 18 19 28 29 48) F7(00 01 02 03) F8(00 01) F9 E4 E5 E6 E7 E8 E9 EA EB EF FD(00 01) FE(00 01 02) FF)mccs_ver(2.1)mswhql(1))";
    let capabilities =
        chmi::parse_capabilities_string(&capabilities_string).unwrap();
    insta::assert_debug_snapshot!(capabilities);
}

#[test]
fn parse_u32j59x_capabilities() {
    let capabilities_string = "(prot(monitor)type(lcd)SAMSUNGcmds(01 02 03 07 0C E3 F3)vcp(02 04 05 08 10 12 14(05 08 0B 0C) 16 18 1A 52 60( 11 12 0F) AC AE B2 B6 C6 C8 C9 D6(01 04 05) DC(00 02 03 05 ) DF FD)mccs_ver(2.1)mswhql(1))";
    let capabilities =
        chmi::parse_capabilities_string(&capabilities_string).unwrap();
    insta::assert_debug_snapshot!(capabilities);
}

#[test]
fn parse_vg259_capabilities() {
    let capabilities_string = "(prot(monitor) type(LCD)model(VG259) cmds(01 02 03 07 0C F3) vcp(02 04 05 08 10 12 14(05 06 08 0B) 16 18 1A 52 60(11 12 0F) 62 6C 6E 70 86(02 0B) 87(00 0A 14 1E 28 32 3C 46 50 5A 64) 8A 8D(01 02) AC AE B6 C6 C8 C9 CC(01 02 03 04 05 06 07 08 09 0A 0C 0D 11 12 14 1A 1E 1F 23 30 31) D6(01 05) DC(01 02 03 04 05 06 07 08) DF E0(00 01 02 03 04 05) E1(00 01) E3(00 01 02 03 04 05 06) E4(00 01 02 03 04 05) E5(00 01 02 03) E6(00 01 02 03 04) E7(00 01) E9(00 01) EA(00 01) EB(00 01))mccs_ver(2.2)asset_eep(32)mpu(01)mswhql(1))";
    let capabilities =
        chmi::parse_capabilities_string(&capabilities_string).unwrap();
    insta::assert_debug_snapshot!(capabilities);
}
