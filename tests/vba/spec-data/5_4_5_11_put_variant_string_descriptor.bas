' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Variant String: descriptor 08 00 then the ANSI characters
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: a Variant String is written with its length descriptor
' EXPECT: len=4 8 0 65 66
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant
    Dim a As Byte, b As Byte, c As Byte, d As Byte
    v = "AB"
    Open "/t/g.bin" For Binary As #1
    Put #1, , v
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Get #1, , c
    Get #1, , d
    Debug.Print "len=" & CStr(LOF(1)) & " " & CStr(a) & " " & CStr(b) & " " & CStr(c) & " " & CStr(d)
    Close #1
End Sub
