' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Variant Empty is 00 00 and Null is 01 00
' EXPECT: len=4 0 0 1 0
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim e As Variant, n As Variant
    Dim a As Byte, b As Byte, c As Byte, d As Byte
    n = Null
    Open "/t/g.bin" For Binary As #1
    Put #1, , e
    Put #1, , n
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Get #1, , c
    Get #1, , d
    Debug.Print "len=" & CStr(LOF(1)) & " " & CStr(a) & " " & CStr(b) & " " & CStr(c) & " " & CStr(d)
    Close #1
End Sub
