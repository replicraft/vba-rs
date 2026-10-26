' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 in binary mode a String has no length prefix and is stored in ANSI form
' EXPECT: len=2 65 66
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String, a As Byte, b As Byte
    s = "AB"
    Open "/t/g.bin" For Binary As #1
    Put #1, , s
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Debug.Print "len=" & CStr(LOF(1)) & " " & CStr(a) & " " & CStr(b)
    Close #1
End Sub
