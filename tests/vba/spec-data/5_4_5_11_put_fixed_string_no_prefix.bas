' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 fixed-length String has no prefix
' EXPECT: len=3 65 66 67
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String * 3, a As Byte, b As Byte, c As Byte
    s = "ABC"
    Open "/t/g.bin" For Binary As #1
    Put #1, , s
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Get #1, , c
    Debug.Print "len=" & CStr(LOF(1)) & " " & CStr(a) & " " & CStr(b) & " " & CStr(c)
    Close #1
End Sub
