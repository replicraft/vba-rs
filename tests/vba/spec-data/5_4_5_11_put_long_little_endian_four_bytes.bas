' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 Long is written as four bytes
' EXPECT: len=4 4 3 2 1
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim l As Long, a As Byte, b As Byte, c As Byte, d As Byte
    l = &H1020304
    Open "/t/g.bin" For Binary As #1
    Put #1, , l
    Seek #1, 1
    Get #1, , a
    Get #1, , b
    Get #1, , c
    Get #1, , d
    Debug.Print "len=" & CStr(LOF(1)) & " " & CStr(a) & " " & CStr(b) & " " & CStr(c) & " " & CStr(d)
    Close #1
End Sub
