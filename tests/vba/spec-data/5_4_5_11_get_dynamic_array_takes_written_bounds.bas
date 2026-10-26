' INV: stmt.put, stmt.get
' SOURCE: derived the Learn Put and Get statement pages: Get into a dynamic array reads the descriptor and then the data, so the target takes the written bounds
' EXPECT: 5 7 10 20 30
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim a() As Long, b() As Long
    ReDim a(5 To 7)
    a(5) = 10: a(6) = 20: a(7) = 30
    Open "/t/g.bin" For Binary As #1
    Put #1, , a
    ReDim b(1 To 1)
    Seek #1, 1
    Get #1, , b
    Close #1
    Debug.Print CStr(LBound(b)) & " " & CStr(UBound(b)) & " " & CStr(b(5)) & " " & CStr(b(6)) & " " & CStr(b(7))
End Sub
