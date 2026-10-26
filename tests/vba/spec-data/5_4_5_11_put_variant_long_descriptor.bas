' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Variant Long: descriptor 03 00 then four bytes
' EXPECT: len=6 3 0 7
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim v As Variant, a As Byte, b As Byte, c As Byte
    v = CLng(7)
    Open "/t/g.bin" For Binary As #1
    Put #1, , v
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Get #1, , c
    Debug.Print "len=" & CStr(LOF(1)) & " " & CStr(a) & " " & CStr(b) & " " & CStr(c)
    Close #1
End Sub
