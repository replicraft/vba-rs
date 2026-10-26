' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 a Variant is preceded by a two byte type descriptor (Integer 02 00)
' EXPECT: len=4 2 0 5 0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant, a As Byte, b As Byte, c As Byte, d As Byte
    v = CInt(5)
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
