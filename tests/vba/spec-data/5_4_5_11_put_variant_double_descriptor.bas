' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Variant Double: descriptor 05 00 then eight bytes
' EXPECT: len=10 5 0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant, a As Byte, b As Byte
    v = CDbl(1.5)
    Open "/t/g.bin" For Binary As #1
    Put #1, , v
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Debug.Print "len=" & CStr(LOF(1)) & " " & CStr(a) & " " & CStr(b)
    Close #1
End Sub
